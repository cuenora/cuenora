//! Accountless income through the public API; synthetic isolated data only.
mod common;
#[path = "common/http.rs"]
mod http;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn request(
    router: Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    keys: &[&str],
    raw: &str,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    for key in keys {
        req = req.header("idempotency-key", *key);
    }
    let reply = router
        .oneshot(req.body(Body::from(raw.to_owned())).unwrap())
        .await
        .unwrap();
    let status = reply.status();
    let headers = reply.headers().clone();
    if status != StatusCode::NOT_FOUND {
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    }
    let bytes = to_bytes(reply.into_body(), 16384).await.unwrap();
    let body = http::checked_body(&headers, String::from_utf8(bytes.to_vec()).unwrap());
    (status, serde_json::from_str(&body).unwrap())
}
async fn post(router: Router, token: &str, key: &str, input: Value) -> (StatusCode, Value) {
    request(
        router,
        "POST",
        "/v1/transactions",
        Some(token),
        &[key],
        &input.to_string(),
    )
    .await
}
fn input() -> Value {
    json!({"kind":"income","amount":"300.00","currency":"SEK","occurred_on":"2024-02-29"})
}
fn error(reply: (StatusCode, Value), status: StatusCode, code: &str) {
    assert_eq!(reply.0, status, "{}", reply.1);
    assert_eq!(reply.1["code"], code);
    assert_eq!(reply.1["details"], Value::Null);
    assert_eq!(reply.1["request_id"], "<request-id>");
    assert_eq!(reply.1.as_object().unwrap().len(), 4);
    if matches!(
        code,
        "invalid_request" | "invalid_access_token" | "email_not_verified" | "idempotency_conflict"
    ) {
        assert_eq!(
            reply.1,
            serde_json::from_str::<Value>(&http::error_body(code)).unwrap()
        );
    }
}
async fn seed(pool: &PgPool, verified: bool, zone: Option<&str>) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let owner: String = sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at,profile_time_zone) VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END,$3) RETURNING id::text").bind(email).bind(verified).bind(zone).fetch_one(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&owner)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at) VALUES($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')").bind(session).bind(&owner).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    (owner, tokens.access.expose_secret().to_owned())
}
async fn snapshot(pool: &PgPool) -> String {
    let financial: String = sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),(SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id),'[]') FROM account_movements m),(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM transaction_audit a),(SELECT coalesce(jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash),'[]') FROM income_creation_keys k),(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM accounts a))::text)").fetch_one(pool).await.unwrap();
    format!("{}{}", common::snapshot(pool).await, financial)
}

