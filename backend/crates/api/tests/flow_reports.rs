//! Realized flow totals through the public HTTP boundary.
mod common;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::{Value, json};
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

const URI: &str = "/v1/reports/flows?date_from=2024-02-01&date_to=2024-02-29";

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn exact_totals_include_boundaries_accountless_and_interest_but_not_other_events() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (foreign, _) = seed(&pool, true).await;
    let account: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Report','normal','SEK') RETURNING id::text")
        .bind(&owner).fetch_one(&pool).await.unwrap();
    for (user, kind, currency, amount, date, state, linked) in [
        (
            owner.as_str(),
            "income",
            "SEK",
            i64::MAX,
            "2024-02-01",
            "posted",
            None,
        ),
        (
            &owner,
            "income",
            "SEK",
            i64::MAX,
            "2024-02-29",
            "posted",
            None,
        ),
        (
            &owner,
            "interest_income",
            "SEK",
            4,
            "2024-02-15",
            "posted",
            Some(account.as_str()),
        ),
        (&owner, "expense", "SEK", 305, "2024-02-15", "posted", None),
        (&owner, "expense", "EUR", 799, "2024-02-15", "posted", None),
        (&owner, "income", "SEK", 999, "2024-01-31", "posted", None),
        (&owner, "expense", "SEK", 999, "2024-03-01", "posted", None),
        (&owner, "expense", "SEK", 999, "2024-02-15", "voided", None),
        (
            &owner,
            "opening_balance",
            "SEK",
            999,
            "2024-02-15",
            "posted",
            Some(account.as_str()),
        ),
        (&foreign, "income", "USD", 999, "2024-02-15", "posted", None),
    ] {
        sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on,state,account_id) VALUES($1::uuid,$2,$3,$4,$5::date,$6,$7::uuid)")
            .bind(user).bind(kind).bind(currency).bind(amount).bind(date).bind(state).bind(linked).execute(&pool).await.unwrap();
    }
    let destination: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Other','normal','SEK') RETURNING id::text")
        .bind(&owner).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO transactions(user_id,kind,account_id,destination_account_id,currency,amount_minor,received_currency,received_amount_minor,occurred_on) VALUES($1::uuid,'transfer',$2::uuid,$3::uuid,'SEK',500,'SEK',500,'2024-02-15')")
        .bind(&owner).bind(&account).bind(destination).execute(&pool).await.unwrap();
    // Archiving does not erase prior realized income.
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&account)
        .execute(&pool)
        .await
        .unwrap();
    let before: String = sqlx::query_scalar(
        "SELECT md5(jsonb_agg(to_jsonb(t) ORDER BY id)::text) FROM transactions t",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let credentials_before = common::snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    for _ in 0..2 {
        let (status, _, result) = send(router.clone(), Some(&token), URI, "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            result,
            json!({"date_from":"2024-02-01","date_to":"2024-02-29","currencies":[
                {"currency":"SEK","income":"184467440737095516.18","expenses":"3.05","net_flow":"184467440737095513.13"},
                {"currency":"EUR","income":"0.00","expenses":"7.99","net_flow":"-7.99"},
                {"currency":"USD","income":"0.00","expenses":"0.00","net_flow":"0.00"}
            ]})
        );
    }
    let after: String = sqlx::query_scalar(
        "SELECT md5(jsonb_agg(to_jsonb(t) ORDER BY id)::text) FROM transactions t",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after);
    assert_eq!(credentials_before, common::snapshot(&pool).await);
    let (status, _, empty) = send(
        router,
        Some(&token),
        "/v1/reports/flows?date_from=0001-01-01&date_to=0001-01-01",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for row in empty["currencies"].as_array().unwrap() {
        assert_eq!(row["income"], "0.00");
        assert_eq!(row["expenses"], "0.00");
        assert_eq!(row["net_flow"], "0.00");
    }
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
        "/v1/reports/flows",
        "/v1/reports/flows?date_from=2024-02-01",
        "/v1/reports/flows?date_from=2024-02-30&date_to=2024-03-01",
        "/v1/reports/flows?date_from=2024-03-01&date_to=2024-02-01",
        "/v1/reports/flows?date_from=2024-2-01&date_to=2024-03-01",
        "/v1/reports/flows?date_from=0000-01-01&date_to=2024-03-01",
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

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn corrected_events_one_day_auth_and_all_table_read_only() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (foreign, foreign_token) = seed(&pool, true).await;
    let (_, pending) = seed(&pool, false).await;
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let id: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on) VALUES($1::uuid,'expense','USD',100,'2024-02-28') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on) VALUES($1::uuid,'income','USD',99999,'2024-02-29')").bind(&foreign).execute(&pool).await.unwrap();
    // A future ungenerated rule exists: reporting must not generate or count it.
    sqlx::query("INSERT INTO recurring_rules(user_id,amount_minor,currency,cadence,starts_on,time_zone) VALUES($1::uuid,50000,'USD','monthly','2099-01-01','Etc/UTC')").bind(&owner).execute(&pool).await.unwrap();
    let router = app(Some(pool.clone()));
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/v1/transactions/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("idempotency-key", "independent-edit")
                .body(Body::from(
                    r#"{"amount":"2.75","occurred_on":"2024-02-29"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let before = all_tables_snapshot(&pool).await;
    let one_day = "/v1/reports/flows?date_from=2024-02-29&date_to=2024-02-29";
    let (status, _, report) = send(router.clone(), Some(&token), one_day, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        report["currencies"][2],
        json!({"currency":"USD","income":"0.00","expenses":"2.75","net_flow":"-2.75"})
    );
    for period in [
        "date_from=2024-02-28&date_to=2024-02-28",
        "date_from=2099-01-01&date_to=2099-12-31",
    ] {
        let (status, _, report) = send(
            router.clone(),
            Some(&token),
            &format!("/v1/reports/flows?{period}"),
            "",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        for row in report["currencies"].as_array().unwrap() {
            for key in ["income", "expenses", "net_flow"] {
                assert_eq!(row[key], "0.00");
            }
        }
    }
    assert_eq!(
        send(router.clone(), Some(&foreign_token), one_day, "")
            .await
            .2["currencies"][2]["income"],
        "999.99"
    );
    for (credential, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("invalid"), StatusCode::UNAUTHORIZED),
        (Some(pending.as_str()), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            send(router.clone(), credential, one_day, "").await.0,
            expected
        );
    }
    for field in ["user_id", "owner_id", "account_id"] {
        assert_eq!(
            send(
                router.clone(),
                Some(&token),
                &format!("{one_day}&{field}={foreign}"),
                ""
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(before, all_tables_snapshot(&pool).await);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/transactions/{id}/void"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("idempotency-key", "independent-void")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let before = all_tables_snapshot(&pool).await;
    let report = send(router, Some(&token), one_day, "").await;
    assert_eq!(report.0, StatusCode::OK);
    assert_eq!(report.2["currencies"][2]["expenses"], "0.00");
    assert_eq!(before, all_tables_snapshot(&pool).await);
}
