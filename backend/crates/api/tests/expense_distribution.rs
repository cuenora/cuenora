//! Expense distribution through the public HTTP boundary.
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

const URI: &str = "/v1/reports/expense-distribution?date_from=2024-02-01&date_to=2024-02-29";

async fn expense(
    pool: &PgPool,
    owner: &str,
    currency: &str,
    minor: i64,
    category: Option<&str>,
    date: &str,
) {
    sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,category_id,occurred_on) VALUES($1::uuid,'expense',$2,$3,$4::uuid,$5::date)")
        .bind(owner).bind(currency).bind(minor).bind(category).bind(date).execute(pool).await.unwrap();
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn categories_reconcile_by_currency_with_inclusive_dates_and_no_writes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (foreign, _) = seed(&pool, true).await;
    let housing = "10000000-0000-4000-8000-000000000004";
    let groceries = "10000000-0000-4000-8000-000000000005";
    expense(&pool, &owner, "SEK", 1, Some(housing), "2024-02-01").await;
    expense(&pool, &owner, "SEK", 1, Some(groceries), "2024-02-29").await;
    expense(&pool, &owner, "SEK", 1, None, "2024-02-15").await;
    expense(&pool, &owner, "EUR", i64::MAX, None, "2024-02-15").await;
    expense(&pool, &owner, "EUR", i64::MAX, None, "2024-02-15").await;
    for (user, date) in [
        (owner.as_str(), "2024-01-31"),
        (&owner, "2024-03-01"),
        (&foreign, "2024-02-15"),
    ] {
        expense(&pool, user, "SEK", 999, Some(housing), date).await;
    }
    sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on,state) VALUES($1::uuid,'income','SEK',999,'2024-02-15','posted'),($1::uuid,'expense','SEK',999,'2024-02-15','voided')")
        .bind(&owner).execute(&pool).await.unwrap();
    let credentials = common::snapshot(&pool).await;
    let before = all_tables_snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    for _ in 0..2 {
        let (status, _, body) = send(router.clone(), Some(&token), URI, "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["date_from"], "2024-02-01");
        assert_eq!(body["date_to"], "2024-02-29");
        let currencies = body["currencies"].as_array().unwrap();
        assert_eq!(currencies.len(), 3);
        for (i, currency) in ["SEK", "EUR", "USD"].iter().enumerate() {
            assert_eq!(currencies[i]["currency"], *currency);
            assert_eq!(currencies[i]["categories"].as_array().unwrap().len(), 8);
        }
        assert_eq!(currencies[0]["expenses"], "0.03");
        assert_eq!(
            currencies[0]["categories"][0],
            json!({"category_id":housing,"code":"housing","name":"Housing","expenses":"0.01","percentage":"33.33"})
        );
        assert_eq!(currencies[0]["categories"][1]["percentage"], "33.33");
        assert_eq!(
            currencies[0]["categories"][7],
            json!({"category_id":null,"code":null,"name":null,"expenses":"0.01","percentage":"33.33"})
        );
        assert_eq!(currencies[1]["expenses"], "184467440737095516.14");
        assert_eq!(currencies[1]["categories"][7]["percentage"], "100.00");
        assert_eq!(currencies[2]["expenses"], "0.00");
        for row in currencies[2]["categories"].as_array().unwrap() {
            assert_eq!(row["expenses"], "0.00");
            assert!(row["percentage"].is_null());
        }
        let flows = send(
            router.clone(),
            Some(&token),
            &URI.replace("expense-distribution", "flows"),
            "",
        )
        .await
        .2;
        for i in 0..3 {
            assert_eq!(
                currencies[i]["expenses"],
                flows["currencies"][i]["expenses"]
            );
        }
    }
    assert_eq!(before, all_tables_snapshot(&pool).await);
    assert_eq!(credentials, common::snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn half_even_ties_round_down_even_up_odd_and_empty_period_has_no_shares() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    for (amount, category) in [
        (1, Some("10000000-0000-4000-8000-000000000004")),
        (3, Some("10000000-0000-4000-8000-000000000005")),
        (19996, None),
    ] {
        expense(&pool, &owner, "SEK", amount, category, "2024-02-15").await;
    }
    let router = app(Some(pool));
    let (status, _, body) = send(router.clone(), Some(&token), URI, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["currencies"][0]["categories"][0]["percentage"], "0.00");
    assert_eq!(body["currencies"][0]["categories"][1]["percentage"], "0.02");
    assert_eq!(
        body["currencies"][0]["categories"][7]["percentage"],
        "99.98"
    );
    let (status, _, body) = send(
        router,
        Some(&token),
        "/v1/reports/expense-distribution?date_from=0001-01-01&date_to=0001-01-01",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for c in body["currencies"].as_array().unwrap() {
        assert_eq!(c["expenses"], "0.00");
        for row in c["categories"].as_array().unwrap() {
            assert!(row["percentage"].is_null());
        }
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
        "/v1/reports/expense-distribution",
        "/v1/reports/expense-distribution?date_from=2024-02-01",
        "/v1/reports/expense-distribution?date_from=2024-02-30&date_to=2024-03-01",
        "/v1/reports/expense-distribution?date_from=2024-03-01&date_to=2024-02-01",
        "/v1/reports/expense-distribution?date_from=2024-2-01&date_to=2024-03-01",
        "/v1/reports/expense-distribution?date_from=0000-01-01&date_to=2024-03-01",
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

async fn mutate_probe(
    router: Router,
    token: &str,
    method: &str,
    path: &str,
    key: &str,
    body: Value,
) {
    let response = router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("idempotency-key", key)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn corrections_archived_accounts_ownership_and_exact_near_tie_shares() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (foreign, foreign_token) = seed(&pool, true).await;
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let housing = "10000000-0000-4000-8000-000000000004";
    let grocery = "10000000-0000-4000-8000-000000000005";
    let account:String=sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Verifier','normal','SEK') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    let destination:String=sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Verifier destination','normal','SEK') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    let id:String=sqlx::query_scalar("INSERT INTO transactions(user_id,kind,amount_minor,currency,occurred_on,category_id) VALUES($1::uuid,'expense',100,'SEK','2024-02-15',$2::uuid) RETURNING id::text").bind(&owner).bind(housing).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO transactions(user_id,kind,amount_minor,currency,occurred_on,account_id,category_id) VALUES($1::uuid,'expense',500,'SEK','2024-02-29',$2::uuid,$3::uuid),($1::uuid,'opening_balance',999,'SEK','2024-02-15',$2::uuid,NULL),($1::uuid,'interest_income',999,'SEK','2024-02-15',$2::uuid,NULL),($1::uuid,'income',999,'SEK','2024-02-15',$2::uuid,NULL)").bind(&owner).bind(&account).bind(housing).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO transactions(user_id,kind,amount_minor,currency,occurred_on,account_id,destination_account_id,received_amount_minor,received_currency) VALUES($1::uuid,'transfer',999,'SEK','2024-02-15',$2::uuid,$3::uuid,999,'SEK')").bind(&owner).bind(&account).bind(&destination).execute(&pool).await.unwrap();
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&account)
        .execute(&pool)
        .await
        .unwrap();
    let rule:String=sqlx::query_scalar("INSERT INTO recurring_rules(user_id,amount_minor,currency,cadence,starts_on,time_zone) VALUES($1::uuid,999,'SEK','monthly','2024-02-15','Etc/UTC') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO recurring_occurrences(rule_id,user_id,scheduled_on,due_on,amount_minor,currency) VALUES($1::uuid,$2::uuid,'2024-02-15','2024-02-15',999,'SEK')").bind(rule).bind(&owner).execute(&pool).await.unwrap();
    expense(&pool, &foreign, "SEK", 12345, None, "2024-02-15").await;
    let router = app(Some(pool.clone()));
    assert_eq!(
        send(router.clone(), Some(&token), URI, "").await.2["currencies"][0]["expenses"],
        "6.00"
    );
    mutate_probe(
        router.clone(),
        &token,
        "PATCH",
        &format!("/v1/transactions/{id}"),
        "probe-recat",
        json!({"amount":"2.35","category_id":grocery,"occurred_on":"2024-02-01"}),
    )
    .await;
    let report = send(router.clone(), Some(&token), URI, "").await.2;
    assert_eq!(report["currencies"][0]["expenses"], "7.35");
    assert_eq!(report["currencies"][0]["categories"][0]["expenses"], "5.00");
    assert_eq!(report["currencies"][0]["categories"][1]["expenses"], "2.35");
    mutate_probe(
        router.clone(),
        &token,
        "PATCH",
        &format!("/v1/transactions/{id}"),
        "probe-date",
        json!({"occurred_on":"2024-03-01"}),
    )
    .await;
    assert_eq!(
        send(router.clone(), Some(&token), URI, "").await.2["currencies"][0]["expenses"],
        "5.00"
    );
    mutate_probe(
        router.clone(),
        &token,
        "PATCH",
        &format!("/v1/transactions/{id}"),
        "probe-return",
        json!({"occurred_on":"2024-02-29"}),
    )
    .await;
    mutate_probe(
        router.clone(),
        &token,
        "POST",
        &format!("/v1/transactions/{id}/void"),
        "probe-void",
        json!({}),
    )
    .await;
    assert_eq!(
        send(router.clone(), Some(&token), URI, "").await.2["currencies"][0]["expenses"],
        "5.00"
    );
    // Independent exact u128 oracle: values straddle a half tie by a single minor unit.
    for (day, base, delta) in [
        (16, 400_000_000_000_000_i64, -1_i64),
        (17, 400_000_000_000_000, 0),
        (18, 400_000_000_000_000, 1),
        (19, 1_200_000_000_000_000, -1),
        (20, 1_200_000_000_000_000, 0),
        (21, 1_200_000_000_000_000, 1),
    ] {
        let numerator = base + delta;
        let total = 8_000_000_000_000_000_000_i64;
        let date = format!("2024-02-{day}");
        expense(&pool, &owner, "USD", numerator, Some(housing), &date).await;
        expense(&pool, &owner, "USD", total - numerator, None, &date).await;
        let uri = format!("/v1/reports/expense-distribution?date_from={date}&date_to={date}");
        let report = send(router.clone(), Some(&token), &uri, "").await.2;
        for (index, amount) in [(0, numerator), (7, total - numerator)] {
            let n = amount as u128 * 10000;
            let d = total as u128;
            let q = n / d;
            let rem = n % d;
            let expected = q + u128::from(rem * 2 > d || (rem * 2 == d && q % 2 == 1));
            assert_eq!(
                report["currencies"][2]["categories"][index]["percentage"],
                format!("{}.{:02}", expected / 100, expected % 100)
            );
        }
    }
    let before = all_tables_snapshot(&pool).await;
    for (token,uri,body,status) in [
        (token.as_str(),URI.to_string(),"",StatusCode::OK),
        (foreign_token.as_str(),URI.to_string(),"",StatusCode::OK),
        (token.as_str(),format!("{URI}&user_id={foreign}"),"",StatusCode::BAD_REQUEST),
        (token.as_str(),format!("{URI}&category_id={housing}"),"",StatusCode::BAD_REQUEST),
        (token.as_str(),"/v1/reports/expense-distribution?date_from=2024-02-01%27%20OR%201=1--&date_to=2024-02-29".into(),"",StatusCode::BAD_REQUEST),
        (token.as_str(),URI.to_string(),"{}",StatusCode::BAD_REQUEST),
    ] {
        let result=send(router.clone(),Some(token),&uri,body).await;
        assert_eq!(result.0,status,"{}",result.2);
        if status==StatusCode::OK {
            let flows=send(router.clone(),Some(token),&uri.replace("expense-distribution","flows"),"").await.2;
            for i in 0..3 {assert_eq!(result.2["currencies"][i]["expenses"],flows["currencies"][i]["expenses"]);}
            if token==foreign_token {assert_eq!(result.2["currencies"][0]["expenses"],"123.45");}
        }
        assert_eq!(before,all_tables_snapshot(&pool).await);
    }
}
