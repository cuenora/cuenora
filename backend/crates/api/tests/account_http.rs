//! Account creation acceptance via HTTP and disposable PostgreSQL, not a verification bypass.
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

async fn send(
    router: Router,
    token: Option<&str>,
    key: Option<&str>,
    body: String,
) -> (StatusCode, Value) {
    let mut request =
        Request::post("/v1/accounts").header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(key) = key {
        request = request.header("idempotency-key", key);
    }
    let response = router
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
    let body = http::checked_body(&headers, String::from_utf8(bytes.to_vec()).unwrap());
    (status, serde_json::from_str(&body).unwrap())
}
fn input(name: &str, kind: &str, currency: &str) -> String {
    json!({"name":name,"account_type":kind,"currency":currency}).to_string()
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
    let financial: String = sqlx::query_scalar("SELECT jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM accounts a),(SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY account_id),'[]') FROM account_creation_keys i),(SELECT count(*) FROM account_movements))::text")
        .fetch_one(pool).await.unwrap();
    format!("{}{}", common::snapshot(pool).await, financial)
}
fn error(reply: &(StatusCode, Value), status: StatusCode, code: &str) {
    assert_eq!(reply.0, status);
    assert_eq!(
        reply.1,
        serde_json::from_str::<Value>(&http::error_body(code)).unwrap()
    );
    assert_eq!(reply.1["details"], Value::Null);
    assert_eq!(reply.1["request_id"], "<request-id>");
}

