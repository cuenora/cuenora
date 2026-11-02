//! Monthly realized flow history through the public HTTP boundary.
mod common;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn send(
    router: Router,
    token: Option<&str>,
    uri: &str,
    body: &str,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::get(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(response.into_body(), 256 * 1024).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if body.get("code").is_some() {
        assert_eq!(
            body["request_id"],
            headers["x-request-id"].to_str().unwrap()
        );
        assert_eq!(body["details"], Value::Null);
    }
    (status, headers, body)
}

async fn seed(pool: &PgPool, verified: bool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let user: String = sqlx::query_scalar("INSERT INTO users (email_original,email_address,email_verified_at) VALUES ($1,$1,CASE WHEN $2 THEN statement_timestamp() END) RETURNING id::text")
        .bind(email).bind(verified).fetch_one(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions (user_id) VALUES ($1::uuid) RETURNING id::text",
    )
    .bind(&user)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens (session_id,user_id,token_hash,expires_at) VALUES ($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')")
        .bind(session).bind(&user).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    (user, tokens.access.expose_secret().to_owned())
}

const URI: &str = "/v1/reports/monthly-flows?date_from=2023-11-15&date_to=2024-03-10";

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn monthly_history_zero_fills_clips_edges_and_reconciles_exact_currency_flows() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (foreign, _) = seed(&pool, true).await;
    let account: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Monthly','normal','SEK') RETURNING id::text")
        .bind(&owner).fetch_one(&pool).await.unwrap();
    for (user, kind, currency, amount, date, state, linked) in [
        (
            owner.as_str(),
            "income",
            "SEK",
            100,
            "2023-11-14",
            "posted",
            None,
        ),
        (&owner, "income", "SEK", 123, "2023-11-15", "posted", None),
        (&owner, "expense", "SEK", 23, "2023-11-30", "posted", None),
        (
            &owner,
            "income",
            "SEK",
            i64::MAX,
            "2024-01-01",
            "posted",
            None,
        ),
        (
            &owner,
            "income",
            "SEK",
            i64::MAX,
            "2024-01-31",
            "posted",
            None,
        ),
        (
            &owner,
            "interest_income",
            "SEK",
            4,
            "2024-01-15",
            "posted",
            Some(account.as_str()),
        ),
        (&owner, "expense", "EUR", 799, "2024-02-29", "posted", None),
        (&owner, "expense", "USD", 25, "2024-03-10", "posted", None),
        (&owner, "expense", "USD", 100, "2024-03-11", "posted", None),
        (&owner, "expense", "SEK", 999, "2023-12-15", "voided", None),
        (
            &owner,
            "opening_balance",
            "SEK",
            999,
            "2023-12-15",
            "posted",
            Some(account.as_str()),
        ),
        (&foreign, "income", "USD", 999, "2023-12-15", "posted", None),
    ] {
        sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on,state,account_id) VALUES($1::uuid,$2,$3,$4,$5::date,$6,$7::uuid)")
            .bind(user).bind(kind).bind(currency).bind(amount).bind(date).bind(state).bind(linked).execute(&pool).await.unwrap();
    }
    let destination: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Destination','normal','SEK') RETURNING id::text")
        .bind(&owner).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO transactions(user_id,kind,account_id,destination_account_id,currency,amount_minor,received_currency,received_amount_minor,occurred_on) VALUES($1::uuid,'transfer',$2::uuid,$3::uuid,'SEK',500,'SEK',500,'2023-12-15')")
        .bind(&owner).bind(&account).bind(destination).execute(&pool).await.unwrap();
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    let credentials = common::snapshot(&pool).await;
    let before = all_tables_snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let (status, _, report) = send(router.clone(), Some(&token), URI, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["date_from"], "2023-11-15");
    assert_eq!(report["date_to"], "2024-03-10");
    assert_eq!(report["currencies"].as_array().unwrap().len(), 3);
    for (index, currency) in ["SEK", "EUR", "USD"].iter().enumerate() {
        let rows = &report["currencies"][index];
        assert_eq!(rows["currency"], *currency);
        let months = rows["months"].as_array().unwrap();
        assert_eq!(months.len(), 5);
        for (i, (month, from, to)) in [
            ("2023-11", "2023-11-15", "2023-11-30"),
            ("2023-12", "2023-12-01", "2023-12-31"),
            ("2024-01", "2024-01-01", "2024-01-31"),
            ("2024-02", "2024-02-01", "2024-02-29"),
            ("2024-03", "2024-03-01", "2024-03-10"),
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(months[i]["month"], *month);
            assert_eq!(months[i]["date_from"], *from);
            assert_eq!(months[i]["date_to"], *to);
            let (status, _, flows) = send(
                router.clone(),
                Some(&token),
                &format!("/v1/reports/flows?date_from={from}&date_to={to}"),
                "",
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            for field in ["income", "expenses", "net_flow"] {
                assert_eq!(months[i][field], flows["currencies"][index][field]);
            }
        }
    }
    assert_eq!(report["currencies"][0]["months"][0]["net_flow"], "1.00");
    for field in ["income", "expenses", "net_flow"] {
        assert_eq!(report["currencies"][0]["months"][1][field], "0.00");
    }
    assert_eq!(
        report["currencies"][0]["months"][2]["income"],
        "184467440737095516.18"
    );
    assert_eq!(report["currencies"][1]["months"][3]["net_flow"], "-7.99");
    assert_eq!(report["currencies"][2]["months"][4]["expenses"], "0.25");
    assert_eq!(send(router, Some(&token), URI, "").await.2, report);
    assert_eq!(before, all_tables_snapshot(&pool).await);
    assert_eq!(credentials, common::snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn empty_history_supports_same_day_leap_and_year_extremes_and_exact_120_month_limit() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool));
    for (from, to, count) in [
        ("0001-01-01", "0001-01-01", 1),
        ("9999-12-31", "9999-12-31", 1),
        ("2024-02-29", "2024-02-29", 1),
        ("2015-01-31", "2024-12-01", 120),
    ] {
        let uri = format!("/v1/reports/monthly-flows?date_from={from}&date_to={to}");
        let (status, _, body) = send(router.clone(), Some(&token), &uri, "").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        for currency in body["currencies"].as_array().unwrap() {
            let months = currency["months"].as_array().unwrap();
            assert_eq!(months.len(), count);
            assert_eq!(months[0]["date_from"], from);
            assert_eq!(months[count - 1]["date_to"], to);
            for month in months {
                for field in ["income", "expenses", "net_flow"] {
                    assert_eq!(month[field], "0.00");
                }
            }
        }
    }
    let (status, _, body) = send(
        router,
        Some(&token),
        "/v1/reports/monthly-flows?date_from=2015-01-31&date_to=2025-01-01",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_request");
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn rejects_invalid_filters_and_requires_verified_bearer_identity() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let (_, pending) = seed(&pool, false).await;
    let router = app(Some(pool));
    assert_eq!(
        send(router.clone(), None, URI, "").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(router.clone(), Some("invalid"), URI, "").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(router.clone(), Some(&pending), URI, "").await.0,
        StatusCode::FORBIDDEN
    );
    for uri in [
        "/v1/reports/monthly-flows",
        "/v1/reports/monthly-flows?date_from=2024-02-01",
        "/v1/reports/monthly-flows?date_from=2024-02-30&date_to=2024-03-01",
        "/v1/reports/monthly-flows?date_from=2024-03-01&date_to=2024-02-01",
        "/v1/reports/monthly-flows?date_from=2024-2-01&date_to=2024-03-01",
        "/v1/reports/monthly-flows?date_from=0000-01-01&date_to=2024-03-01",
        &format!("{URI}&currency=SEK"),
        &format!("{URI}&date_from=2024-02-01"),
        &format!("{URI}&x={}", "a".repeat(1024)),
    ] {
        let (status, _, error) = send(router.clone(), Some(&token), uri, "").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {error}");
        assert_eq!(error["code"], "invalid_request");
    }
    assert_eq!(
        send(router, Some(&token), URI, "{}").await.0,
        StatusCode::BAD_REQUEST
    );
}

async fn all_tables_snapshot(pool: &PgPool) -> Vec<(String, String)> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename::text FROM pg_tables WHERE schemaname='public' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut result = Vec::new();
    for table in tables {
        let query = format!(
            "SELECT md5(coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]')::text) FROM \"{table}\" t"
        );
        result.push((
            table,
            sqlx::query_scalar(&query).fetch_one(pool).await.unwrap(),
        ));
    }
    result
}