#[tokio::test]
async fn financial_creation_requires_bearer_before_input_validation() {
    error(
        request(app(None), "POST", "/v1/transactions", None, &[], "{").await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn creates_exact_owned_income_with_audit_and_no_balance_effect() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Europe/Stockholm")).await;
    // Two independently funded accounts, including savings; neither is inferred.
    let mut accounts = vec![];
    for (kind, amount) in [("normal", 100000_i64), ("savings", 50000)] {
        let id: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Synthetic',$2,'SEK') RETURNING id::text").bind(&owner).bind(kind).fetch_one(&pool).await.unwrap();
        let event: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,account_id) VALUES($1::uuid,'opening_balance','2024-01-01',$2,'SEK',$3::uuid) RETURNING id::text").bind(&owner).bind(amount).bind(&id).fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,$4,'SEK','2024-01-01')").bind(&owner).bind(event).bind(&id).bind(amount).execute(&pool).await.unwrap();
        accounts.push((id, amount));
    }
    let category: String = sqlx::query_scalar(
        "SELECT id::text FROM categories WHERE kind='income' ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let router = app(Some(pool.clone()));
    for currency in ["SEK", "EUR", "USD"] {
        let mut body = input();
        body["currency"] = json!(currency);
        body["category_id"] = json!(category);
        body["description"] = json!("Synthetic income");
        let reply = post(router.clone(), &token, currency, body).await;
        assert_eq!(reply.0, StatusCode::CREATED, "{}", reply.1);
        let id = reply.1["id"].as_str().unwrap();
        assert_eq!(
            reply.1,
            json!({"id":id,"kind":"income","state":"posted","amount":"300.00","currency":currency,"occurred_on":"2024-02-29","account_id":null,"category_id":category,"description":"Synthetic income"})
        );
        let event: (String,i64,bool) = sqlx::query_as("SELECT user_id::text,amount_minor,account_id IS NULL AND destination_account_id IS NULL AND state='posted' AND kind='income' AND category_id=$2::uuid AND description='Synthetic income' AND occurred_on=DATE '2024-02-29' AND currency=$3 FROM transactions WHERE id=$1::uuid").bind(id).bind(&category).bind(currency).fetch_one(&pool).await.unwrap();
        assert_eq!(event, (owner.clone(), 30000, true));
        let audit: (String,String,Vec<String>) = sqlx::query_as("SELECT user_id::text,action,changed_fields FROM transaction_audit WHERE transaction_id=$1::uuid AND actor_id=user_id").bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(audit, (owner.clone(), "create".into(), vec![]));
    }
    let totals: Vec<(String,i64)> = sqlx::query_as("SELECT account_id::text,sum(signed_amount_minor)::bigint FROM account_movements WHERE user_id=$1::uuid GROUP BY account_id ORDER BY account_id").bind(&owner).fetch_all(&pool).await.unwrap();
    accounts.sort();
    assert_eq!(totals, accounts);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM account_movements m JOIN transactions t ON t.id=m.transaction_id WHERE t.user_id=$1::uuid AND t.kind='income'").bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
    let total: i64 = sqlx::query_scalar("SELECT sum(amount_minor)::bigint FROM transactions WHERE user_id=$1::uuid AND kind='income' AND currency='SEK' AND state='posted'").bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(total, 30000);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn retries_are_permanent_normalized_owner_scoped_and_conflict_without_writes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (_, other) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let original = post(router.clone(), &token, "same", input()).await;
    assert_eq!(original.0, StatusCode::CREATED);
    let before = snapshot(&pool).await;
    let mut normalized = input();
    normalized["amount"] = json!("0300");
    normalized["account_id"] = Value::Null;
    normalized["description"] = Value::Null;
    normalized["category_id"] = Value::Null;
    assert_eq!(
        post(router.clone(), &token, "same", normalized).await,
        original
    );
    for (field, value) in [
        ("amount", json!("301")),
        ("currency", json!("EUR")),
        ("occurred_on", json!("2024-03-01")),
        ("description", json!("different")),
    ] {
        let mut body = input();
        body[field] = value;
        error(
            post(router.clone(), &token, "same", body).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    // Replay is original creation output, not a read of a later edited event.
    sqlx::query(
        "UPDATE transactions SET description='later edit' WHERE id=$1::uuid AND user_id=$2::uuid",
    )
    .bind(original.1["id"].as_str().unwrap())
    .bind(&owner)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE users SET profile_time_zone=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        post(router.clone(), &token, "same", input()).await,
        original
    );
    let different = post(router.clone(), &other, "same", input()).await;
    assert_eq!(different.0, StatusCode::CREATED);
    assert_ne!(original.1["id"], different.1["id"]);
    // Namespace does not collide with account creation keys.
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let account = request(
        router,
        "POST",
        "/v1/accounts",
        Some(&token),
        &["same"],
        r#"{"name":"Same key","account_type":"normal","currency":"SEK"}"#,
    )
    .await;
    assert_eq!(account.0, StatusCode::CREATED);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_retries_have_one_event_audit_and_key() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let (a, b) = tokio::join!(
        post(router.clone(), &token, "concurrent", input()),
        post(router.clone(), &token, "concurrent", input())
    );
    assert_eq!(a.0, StatusCode::CREATED);
    assert_eq!(a, b);
    let mut changed = input();
    changed["amount"] = json!("400");
    let (a, b) = tokio::join!(
        post(router.clone(), &token, "race", input()),
        post(router, &token, "race", changed)
    );
    assert!(
        (a.0 == StatusCode::CREATED && b.0 == StatusCode::CONFLICT)
            || (b.0 == StatusCode::CREATED && a.0 == StatusCode::CONFLICT)
    );
    let counts:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid),(SELECT count(*) FROM income_creation_keys WHERE user_id=$1::uuid),(SELECT count(*) FROM account_movements WHERE user_id=$1::uuid)").bind(owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (2, 2, 2, 0));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn financial_gates_and_invalid_input_leave_everything_unchanged() {
    let pool = common::pool().await;
    let (_, pending) = seed(&pool, false, Some("Etc/UTC")).await;
    let (owner, token) = seed(&pool, true, None).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    error(
        post(router.clone(), &pending, "key", input()).await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    error(
        post(router.clone(), &token, "key", input()).await,
        StatusCode::CONFLICT,
        "profile_zone_unconfirmed",
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    let expense: String =
        sqlx::query_scalar("SELECT id::text FROM categories WHERE kind='expense' LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    let invalids = vec![
        ("kind", json!("transfer"), "invalid_request"),
        ("amount", json!(0), "invalid_request"),
        ("amount", json!("0"), "invalid_request"),
        ("amount", json!("-1"), "invalid_request"),
        ("amount", json!("1.001"), "invalid_request"),
        ("amount", json!("92233720368547758.08"), "invalid_request"),
        ("currency", json!("GBP"), "invalid_request"),
        ("occurred_on", json!("2024-02-30"), "invalid_business_date"),
        ("occurred_on", json!("0000-01-01"), "date_out_of_range"),
        ("occurred_on", json!("9999-12-31"), "future_business_date"),
        ("category_id", json!(expense), "invalid_request"),
        (
            "category_id",
            json!("00000000-0000-0000-0000-000000000000"),
            "invalid_request",
        ),
        ("category_id", json!("bad"), "invalid_request"),
        ("description", json!("x".repeat(1001)), "invalid_request"),
        ("description", json!("line\nbreak"), "invalid_request"),
        ("account_id", json!("bad"), "invalid_request"),
        ("user_id", json!(owner), "invalid_request"),
        ("state", json!("voided"), "invalid_request"),
    ];
    for (field, value, code) in invalids {
        let mut body = input();
        body[field] = value;
        error(
            post(router.clone(), &token, "key", body).await,
            StatusCode::BAD_REQUEST,
            code,
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for keys in [vec![], vec![""], vec!["bad key"], vec!["x", "y"]] {
        error(
            request(
                router.clone(),
                "POST",
                "/v1/transactions",
                Some(&token),
                &keys,
                &input().to_string(),
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for raw in [
        r#"{"kind":"income","kind":"income","amount":"1","currency":"SEK","occurred_on":"2024-01-01"}"#,
        "{}",
        "{",
    ] {
        error(
            request(
                router.clone(),
                "POST",
                "/v1/transactions",
                Some(&token),
                &["key"],
                raw,
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    error(
        request(
            router.clone(),
            "POST",
            "/v1/transactions?ignored=1",
            Some(&token),
            &["key"],
            &input().to_string(),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    error(
        post(router.clone(), "invalid-token", "key", input()).await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(
        post(router, &token, "key", input()).await.0,
        StatusCode::CREATED
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn audit_failure_rolls_back_event_and_retry_key() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    sqlx::query(&format!("ALTER TABLE transaction_audit ADD CONSTRAINT income_test_failure CHECK (user_id <> '{owner}'::uuid) NOT VALID")).execute(&pool).await.unwrap();
    error(
        post(router.clone(), &token, "retry", input()).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "income_creation_unavailable",
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("ALTER TABLE transaction_audit DROP CONSTRAINT income_test_failure")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        post(router, &token, "retry", input()).await.0,
        StatusCode::CREATED
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn retry_schema_is_owned_immutable_and_transactional() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (other, _) = seed(&pool, true, Some("Etc/UTC")).await;
    let reply = post(app(Some(pool.clone())), &token, "schema", input()).await;
    assert_eq!(reply.0, StatusCode::CREATED);
    let before = snapshot(&pool).await;
    for query in [
        "UPDATE income_creation_keys SET request_hash=request_hash".to_owned(),
        "DELETE FROM income_creation_keys WHERE false".to_owned(),
        "TRUNCATE income_creation_keys".to_owned(),
        format!(
            "INSERT INTO income_creation_keys(user_id,key_hash,request_hash,transaction_id) VALUES('{other}'::uuid,decode(repeat('ab',32),'hex'),decode(repeat('cd',32),'hex'),'{}'::uuid)",
            reply.1["id"].as_str().unwrap()
        ),
    ] {
        assert!(sqlx::query(&query).execute(&pool).await.is_err());
        assert_eq!(snapshot(&pool).await, before);
    }
    // A late key failure must roll back the event and audit too.
    sqlx::query("ALTER TABLE income_creation_keys ADD CONSTRAINT income_key_test_failure CHECK (false) NOT VALID").execute(&pool).await.unwrap();
    error(
        post(app(Some(pool.clone())), &token, "fail-key", input()).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "income_creation_unavailable",
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("ALTER TABLE income_creation_keys DROP CONSTRAINT income_key_test_failure")
        .execute(&pool)
        .await
        .unwrap();
    // Reapplication is rejected transactionally without altering existing keys.
    let mut tx = pool.begin().await.unwrap();
    assert!(
        sqlx::raw_sql(include_str!(
            "../../../migrations/0012_income_creation_keys.sql"
        ))
        .execute(&mut *tx)
        .await
        .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn bounded_transport_and_exact_boundaries_are_enforced() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    for (content_type, raw, status) in [
        (
            "text/plain",
            input().to_string(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            "application/json",
            format!("{}{}", input(), " ".repeat(8192)),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let reply = router
            .clone()
            .oneshot(
                Request::post("/v1/transactions")
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header("idempotency-key", "transport")
                    .body(Body::from(raw))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reply.status(), status);
        assert_eq!(reply.headers()[header::CACHE_CONTROL], "no-store");
        let body: Value =
            serde_json::from_slice(&to_bytes(reply.into_body(), 16384).await.unwrap()).unwrap();
        assert_eq!(body["code"], "invalid_request");
        assert_eq!(snapshot(&pool).await, before);
    }
    let mut body = input();
    body["amount"] = json!("92233720368547758.07");
    body["description"] = json!("é".repeat(1000));
    body["occurred_on"] = json!(
        chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now())
            .format("%Y-%m-%d")
            .to_string()
    );
    let reply = post(router, &token, "max", body).await;
    assert_eq!(reply.0, StatusCode::CREATED, "{}", reply.1);
    assert_eq!(reply.1["amount"], "92233720368547758.07");
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn category_conflicts_revoked_verification_and_hash_bounds() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let original = post(router.clone(), &token, "independent", input()).await;
    assert_eq!(original.0, StatusCode::CREATED);
    let cat: String =
        sqlx::query_scalar("SELECT id::text FROM categories WHERE kind='income' LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    let before = snapshot(&pool).await;
    let mut changed = input();
    changed["category_id"] = json!(cat.to_uppercase());
    error(
        post(router.clone(), &token, "independent", changed).await,
        StatusCode::CONFLICT,
        "idempotency_conflict",
    );
    assert_eq!(snapshot(&pool).await, before);
    for (key, request) in [(31, 32), (32, 31), (33, 32), (32, 33)] {
        let query = format!(
            "INSERT INTO income_creation_keys(user_id,key_hash,request_hash,transaction_id) VALUES($1::uuid,decode(repeat('ab',{key}),'hex'),decode(repeat('cd',{request}),'hex'),$2::uuid)"
        );
        let failure = sqlx::query(&query)
            .bind(&owner)
            .bind(original.1["id"].as_str().unwrap())
            .execute(&pool)
            .await
            .unwrap_err();
        assert_eq!(
            failure.as_database_error().unwrap().code().as_deref(),
            Some("23514")
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    error(
        post(router, &token, "independent", input()).await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
}

async fn account(pool: &PgPool, owner: &str, kind: &str, currency: &str) -> String {
    sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Synthetic linked',$2,$3) RETURNING id::text")
        .bind(owner).bind(kind).bind(currency).fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn linked_normal_income_posts_once_and_preserves_accountless_retries() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let legacy = post(router.clone(), &token, "legacy", input()).await;
    assert_eq!(legacy.0, StatusCode::CREATED);
    let alternative = account(&pool, &owner, "normal", "SEK").await;
    for currency in ["SEK", "EUR", "USD"] {
        let id = account(&pool, &owner, "normal", currency).await;
        let mut body = input();
        body["currency"] = json!(currency);
        body["account_id"] = json!(id.to_uppercase());
        let (a, b) = tokio::join!(
            post(router.clone(), &token, currency, body.clone()),
            post(router.clone(), &token, currency, body.clone())
        );
        assert_eq!(a.0, StatusCode::CREATED, "{}", a.1);
        assert_eq!(a, b);
        assert_eq!(a.1["account_id"], id);
        let event = a.1["id"].as_str().unwrap();
        let effects: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE id=$1::uuid AND account_id=$2::uuid AND user_id=$3::uuid AND kind='income' AND state='posted'),(SELECT count(*) FROM transaction_audit WHERE transaction_id=$1::uuid AND user_id=$3::uuid AND actor_id=user_id AND action='create' AND changed_fields=ARRAY[]::text[]),(SELECT count(*) FROM income_creation_keys WHERE transaction_id=$1::uuid AND user_id=$3::uuid),(SELECT count(*) FROM account_movements WHERE transaction_id=$1::uuid AND account_id=$2::uuid AND user_id=$3::uuid AND signed_amount_minor=30000 AND currency=$4 AND occurred_on=DATE '2024-02-29')")
            .bind(event).bind(&id).bind(&owner).bind(currency).fetch_one(&pool).await.unwrap();
        assert_eq!(effects, (1, 1, 1, 1));
        let listing = request(router.clone(), "GET", "/v1/accounts", Some(&token), &[], "").await;
        assert_eq!(listing.0, StatusCode::OK);
        let listed = listing.1["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == id)
            .unwrap();
        assert_eq!(listed["balance"], "300.00");
        assert_eq!(listed["currency"], currency);
        let before = snapshot(&pool).await;
        body["account_id"] = json!(alternative);
        error(
            post(router.clone(), &token, currency, body.clone()).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
        body["account_id"] = json!(id);
        error(
            post(router.clone(), &token, "legacy", body.clone()).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
        body["account_id"] = Value::Null;
        error(
            post(router.clone(), &token, currency, body.clone()).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
        body["account_id"] = json!(id);
        // A replay is not a new posting and survives later archive/profile changes.
        sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();
        let before = snapshot(&pool).await;
        assert_eq!(post(router.clone(), &token, currency, body).await, a);
        assert_eq!(snapshot(&pool).await, before);
    }
    assert_eq!(post(router, &token, "legacy", input()).await, legacy);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn linked_destination_rejections_and_late_failures_are_atomic() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (foreign, _) = seed(&pool, true, Some("Etc/UTC")).await;
    let savings = account(&pool, &owner, "savings", "SEK").await;
    let mismatch = account(&pool, &owner, "normal", "EUR").await;
    let archived = account(&pool, &owner, "normal", "SEK").await;
    let other = account(&pool, &foreign, "normal", "SEK").await;
    let normal = account(&pool, &owner, "normal", "SEK").await;
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&archived)
        .execute(&pool)
        .await
        .unwrap();
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    let mut missing_response = None;
    for (id, status, code) in [
        (&savings, StatusCode::BAD_REQUEST, "invalid_request"),
        (&mismatch, StatusCode::BAD_REQUEST, "invalid_request"),
        (&archived, StatusCode::BAD_REQUEST, "invalid_request"),
        (&other, StatusCode::NOT_FOUND, "not_found"),
        (
            &"00000000-0000-0000-0000-000000000000".to_owned(),
            StatusCode::NOT_FOUND,
            "not_found",
        ),
    ] {
        let mut body = input();
        body["account_id"] = json!(id);
        let reply = post(router.clone(), &token, "rejected", body).await;
        if status == StatusCode::NOT_FOUND {
            if let Some(previous) = &missing_response {
                assert_eq!(&reply, previous);
            }
            missing_response = Some(reply.clone());
        }
        error(reply, status, code);
        assert_eq!(snapshot(&pool).await, before);
    }
    let mut body = input();
    body["account_id"] = json!(normal);
    for table in [
        "account_movements",
        "transaction_audit",
        "income_creation_keys",
    ] {
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD CONSTRAINT linked_failure CHECK (false) NOT VALID"
        ))
        .execute(&pool)
        .await
        .unwrap();
        error(
            post(router.clone(), &token, "rejected", body.clone()).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "income_creation_unavailable",
        );
        assert_eq!(snapshot(&pool).await, before);
        sqlx::query(&format!(
            "ALTER TABLE {table} DROP CONSTRAINT linked_failure"
        ))
        .execute(&pool)
        .await
        .unwrap();
    }
    let accepted = post(router.clone(), &token, "rejected", body).await;
    assert_eq!(accepted.0, StatusCode::CREATED);
    let accountless = post(router, &token, "accountless-edit", input()).await;
    assert_eq!(accountless.0, StatusCode::CREATED);
    let before = snapshot(&pool).await;
    // Future edit/reassignment entry points inherit the persisted savings guard.
    for event in [
        accepted.1["id"].as_str().unwrap(),
        accountless.1["id"].as_str().unwrap(),
    ] {
        let failure = sqlx::query("UPDATE transactions SET account_id=$1::uuid WHERE id=$2::uuid")
            .bind(&savings)
            .bind(event)
            .execute(&pool)
            .await
            .unwrap_err();
        assert_eq!(
            failure.as_database_error().unwrap().constraint(),
            Some("transactions_income_account_type")
        );
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn posting_and_archive_serialize_without_posting_into_archived_accounts() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let id = account(&pool, &owner, "normal", "SEK").await;
    let mut body = input();
    body["account_id"] = json!(id);
    let path = format!("/v1/accounts/{id}");
    let (posting, archive) = tokio::join!(
        post(router.clone(), &token, "archive-race", body),
        request(
            router.clone(),
            "PATCH",
            &path,
            Some(&token),
            &[],
            r#"{"status":"archived"}"#
        )
    );
    let (active, count, total):(bool,i64,i64) = sqlx::query_as("SELECT archived_at IS NULL,(SELECT count(*) FROM account_movements WHERE account_id=$1::uuid),(SELECT coalesce(sum(signed_amount_minor),0)::bigint FROM account_movements WHERE account_id=$1::uuid) FROM accounts WHERE id=$1::uuid").bind(&id).fetch_one(&pool).await.unwrap();
    if posting.0 == StatusCode::CREATED {
        error(archive, StatusCode::CONFLICT, "account_balance_not_zero");
        assert_eq!((active, count, total), (true, 1, 30000));
    } else {
        error(posting, StatusCode::BAD_REQUEST, "invalid_request");
        assert_eq!(archive.0, StatusCode::OK);
        assert_eq!((active, count, total), (false, 0, 0));
    }
}
