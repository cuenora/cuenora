//! Recorded transaction history through the public HTTP interface.
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

async fn snapshot(pool: &PgPool) -> String {
    let financial: String = sqlx::query_scalar(
        "SELECT md5(jsonb_build_array(
        (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM transactions t),
        (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM account_movements m),
        (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM transaction_audit a),
        (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM accounts a),
        (SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
        (SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM income_creation_keys k),
        (SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM expense_creation_keys k)
    )::text)",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    format!("{}:{financial}", common::snapshot(pool).await)
}

async fn account(pool: &PgPool, owner: &str, savings: bool, currency: &str) -> String {
    sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'History', $2,$3) RETURNING id::text")
        .bind(owner).bind(if savings { "savings" } else { "normal" }).bind(currency)
        .fetch_one(pool).await.unwrap()
}

async fn event(
    pool: &PgPool,
    owner: &str,
    kind: &str,
    account: Option<&str>,
    date: &str,
) -> String {
    sqlx::query_scalar("INSERT INTO transactions(user_id,kind,account_id,currency,amount_minor,occurred_on) VALUES($1::uuid,$2,$3::uuid,'SEK',12345,$4::date) RETURNING id::text")
        .bind(owner).bind(kind).bind(account).bind(date).fetch_one(pool).await.unwrap()
}