async fn mutate(router: Router, token: &str, method: &str, path: &str, key: &str, body: &str) {
    let response = router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", key)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 65536).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn corrections_planning_timezones_and_safe_failures_preserve_read_only_ownership() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (foreign, foreign_token) = seed(&pool, true).await;
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let id: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on) VALUES($1::uuid,'expense','SEK',1250,'2023-12-31') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on) VALUES($1::uuid,'income','SEK',9900,'2024-01-01')").bind(&foreign).execute(&pool).await.unwrap();
    let rule: String = sqlx::query_scalar("INSERT INTO recurring_rules(user_id,amount_minor,currency,cadence,starts_on,time_zone) VALUES($1::uuid,50000,'SEK','monthly','2090-01-01','Etc/UTC') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO recurring_occurrences(rule_id,user_id,scheduled_on,due_on,amount_minor,currency,state) VALUES($1::uuid,$2::uuid,'2090-01-01','2090-01-01',50000,'SEK','planned')").bind(&rule).bind(&owner).execute(&pool).await.unwrap();
    let router = app(Some(pool.clone()));
    let uri = "/v1/reports/monthly-flows?date_from=2023-12-31&date_to=2024-01-01";
    let before = all_tables_snapshot(&pool).await;
    let initial = send(router.clone(), Some(&token), uri, "").await;
    assert_eq!(initial.0, StatusCode::OK);
    assert_eq!(initial.2["currencies"][0]["months"][0]["expenses"], "12.50");
    assert_eq!(initial.2["currencies"][0]["months"][1]["income"], "0.00");
    let other = send(router.clone(), Some(&foreign_token), uri, "").await;
    assert_eq!(other.0, StatusCode::OK);
    assert_eq!(other.2["currencies"][0]["months"][0]["expenses"], "0.00");
    assert_eq!(other.2["currencies"][0]["months"][1]["income"], "99.00");
    for query in [
        format!("{uri}&user_id={foreign}"),
        format!("{uri}&date_to=2024-01-01%27%20OR%201%3D1--"),
    ] {
        assert_eq!(
            send(router.clone(), Some(&token), &query, "").await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let future = send(
        router.clone(),
        Some(&token),
        "/v1/reports/monthly-flows?date_from=2090-01-01&date_to=2090-03-31",
        "",
    )
    .await;
    assert_eq!(future.0, StatusCode::OK);
    for c in future.2["currencies"].as_array().unwrap() {
        for m in c["months"].as_array().unwrap() {
            for f in ["income", "expenses", "net_flow"] {
                assert_eq!(m[f], "0.00");
            }
        }
    }
    assert_eq!(before, all_tables_snapshot(&pool).await);
    mutate(
        router.clone(),
        &token,
        "PATCH",
        &format!("/v1/transactions/{id}"),
        "independent-move",
        r#"{"amount":"34.56","occurred_on":"2024-01-01"}"#,
    )
    .await;
    let after_edit = all_tables_snapshot(&pool).await;
    let corrected = send(router.clone(), Some(&token), uri, "").await;
    assert_eq!(corrected.0, StatusCode::OK);
    assert_eq!(
        corrected.2["currencies"][0]["months"][0]["expenses"],
        "0.00"
    );
    assert_eq!(
        corrected.2["currencies"][0]["months"][1]["expenses"],
        "34.56"
    );
    for zone in ["Pacific/Kiritimati", "America/Los_Angeles"] {
        let zoned = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .after_connect(move |conn, _| {
                Box::pin(async move {
                    sqlx::query("SELECT set_config('TimeZone',$1,false)")
                        .bind(zone)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_with((*pool.connect_options()).clone())
            .await
            .unwrap();
        let actual: String = sqlx::query_scalar("SHOW TimeZone")
            .fetch_one(&zoned)
            .await
            .unwrap();
        assert_eq!(actual, zone);
        let reply = send(app(Some(zoned.clone())), Some(&token), uri, "").await;
        assert_eq!(reply.0, StatusCode::OK);
        assert_eq!(reply.2, corrected.2);
        zoned.close().await;
    }
    assert_eq!(after_edit, all_tables_snapshot(&pool).await);
    mutate(
        router.clone(),
        &token,
        "POST",
        &format!("/v1/transactions/{id}/void"),
        "independent-void",
        "{}",
    )
    .await;
    let after_void = all_tables_snapshot(&pool).await;
    let voided = send(router.clone(), Some(&token), uri, "").await;
    assert_eq!(voided.0, StatusCode::OK);
    for m in voided.2["currencies"][0]["months"].as_array().unwrap() {
        assert_eq!(m["expenses"], "0.00");
    }
    assert_eq!(after_void, all_tables_snapshot(&pool).await);
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE transactions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let failed = send(router, Some(&token), uri, "").await;
    lock.rollback().await.unwrap();
    assert_eq!(failed.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(failed.2["code"], "monthly_flows_unavailable");
    assert!(failed.2.get("currencies").is_none());
    assert!(!failed.2.to_string().contains("transactions"));
    assert_eq!(after_void, all_tables_snapshot(&pool).await);
}
