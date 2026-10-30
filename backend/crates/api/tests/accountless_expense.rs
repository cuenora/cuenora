//! Accountless expense through the public API; synthetic isolated data only.
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
    json!({"kind":"expense","amount":"300.00","currency":"SEK","occurred_on":"2024-02-29"})
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
    let financial: String = sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),(SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id),'[]') FROM account_movements m),(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM transaction_audit a),(SELECT coalesce(jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash),'[]') FROM income_creation_keys k),(SELECT coalesce(jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash),'[]') FROM expense_creation_keys k),(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM accounts a))::text)").fetch_one(pool).await.unwrap();
    format!("{}{}", common::snapshot(pool).await, financial)
}

#[tokio::test]
async fn expense_transport_rejects_invalid_bearer_before_input_validation() {
    let token = "synthetic";
    // Authentication still precedes validation even without a configured database.
    error(
        post(app(None), token, "expense", input()).await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn creates_exact_expense_without_movements_and_preserves_income_namespace() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let account: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Synthetic','normal','SEK') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    let opening: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,account_id) VALUES($1::uuid,'opening_balance','2024-01-01',100000,'SEK',$2::uuid) RETURNING id::text").bind(&owner).bind(&account).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,100000,'SEK','2024-01-01')").bind(&owner).bind(opening).bind(&account).execute(&pool).await.unwrap();
    let router = app(Some(pool.clone()));
    let category: String = sqlx::query_scalar(
        "SELECT id::text FROM categories WHERE kind='expense' ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    for currency in ["SEK", "EUR", "USD"] {
        let mut body = input();
        body["currency"] = json!(currency);
        body["category_id"] = json!(category);
        body["description"] = json!("Synthetic expense");
        let reply = post(router.clone(), &token, currency, body.clone()).await;
        assert_eq!(reply.0, StatusCode::CREATED, "{}", reply.1);
        let id = reply.1["id"].as_str().unwrap();
        assert_eq!(
            reply.1,
            json!({"id":id,"kind":"expense","state":"posted","amount":"300.00","currency":currency,"occurred_on":"2024-02-29","account_id":null,"category_id":category,"description":"Synthetic expense"})
        );
        let stored: (String, i64, bool) = sqlx::query_as("SELECT user_id::text,amount_minor,kind='expense' AND state='posted' AND account_id IS NULL AND category_id=$2::uuid AND currency=$3 AND occurred_on=DATE '2024-02-29' AND description='Synthetic expense' FROM transactions WHERE id=$1::uuid").bind(id).bind(&category).bind(currency).fetch_one(&pool).await.unwrap();
        assert_eq!(stored, (owner.clone(), 30000, true));
        let before = snapshot(&pool).await;
        assert_eq!(post(router.clone(), &token, currency, body).await, reply);
        assert_eq!(snapshot(&pool).await, before);
        // Identical keys belong to independent operation namespaces.
        let mut income = input();
        income["kind"] = json!("income");
        income["currency"] = json!(currency);
        let income = post(router.clone(), &token, currency, income).await;
        assert_eq!(income.0, StatusCode::CREATED);
        assert_ne!(income.1["id"], reply.1["id"]);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_movements WHERE user_id=$1::uuid AND transaction_id IN (SELECT id FROM transactions WHERE kind='expense')")
            .bind(&owner)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let balance: i64 = sqlx::query_scalar(
        "SELECT sum(signed_amount_minor)::bigint FROM account_movements WHERE account_id=$1::uuid",
    )
    .bind(&account)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(balance, 100000);

    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid AND actor_id=user_id AND action='create' AND changed_fields=ARRAY[]::text[]").bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(audits, 6);
    let total: i64 = sqlx::query_scalar("SELECT sum(amount_minor)::bigint FROM transactions WHERE user_id=$1::uuid AND kind='expense' AND currency='SEK' AND state='posted'").bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(total, 30000);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn permanent_normalized_retries_conflicts_and_owner_isolation() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (_, other) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let first = post(router.clone(), &token, "retry", input()).await;
    assert_eq!(first.0, StatusCode::CREATED);
    // Fixed P05.10 request hash: permanent pre-upgrade keys must still replay.
    let stored: String = sqlx::query_scalar("SELECT encode(request_hash,'hex') FROM expense_creation_keys WHERE transaction_id=$1::uuid")
        .bind(first.1["id"].as_str().unwrap()).fetch_one(&pool).await.unwrap();
    assert_eq!(
        stored,
        "9228082deb7f8381d7d03345ff6199b98f945957cf00203ad437c195d26e6dc6"
    );
    let mut normalized = input();
    normalized["amount"] = json!("0300");
    normalized["account_id"] = Value::Null;
    let before = snapshot(&pool).await;
    assert_eq!(
        post(router.clone(), &token, "retry", normalized).await,
        first
    );
    for (field, value) in [
        ("amount", json!("301")),
        ("currency", json!("EUR")),
        ("occurred_on", json!("2024-03-01")),
        ("description", json!("")),
    ] {
        let mut body = input();
        body[field] = value;
        error(
            post(router.clone(), &token, "retry", body).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    sqlx::query("UPDATE transactions SET description='later edit',updated_at=statement_timestamp() WHERE id=$1::uuid").bind(first.1["id"].as_str().unwrap()).execute(&pool).await.unwrap();
    let edited = snapshot(&pool).await;
    assert_eq!(post(router.clone(), &token, "retry", input()).await, first);
    assert_eq!(snapshot(&pool).await, edited);
    let other_reply = post(router.clone(), &other, "retry", input()).await;
    assert_eq!(other_reply.0, StatusCode::CREATED);
    assert_ne!(other_reply.1["id"], first.1["id"]);
    sqlx::query("UPDATE users SET profile_time_zone=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(post(router.clone(), &token, "retry", input()).await, first);
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    error(
        post(router, &token, "retry", input()).await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
    for statement in [
        "UPDATE expense_creation_keys SET key_hash=key_hash",
        "DELETE FROM expense_creation_keys",
        "TRUNCATE expense_creation_keys",
    ] {
        assert!(sqlx::query(statement).execute(&pool).await.is_err());
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn rejected_expenses_leave_financial_state_unchanged() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (_, unverified) = seed(&pool, false, Some("Etc/UTC")).await;
    let (_, unzoned) = seed(&pool, true, None).await;
    let router = app(Some(pool.clone()));
    let category: String = sqlx::query_scalar(
        "SELECT id::text FROM categories WHERE kind='income' ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool).await;
    for (field, value) in [
        ("amount", json!("0")),
        ("amount", json!("-1")),
        ("amount", json!(1)),
        ("amount", json!("1.001")),
        ("currency", json!("GBP")),
        ("occurred_on", json!("9999-12-31")),
        ("occurred_on", json!("2024-02-30")),
        ("category_id", json!(category)),
        ("account_id", json!("malformed")),
        ("description", json!("x".repeat(1001))),
        ("description", json!("line\nfeed")),
        ("user_id", json!("injected")),
    ] {
        let mut body = input();
        body[field] = value;
        assert_eq!(
            post(router.clone(), &token, "bad", body).await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    error(
        post(router.clone(), &unverified, "bad", input()).await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    error(
        post(router.clone(), &unzoned, "bad", input()).await,
        StatusCode::CONFLICT,
        "profile_zone_unconfirmed",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_retries_create_one_event_and_late_failure_rolls_back() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let (a, b) = tokio::join!(
        post(router.clone(), &token, "race", input()),
        post(router.clone(), &token, "race", input())
    );
    assert_eq!(a.0, StatusCode::CREATED);
    assert_eq!(a, b);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM transactions WHERE user_id=$1::uuid")
        .bind(&owner)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    // A key-table constraint fails after event and audit writes.
    sqlx::query(
        "ALTER TABLE expense_creation_keys ADD CONSTRAINT synthetic_failure CHECK(false) NOT VALID",
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool).await;
    let failed = post(router, &token, "late", input()).await;
    sqlx::query("ALTER TABLE expense_creation_keys DROP CONSTRAINT synthetic_failure")
        .execute(&pool)
        .await
        .unwrap();
    error(
        failed,
        StatusCode::SERVICE_UNAVAILABLE,
        "expense_creation_unavailable",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn competing_requests_normalize_categories_and_lock_timeouts_write_nothing() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let category: String = sqlx::query_scalar(
        "SELECT id::text FROM categories WHERE kind='expense' ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut a = input();
    a["category_id"] = json!(category.to_uppercase());
    let mut b = a.clone();
    b["amount"] = json!("301");
    let (x, y) = tokio::join!(
        post(router.clone(), &token, "compete", a.clone()),
        post(router.clone(), &token, "compete", b.clone())
    );
    assert!(matches!(
        (x.0, y.0),
        (StatusCode::CREATED, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::CREATED)
    ));
    let (mut winner, response) = if x.0 == StatusCode::CREATED {
        (a, x)
    } else {
        (b, y)
    };
    winner["category_id"] = json!(category);
    let before = snapshot(&pool).await;
    assert_eq!(
        post(router.clone(), &token, "compete", winner).await,
        response
    );
    assert_eq!(snapshot(&pool).await, before);
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid),(SELECT count(*) FROM expense_creation_keys WHERE user_id=$1::uuid)").bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (1, 1, 1));
    // Holding the same owner lock models a racing profile update. Failure must leave no event or key.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
        .bind(&owner)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let failed = post(router, &token, "locked", input()).await;
    tx.rollback().await.unwrap();
    error(
        failed,
        StatusCode::SERVICE_UNAVAILABLE,
        "expense_creation_unavailable",
    );
    assert_eq!(snapshot(&pool).await, before);
}

async fn account(pool: &PgPool, owner: &str, kind: &str, currency: &str) -> String {
    sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Synthetic',$2,$3) RETURNING id::text")
        .bind(owner).bind(kind).bind(currency).fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn linked_expenses_post_once_and_allow_savings_overdraft() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    for kind in ["normal", "savings"] {
        let account = account(&pool, &owner, kind, "SEK").await;
        let mut body = input();
        body["account_id"] = json!(account.to_uppercase());
        let (a, b) = tokio::join!(
            post(router.clone(), &token, kind, body.clone()),
            post(router.clone(), &token, kind, body.clone())
        );
        assert_eq!(a.0, StatusCode::CREATED, "{}", a.1);
        assert_eq!(a, b);
        assert_eq!(a.1["account_id"], account);
        let id = a.1["id"].as_str().unwrap();
        let stored: (i64, bool) = sqlx::query_as("SELECT amount_minor,kind='expense' AND account_id=$2::uuid AND currency='SEK' AND occurred_on=DATE '2024-02-29' FROM transactions WHERE id=$1::uuid").bind(id).bind(&account).fetch_one(&pool).await.unwrap();
        assert_eq!(stored, (30000, true));
        let posting: (i64, i64, bool) = sqlx::query_as("SELECT count(*),sum(signed_amount_minor)::bigint,bool_and(user_id=$2::uuid AND transaction_id=$3::uuid AND currency='SEK' AND occurred_on=DATE '2024-02-29') FROM account_movements WHERE account_id=$1::uuid").bind(&account).bind(&owner).bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(posting, (1, -30000, true));
        let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM transaction_audit WHERE transaction_id=$1::uuid AND user_id=$2::uuid AND actor_id=user_id AND action='create' AND changed_fields=ARRAY[]::text[]").bind(id).bind(&owner).fetch_one(&pool).await.unwrap();
        assert_eq!(audits, 1);
        let balances = request(router.clone(), "GET", "/v1/accounts", Some(&token), &[], "").await;
        assert_eq!(balances.0, StatusCode::OK);
        let listed = balances.1["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == account)
            .unwrap();
        assert_eq!(listed["balance"], "-300.00");
        body["account_id"] = json!(account);
        body["amount"] = json!("0300");
        // Replay is not a new posting: mutable account status is not revalidated.
        sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
            .bind(&account)
            .execute(&pool)
            .await
            .unwrap();
        let before = snapshot(&pool).await;
        assert_eq!(post(router.clone(), &token, kind, body.clone()).await, a);
        assert_eq!(snapshot(&pool).await, before);
        body["account_id"] = Value::Null;
        error(
            post(router.clone(), &token, kind, body).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn linked_expense_rejections_and_late_failure_leave_no_writes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (other, _) = seed(&pool, true, Some("Etc/UTC")).await;
    let (_, unverified) = seed(&pool, false, Some("Etc/UTC")).await;
    let (_, unzoned) = seed(&pool, true, None).await;
    let valid = account(&pool, &owner, "normal", "SEK").await;
    let foreign = account(&pool, &other, "normal", "SEK").await;
    let wrong_currency = account(&pool, &owner, "normal", "EUR").await;
    let archived = account(&pool, &owner, "normal", "SEK").await;
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&archived)
        .execute(&pool)
        .await
        .unwrap();
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    let mut not_found = None;
    for (account, status, code) in [
        (foreign.as_str(), StatusCode::NOT_FOUND, "not_found"),
        (
            "00000000-0000-0000-0000-000000000000",
            StatusCode::NOT_FOUND,
            "not_found",
        ),
        (
            wrong_currency.as_str(),
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
        (
            archived.as_str(),
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
        ("malformed", StatusCode::BAD_REQUEST, "invalid_request"),
    ] {
        let mut body = input();
        body["account_id"] = json!(account);
        let reply = post(router.clone(), &token, "rejected", body).await;
        if status == StatusCode::NOT_FOUND {
            if let Some(previous) = &not_found {
                assert_eq!(&reply, previous);
            }
            not_found = Some(reply.clone());
        }
        error(reply, status, code);
        assert_eq!(snapshot(&pool).await, before);
    }
    let mut body = input();
    body["account_id"] = json!(valid);
    for (bearer, status, code) in [
        (&unverified, StatusCode::FORBIDDEN, "email_not_verified"),
        (&unzoned, StatusCode::CONFLICT, "profile_zone_unconfirmed"),
    ] {
        error(
            post(router.clone(), bearer, "gate", body.clone()).await,
            status,
            code,
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    sqlx::query("ALTER TABLE expense_creation_keys ADD CONSTRAINT synthetic_linked_failure CHECK(false) NOT VALID").execute(&pool).await.unwrap();
    let failed = post(router.clone(), &token, "late-linked", body.clone()).await;
    sqlx::query("ALTER TABLE expense_creation_keys DROP CONSTRAINT synthetic_linked_failure")
        .execute(&pool)
        .await
        .unwrap();
    error(
        failed,
        StatusCode::SERVICE_UNAVAILABLE,
        "expense_creation_unavailable",
    );
    assert_eq!(snapshot(&pool).await, before);
    // A timed-out account lock cannot create an event, posting or retry key.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1::uuid FOR UPDATE")
        .bind(&valid)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let failed = post(router.clone(), &token, "locked-linked", body).await;
    tx.rollback().await.unwrap();
    error(
        failed,
        StatusCode::SERVICE_UNAVAILABLE,
        "expense_creation_unavailable",
    );
    assert_eq!(snapshot(&pool).await, before);
    let accountless = post(router.clone(), &token, "old-accountless", input()).await;
    assert_eq!(accountless.0, StatusCode::CREATED);
    let before = snapshot(&pool).await;
    let mut linked = input();
    linked["account_id"] = json!(valid);
    error(
        post(router, &token, "old-accountless", linked).await,
        StatusCode::CONFLICT,
        "idempotency_conflict",
    );
    assert_eq!(snapshot(&pool).await, before);
}