#[tokio::test]
async fn creation_requires_bearer_even_without_storage() {
    error(
        &send(
            app(None),
            None,
            Some("key"),
            input("Daily", "normal", "SEK"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
    let token = generate().unwrap();
    error(
        &send(
            app(None),
            Some(token.access.expose_secret()),
            Some("key"),
            input("Daily", "normal", "SEK"),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
        "authentication_unavailable",
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn creates_supported_accounts_with_authenticated_owner_and_no_movements() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM account_movements")
        .fetch_one(&pool)
        .await
        .unwrap();
    for kind in ["normal", "savings"] {
        for currency in ["SEK", "EUR", "USD"] {
            let reply = send(
                router.clone(),
                Some(&token),
                Some(&format!("{kind}-{currency}")),
                input(" \u{2003}Daily 水  ", kind, currency),
            )
            .await;
            assert_eq!(reply.0, StatusCode::CREATED);
            assert_eq!(reply.1["name"], "Daily 水");
            assert_eq!(reply.1["account_type"], kind);
            assert_eq!(reply.1["currency"], currency);
            assert_eq!(reply.1["status"], "active");
            assert_eq!(reply.1.as_object().unwrap().len(), 5);
            let stored: (String, String) =
                sqlx::query_as("SELECT user_id::text,name FROM accounts WHERE id=$1::uuid")
                    .bind(reply.1["id"].as_str().unwrap())
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(stored, (owner.clone(), "Daily 水".into()));
        }
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts WHERE user_id=$1::uuid")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .unwrap(),
        6
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM account_movements")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn invalid_inputs_and_keys_leave_all_financial_state_unchanged() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    for body in [
        "{".into(),
        "null".into(),
        "{}".into(),
        input("  ", "normal", "SEK"),
        input("bad\nname", "normal", "SEK"),
        input(&"水".repeat(256), "normal", "SEK"),
        input("Daily", "other", "SEK"),
        input("Daily", "normal", "GBP"),
        input("Daily", "normal", "sek"),
        r#"{"name":"A","name":"B","account_type":"normal","currency":"SEK"}"#.into(),
        r#"{"name":"A","account_type":"normal","currency":"SEK","user_id":"forged"}"#.into(),
        r#"{"name":"A","account_type":"normal","currency":"SEK","balance":"2.00"}"#.into(),
        r#"{"name":"A","account_type":"normal","currency":"SEK","opening_balance":"0"}"#.into(),
    ] {
        error(
            &send(router.clone(), Some(&token), Some("invalid"), body).await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for key in [
        None,
        Some(""),
        Some("has space"),
        Some("colon:key"),
        Some(&"x".repeat(129)),
    ] {
        error(
            &send(
                router.clone(),
                Some(&token),
                key,
                input("Daily", "normal", "SEK"),
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    error(
        &send(
            router,
            Some(&token),
            Some("big"),
            input(&"x".repeat(8192), "normal", "SEK"),
        )
        .await,
        StatusCode::PAYLOAD_TOO_LARGE,
        "invalid_request",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn pending_expired_and_revoked_identities_cannot_write() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, false).await;
    let (expired, expired_token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    error(
        &send(
            router.clone(),
            Some(&token),
            Some("pending"),
            input("A", "normal", "EUR"),
        )
        .await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    error(
        &send(
            router.clone(),
            None,
            Some("pending"),
            input("A", "normal", "EUR"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
    sqlx::query("UPDATE user_sessions SET revoked_at=statement_timestamp() WHERE user_id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    error(
        &send(
            router.clone(),
            Some(&token),
            Some("pending"),
            input("A", "normal", "EUR"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
    sqlx::query("UPDATE session_access_tokens SET created_at=statement_timestamp()-interval '2 hours',expires_at=statement_timestamp()-interval '1 hour' WHERE user_id=$1::uuid").bind(expired).execute(&pool).await.unwrap();
    error(
        &send(
            router,
            Some(&expired_token),
            Some("expired"),
            input("A", "normal", "EUR"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn original_result_replays_for_lifetime_and_changed_content_conflicts() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let first = send(
        router.clone(),
        Some(&token),
        Some("original"),
        input(" Daily ", "normal", "SEK"),
    )
    .await;
    assert_eq!(first.0, StatusCode::CREATED);
    // Future edits must not replace the creation snapshot used by a retry.
    sqlx::query("UPDATE accounts SET name='Renamed' WHERE id=$1::uuid")
        .bind(first.1["id"].as_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE account_creation_keys SET created_at=statement_timestamp()-interval '10 years' WHERE user_id=$1::uuid").bind(&owner).execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    assert_eq!(
        send(
            router.clone(),
            Some(&token),
            Some("original"),
            input("Daily", "normal", "SEK")
        )
        .await,
        first
    );
    for body in [
        input("Other", "normal", "SEK"),
        input("Daily", "savings", "SEK"),
        input("Daily", "normal", "EUR"),
    ] {
        error(
            &send(router.clone(), Some(&token), Some("original"), body).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let other_key = send(
        router.clone(),
        Some(&token),
        Some("different"),
        input("Daily", "normal", "SEK"),
    )
    .await;
    assert_eq!(other_key.0, StatusCode::CREATED);
    assert_ne!(other_key.1["id"], first.1["id"]);
    let (_, second_token) = seed(&pool, true).await;
    let second_owner = send(
        router,
        Some(&second_token),
        Some("original"),
        input("Daily", "normal", "SEK"),
    )
    .await;
    assert_eq!(second_owner.0, StatusCode::CREATED);
    assert_ne!(second_owner.1["id"], first.1["id"]);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_retries_create_exactly_once_and_conflicting_retry_writes_nothing() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let router = router.clone();
        let token = token.clone();
        tasks.push(tokio::spawn(async move {
            send(
                router,
                Some(&token),
                Some("concurrent"),
                input("Same", "savings", "USD"),
            )
            .await
        }));
    }
    let first = tasks.remove(0).await.unwrap();
    assert_eq!(first.0, StatusCode::CREATED);
    for task in tasks {
        assert_eq!(task.await.unwrap(), first);
    }
    let (a, b) = tokio::join!(
        send(
            router.clone(),
            Some(&token),
            Some("conflicting"),
            input("A", "normal", "SEK")
        ),
        send(
            router,
            Some(&token),
            Some("conflicting"),
            input("B", "normal", "SEK")
        )
    );
    assert!(
        (a.0 == StatusCode::CREATED && b.0 == StatusCode::CONFLICT)
            || (b.0 == StatusCode::CREATED && a.0 == StatusCode::CONFLICT)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts WHERE user_id=$1::uuid")
            .bind(&owner)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM account_creation_keys WHERE user_id=$1::uuid"
        )
        .bind(owner)
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn failed_key_insert_rolls_back_account_and_allows_retry() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    sqlx::raw_sql("CREATE FUNCTION reject_account_key() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END $$; CREATE TRIGGER reject_account_key BEFORE INSERT ON account_creation_keys FOR EACH ROW EXECUTE FUNCTION reject_account_key();").execute(&pool).await.unwrap();
    let reply = send(
        router.clone(),
        Some(&token),
        Some("rollback"),
        input("A", "normal", "EUR"),
    )
    .await;
    sqlx::raw_sql("DROP TRIGGER reject_account_key ON account_creation_keys; DROP FUNCTION reject_account_key();").execute(&pool).await.unwrap();
    error(
        &reply,
        StatusCode::SERVICE_UNAVAILABLE,
        "account_creation_unavailable",
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(
        send(
            router,
            Some(&token),
            Some("rollback"),
            input("A", "normal", "EUR")
        )
        .await
        .0,
        StatusCode::CREATED
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn duplicate_keys_and_unverified_replay_write_nothing() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let first = send(
        router.clone(),
        Some(&token),
        Some("independent"),
        input("A", "normal", "EUR"),
    )
    .await;
    assert_eq!(first.0, StatusCode::CREATED);
    let before = snapshot(&pool).await;
    for values in [vec!["one", "two"], vec!["one,two"], vec!["é"]] {
        let mut req = Request::post("/v1/accounts")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"));
        for val in values {
            req = req.header("idempotency-key", val);
        }
        let response = router
            .clone()
            .oneshot(req.body(Body::from(input("A", "normal", "EUR"))).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(snapshot(&pool).await, before);
    }
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    error(
        &send(
            router,
            Some(&token),
            Some("independent"),
            input("A", "normal", "EUR"),
        )
        .await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
}