fn ids(body: &Value) -> Vec<&str> {
    body["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect()
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn every_recorded_kind_has_exact_fields_and_pages_without_duplicates() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let normal = account(&pool, &owner, false, "SEK").await;
    let savings = account(&pool, &owner, true, "EUR").await;
    let mut expected = Vec::new();
    for (kind, linked) in [
        ("income", None),
        ("expense", Some(normal.as_str())),
        ("opening_balance", Some(normal.as_str())),
    ] {
        expected.push(event(&pool, &owner, kind, linked, "2024-02-29").await);
    }
    let transfer: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,account_id,destination_account_id,currency,amount_minor,received_currency,received_amount_minor,occurred_on) VALUES($1::uuid,'transfer',$2::uuid,$3::uuid,'SEK',9223372036854775807,'EUR',1795,'2024-02-29') RETURNING id::text")
        .bind(&owner).bind(&normal).bind(&savings).fetch_one(&pool).await.unwrap();
    expected.push(transfer.clone());
    let interest: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,account_id,currency,amount_minor,occurred_on) VALUES($1::uuid,'interest_income',$2::uuid,'EUR',1,'2024-02-29') RETURNING id::text")
        .bind(&owner).bind(&savings).fetch_one(&pool).await.unwrap();
    expected.push(interest);
    let oldest = event(
        &pool,
        &owner,
        "opening_balance",
        Some(&normal),
        "0001-01-01",
    )
    .await;
    sqlx::query("UPDATE transactions SET amount_minor=-9223372036854775807,state='voided' WHERE id=$1::uuid").bind(&oldest).execute(&pool).await.unwrap();
    expected.sort_by(|a, b| b.cmp(a));
    expected.push(oldest.clone());
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&normal)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let mut seen = Vec::new();
    let mut uri = "/v1/transactions?limit=2".to_owned();
    loop {
        let reply = send(router.clone(), Some(&token), &uri, "").await;
        assert_eq!(reply.0, StatusCode::OK, "{}", reply.2);
        for transaction in reply.2["transactions"].as_array().unwrap() {
            assert_eq!(transaction.as_object().unwrap().len(), 12);
            if transaction["id"] == transfer {
                assert_eq!(transaction["amount"], "92233720368547758.07");
                assert_eq!(transaction["received_amount"], "17.95");
                assert_eq!(transaction["received_currency"], "EUR");
                assert_eq!(transaction["destination_account_id"], savings);
            } else if transaction["id"] == oldest {
                assert_eq!(transaction["amount"], "-92233720368547758.07");
                assert_eq!(transaction["occurred_on"], "0001-01-01");
                assert_eq!(transaction["state"], "voided");
            }
            seen.push(transaction["id"].as_str().unwrap().to_owned());
        }
        let Some(cursor) = reply.2["next_cursor"].as_str() else {
            break;
        };
        assert_eq!(Some(cursor), seen.last().map(String::as_str));
        uri = format!("/v1/transactions?limit=2&cursor={cursor}");
    }
    assert_eq!(seen, expected);
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn dates_categories_and_both_transfer_accounts_compose_with_accountless_filter() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let a = account(&pool, &owner, false, "SEK").await;
    let b = account(&pool, &owner, false, "SEK").await;
    let category: String = sqlx::query_scalar(
        "SELECT id::text FROM categories WHERE kind='expense' ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let expense = event(&pool, &owner, "expense", Some(&a), "2024-02-29").await;
    sqlx::query("UPDATE transactions SET category_id=$1::uuid WHERE id=$2::uuid")
        .bind(&category)
        .bind(&expense)
        .execute(&pool)
        .await
        .unwrap();
    let unlinked = event(&pool, &owner, "income", None, "2024-02-29").await;
    event(&pool, &owner, "expense", Some(&a), "2024-03-01").await;
    event(&pool, &owner, "expense", Some(&a), "2024-02-28").await;
    let transfer: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,account_id,destination_account_id,currency,amount_minor,received_currency,received_amount_minor,occurred_on) VALUES($1::uuid,'transfer',$2::uuid,$3::uuid,'SEK',100,'SEK',100,'2024-02-29') RETURNING id::text")
        .bind(&owner).bind(&a).bind(&b).fetch_one(&pool).await.unwrap();
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    for (query, expected) in [
        (
            format!(
                "date_from=2024-02-29&date_to=2024-02-29&category_id={category}&account_id={a}"
            ),
            vec![expense.as_str()],
        ),
        ("account_id=none".into(), vec![unlinked.as_str()]),
        (format!("account_id={b}"), vec![transfer.as_str()]),
        ("date_from=9999-12-31".into(), vec![]),
        ("date_to=0001-01-01".into(), vec![]),
    ] {
        let reply = send(
            router.clone(),
            Some(&token),
            &format!("/v1/transactions?{query}"),
            "",
        )
        .await;
        assert_eq!(reply.0, StatusCode::OK, "{}", reply.2);
        assert_eq!(ids(&reply.2), expected);
    }
    let reply = send(
        router.clone(),
        Some(&token),
        &format!("/v1/transactions?account_id={a}&date_from=2024-02-29&date_to=2024-02-29&limit=1"),
        "",
    )
    .await;
    let cursor = reply.2["next_cursor"].as_str().unwrap();
    let next = send(router, Some(&token), &format!("/v1/transactions?account_id={a}&date_from=2024-02-29&date_to=2024-02-29&limit=1&cursor={cursor}"), "").await;
    assert_eq!(next.0, StatusCode::OK);
    assert_eq!(ids(&reply.2).len() + ids(&next.2).len(), 2);
    assert_ne!(ids(&reply.2), ids(&next.2));
    assert!(next.2["next_cursor"].is_null());
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn authority_invalid_filters_and_cursors_never_leak_or_write() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (other, other_token) = seed(&pool, true).await;
    let (_, pending) = seed(&pool, false).await;
    let foreign_account = account(&pool, &other, false, "SEK").await;
    let foreign = event(
        &pool,
        &other,
        "income",
        Some(&foreign_account),
        "2024-01-01",
    )
    .await;
    let own = event(&pool, &owner, "income", None, "2024-01-01").await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    assert_eq!(
        send(router.clone(), None, "/v1/transactions", "").await.0,
        StatusCode::UNAUTHORIZED
    );
    let denied = send(router.clone(), Some(&pending), "/v1/transactions", "").await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    assert_eq!(denied.2["code"], "email_not_verified");
    for (credential, expected) in [(&token, &own), (&other_token, &foreign)] {
        let reply = send(router.clone(), Some(credential), "/v1/transactions", "").await;
        assert_eq!(reply.0, StatusCode::OK);
        assert_eq!(ids(&reply.2), vec![expected.as_str()]);
    }
    for query in [
        "limit=0".to_owned(),
        "limit=101".into(),
        "limit=65536".into(),
        "limit=1&limit=2".into(),
        "cursor=".into(),
        "cursor=bad".into(),
        format!("cursor={foreign}"),
        "cursor=00000000-0000-0000-0000-000000000000".into(),
        format!("account_id={foreign_account}"),
        "account_id=00000000-0000-0000-0000-000000000000".into(),
        "account_id=bad".into(),
        "category_id=bad".into(),
        "category_id=00000000-0000-0000-0000-000000000000".into(),
        "date_from=2024-02-30".into(),
        "date_to=2024-2-01".into(),
        "date_to=0000-01-01".into(),
        "date_from=2025-01-01&date_to=2024-01-01".into(),
        "date_from=2024-01-01T00:00:00Z".into(),
        "user_id=ignored".into(),
        "kind=income".into(),
        format!("cursor={own}&date_from=2025-01-01"),
        "date_to=10000-01-01".into(),
        format!("unknown={}", "x".repeat(1100)),
    ] {
        let reply = send(
            router.clone(),
            Some(&token),
            &format!("/v1/transactions?{query}"),
            "",
        )
        .await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST, "{query}: {}", reply.2);
        assert_eq!(reply.2["code"], "invalid_request");
    }
    assert_eq!(
        send(router.clone(), Some(&token), "/v1/transactions", "{}")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        send(router, Some(&token), "/v1/transactions", "").await.0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn default_and_max_limits_are_bounded_and_database_failures_are_safe() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on) SELECT $1::uuid,'income','SEK',n,'2024-01-01' FROM generate_series(1,101) n")
        .bind(&owner).execute(&pool).await.unwrap();
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    for (query, size) in [("", 50), ("?limit=100", 100)] {
        let reply = send(
            router.clone(),
            Some(&token),
            &format!("/v1/transactions{query}"),
            "",
        )
        .await;
        assert_eq!(reply.0, StatusCode::OK);
        assert_eq!(ids(&reply.2).len(), size);
        assert!(reply.2["next_cursor"].is_string());
    }
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE transactions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let reply = send(router, Some(&token), "/v1/transactions", "").await;
    assert_eq!(reply.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.2["code"], "transaction_listing_unavailable");
    assert!(reply.2.get("transactions").is_none());
    lock.rollback().await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn shared_route_preserves_existing_income_and_expense_creation_and_query_rejection() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let router = app(Some(pool.clone()));
    for kind in ["income", "expense"] {
        let body = json!({"kind":kind,"amount":"1.23","currency":"SEK","occurred_on":"2024-01-01"})
            .to_string();
        let before = snapshot(&pool).await;
        let response = router
            .clone()
            .oneshot(
                Request::post("/v1/transactions?limit=0&date_from=bad")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("idempotency-key", format!("history-{kind}"))
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(snapshot(&pool).await, before);
        let response = router
            .clone()
            .oneshot(
                Request::post("/v1/transactions")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("idempotency-key", format!("history-{kind}"))
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
        let reply = send(router.clone(), Some(&token), "/v1/transactions", "").await;
        let item = reply.2["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == created["id"])
            .unwrap();
        for (key, value) in created.as_object().unwrap() {
            assert_eq!(&item[key], value);
        }
    }
}
