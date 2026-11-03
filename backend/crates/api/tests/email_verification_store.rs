//! Store boundary tests; PostgreSQL cases run only in the disposable harness.
mod common;

use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    middleware,
    routing::{get, post},
};
use cuenora_api::{
    auth_identity::{AuthenticatedIdentity, require_identity},
    email_verification_store::{self, IssuedChallenge},
    login_store::{issue_initial_session, lookup_credentials},
    registration_policy::{Email, hash_password},
    registration_store::persist_registration,
    session_tokens::generate,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::Barrier;
use tower::ServiceExt;

struct TestAccount {
    access_token: String,
    user_id: String,
}

async fn seed(pool: &PgPool, email: &str) -> TestAccount {
    let parsed = Email::parse(email).unwrap();
    let password_hash = hash_password("synthetic verification password 42").unwrap();
    persist_registration(pool, &parsed, &password_hash)
        .await
        .unwrap();
    let credentials = lookup_credentials(pool, &parsed).await.unwrap().unwrap();
    let tokens = generate().unwrap();
    assert!(
        issue_initial_session(pool, &credentials, &tokens)
            .await
            .unwrap()
    );
    let user_id: String =
        sqlx::query_scalar("SELECT user_id::text FROM session_access_tokens WHERE token_hash=$1")
            .bind(tokens.access.digest().as_slice())
            .fetch_one(pool)
            .await
            .unwrap();
    TestAccount {
        access_token: tokens.access.expose_secret().to_owned(),
        user_id,
    }
}

fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/issue", get(issue_route))
        .route("/confirm", post(confirm_route))
        .route_layer(middleware::from_fn_with_state(
            Some(pool.clone()),
            require_identity,
        ))
        .with_state(pool)
}

async fn issue_route(
    axum::extract::State(pool): axum::extract::State<PgPool>,
    Extension(identity): Extension<AuthenticatedIdentity>,
) -> (StatusCode, Json<Value>) {
    match email_verification_store::issue(&pool, &identity).await {
        Ok(Some(challenge)) => (
            StatusCode::OK,
            Json(json!({
                "email": challenge.email,
                "token": challenge.token,
                "idempotency_key": challenge.idempotency_key,
            })),
        ),
        Ok(None) => (StatusCode::NO_CONTENT, Json(Value::Null)),
        Err(email_verification_store::StoreError::InvalidSession) => {
            (StatusCode::UNAUTHORIZED, Json(Value::Null))
        }
        Err(email_verification_store::StoreError::InvalidChallenge) => {
            (StatusCode::BAD_REQUEST, Json(Value::Null))
        }
        Err(email_verification_store::StoreError::Unavailable) => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(Value::Null))
        }
    }
}

async fn confirm_route(
    axum::extract::State(pool): axum::extract::State<PgPool>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Json(body): Json<Value>,
) -> StatusCode {
    let Some(token) = body.get("token").and_then(Value::as_str) else {
        return StatusCode::BAD_REQUEST;
    };
    match email_verification_store::confirm(&pool, &identity, token).await {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(email_verification_store::StoreError::InvalidSession) => StatusCode::UNAUTHORIZED,
        Err(email_verification_store::StoreError::InvalidChallenge) => StatusCode::BAD_REQUEST,
        Err(email_verification_store::StoreError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn call_issue(pool: &PgPool, access: &str) -> (StatusCode, Value) {
    let response = app(pool.clone())
        .oneshot(
            Request::get("/issue")
                .header(header::AUTHORIZATION, format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let json = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    (status, json)
}

async fn call_confirm(pool: &PgPool, access: &str, token: &str) -> StatusCode {
    app(pool.clone())
        .oneshot(
            Request::post("/confirm")
                .header(header::AUTHORIZATION, format!("Bearer {access}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"token": token}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

async fn issue(pool: &PgPool, account: &TestAccount) -> IssuedChallenge {
    let (status, body) = call_issue(pool, &account.access_token).await;
    assert_eq!(status, StatusCode::OK);
    IssuedChallenge {
        email: body["email"].as_str().unwrap().to_owned(),
        token: body["token"].as_str().unwrap().to_owned(),
        idempotency_key: body["idempotency_key"].as_str().unwrap().to_owned(),
    }
}

async fn state_snapshot(pool: &PgPool, user_id: &str) -> String {
    let digest: String = sqlx::query_scalar(
        "SELECT md5(jsonb_build_array(
            (SELECT to_jsonb(u) FROM users u WHERE id=$1::uuid),
            (SELECT to_jsonb(c) FROM password_credentials c WHERE user_id=$1::uuid),
            (SELECT coalesce(jsonb_agg(to_jsonb(v) ORDER BY id), '[]') FROM email_verification_challenges v WHERE user_id=$1::uuid),
            (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s WHERE user_id=$1::uuid),
            (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM session_access_tokens a WHERE user_id=$1::uuid),
            (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM session_refresh_tokens r WHERE user_id=$1::uuid)
        )::text)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap();
    format!("{}:{digest}", common::snapshot(pool).await)
}

async fn challenge_digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

async fn wait_for_credential_lock(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_stat_activity
                 WHERE pid<>pg_backend_pid() AND wait_event_type='Lock'
                   AND query LIKE '%FROM password_credentials c JOIN users u%'",
            )
            .fetch_one(pool)
            .await
            .unwrap();
            if waiting > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("store operation did not wait for the credential lock");
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn issue_and_confirm_verify_only_the_bound_account_and_consume_once() {
    let pool = common::pool().await;
    let account = seed(&pool, "verification-store-basic@example.test").await;
    let other = seed(&pool, "verification-store-other@example.test").await;
    let challenge = issue(&pool, &account).await;
    assert_eq!(challenge.email, "verification-store-basic@example.test");
    assert!(challenge.idempotency_key.starts_with("email-verification-"));
    assert_eq!(challenge.token.len(), 68);

    let stored: Vec<u8> = sqlx::query_scalar(
        "SELECT token_hash FROM email_verification_challenges WHERE user_id=$1::uuid",
    )
    .bind(&account.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored.as_slice(),
        challenge_digest(&challenge.token).await.as_slice()
    );
    assert!(!String::from_utf8_lossy(&stored).contains(&challenge.token));

    assert_eq!(
        call_confirm(&pool, &other.access_token, &challenge.token).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call_confirm(&pool, &account.access_token, &challenge.token).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call_confirm(&pool, &account.access_token, &challenge.token).await,
        StatusCode::BAD_REQUEST
    );
    let rows = sqlx::query(
        "SELECT u.email_verified_at IS NOT NULL AS verified, v.consumed_at IS NOT NULL AS consumed
         FROM users u JOIN email_verification_challenges v ON v.user_id=u.id
         WHERE u.id=$1::uuid",
    )
    .bind(&account.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(rows.get::<bool, _>("verified"));
    assert!(rows.get::<bool, _>("consumed"));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn invalid_purpose_unknown_expired_and_stale_bindings_do_not_mutate_state() {
    let pool = common::pool().await;
    let account = seed(&pool, "verification-store-invalid@example.test").await;

    let challenge = issue(&pool, &account).await;
    let before = state_snapshot(&pool, &account.user_id).await;
    for token in [
        format!("pr1_{}", "0".repeat(64)),
        format!("ev1_{}", "0".repeat(64)),
        "ev1_not-hex".to_owned(),
    ] {
        assert_eq!(
            call_confirm(&pool, &account.access_token, &token).await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(state_snapshot(&pool, &account.user_id).await, before);
    }

    let digest = challenge_digest(&challenge.token).await;
    sqlx::query(
        "UPDATE email_verification_challenges SET expires_at=clock_timestamp() WHERE token_hash=$1",
    )
    .bind(digest.as_slice())
    .execute(&pool)
    .await
    .unwrap();
    let before_expired = state_snapshot(&pool, &account.user_id).await;
    assert_eq!(
        call_confirm(&pool, &account.access_token, &challenge.token).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        state_snapshot(&pool, &account.user_id).await,
        before_expired
    );

    sqlx::query("UPDATE email_verification_challenges SET created_at=clock_timestamp()-interval '2 minutes' WHERE token_hash=$1")
        .bind(digest.as_slice()).execute(&pool).await.unwrap();
    let stale = issue(&pool, &account).await;
    sqlx::query("UPDATE password_credentials SET credential_revision=credential_revision+1 WHERE user_id=$1::uuid")
        .bind(&account.user_id).execute(&pool).await.unwrap();
    let before_revision = state_snapshot(&pool, &account.user_id).await;
    assert_eq!(
        call_confirm(&pool, &account.access_token, &stale.token).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        state_snapshot(&pool, &account.user_id).await,
        before_revision
    );

    let stale_digest = challenge_digest(&stale.token).await;
    sqlx::query("UPDATE email_verification_challenges SET created_at=clock_timestamp()-interval '2 minutes' WHERE token_hash=$1")
        .bind(stale_digest.as_slice()).execute(&pool).await.unwrap();
    let email_stale = issue(&pool, &account).await;
    let email_digest = challenge_digest(&email_stale.token).await;
    sqlx::query("UPDATE email_verification_challenges SET created_at=clock_timestamp()-interval '2 minutes' WHERE token_hash=$1")
        .bind(email_digest.as_slice()).execute(&pool).await.unwrap();
    sqlx::query("UPDATE users SET email_original='verification-store-invalid@changed.test', email_address='verification-store-invalid@changed.test' WHERE id=$1::uuid")
        .bind(&account.user_id).execute(&pool).await.unwrap();
    let before_email = state_snapshot(&pool, &account.user_id).await;
    assert_eq!(
        call_confirm(&pool, &account.access_token, &email_stale.token).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(state_snapshot(&pool, &account.user_id).await, before_email);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn resend_cooldown_and_verified_account_suppression_are_read_only() {
    let pool = common::pool().await;
    let account = seed(&pool, "verification-store-policy@example.test").await;
    let challenge = issue(&pool, &account).await;
    let before = state_snapshot(&pool, &account.user_id).await;
    let (status, _) = call_issue(&pool, &account.access_token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(state_snapshot(&pool, &account.user_id).await, before);

    assert_eq!(
        call_confirm(&pool, &account.access_token, &challenge.token).await,
        StatusCode::NO_CONTENT
    );
    let verified_before = state_snapshot(&pool, &account.user_id).await;
    let (status, _) = call_issue(&pool, &account.access_token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        state_snapshot(&pool, &account.user_id).await,
        verified_before
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn five_issued_challenges_reach_the_live_and_hourly_caps() {
    let pool = common::pool().await;
    let account = seed(&pool, "verification-store-cap@example.test").await;
    for _ in 0..5 {
        let challenge = issue(&pool, &account).await;
        let digest = challenge_digest(&challenge.token).await;
        sqlx::query("UPDATE email_verification_challenges SET created_at=clock_timestamp()-interval '2 minutes' WHERE token_hash=$1")
            .bind(digest.as_slice()).execute(&pool).await.unwrap();
    }
    let before = state_snapshot(&pool, &account.user_id).await;
    let (status, _) = call_issue(&pool, &account.access_token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(state_snapshot(&pool, &account.user_id).await, before);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn failed_verification_write_rolls_back_challenge_consumption() {
    let pool = common::pool().await;
    let account = seed(&pool, "verification-store-rollback@example.test").await;
    let challenge = issue(&pool, &account).await;
    let constraint = format!(
        "ALTER TABLE users ADD CONSTRAINT email_verification_test_fail_check CHECK (id <> '{}'::uuid OR email_verified_at IS NULL) NOT VALID",
        account.user_id
    );
    sqlx::query(&constraint).execute(&pool).await.unwrap();
    let before = state_snapshot(&pool, &account.user_id).await;
    assert_eq!(
        call_confirm(&pool, &account.access_token, &challenge.token).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(state_snapshot(&pool, &account.user_id).await, before);
    sqlx::query("ALTER TABLE users DROP CONSTRAINT email_verification_test_fail_check")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn concurrent_confirmation_commits_exactly_one_verification() {
    let pool = common::pool().await;
    let account = seed(&pool, "verification-store-race@example.test").await;
    let challenge = issue(&pool, &account).await;
    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let pool = pool.clone();
        let access = account.access_token.clone();
        let token = challenge.token.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            call_confirm(&pool, &access, &token).await
        }));
    }
    barrier.wait().await;
    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.unwrap());
    }
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::NO_CONTENT)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::BAD_REQUEST)
            .count(),
        1
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn expired_or_revoked_bearer_cannot_issue_or_confirm_after_admission() {
    let pool = common::pool().await;
    for (suffix, mutation) in [
        (
            "revoked",
            "UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=(SELECT session_id FROM session_access_tokens WHERE token_hash=$1)",
        ),
        (
            "expired",
            "UPDATE session_access_tokens SET expires_at=clock_timestamp() WHERE token_hash=$1",
        ),
    ] {
        let account = seed(
            &pool,
            &format!("verification-store-session-{suffix}@example.test"),
        )
        .await;
        let challenge = issue(&pool, &account).await;
        let digest: [u8; 32] = Sha256::digest(account.access_token.as_bytes()).into();
        let mut blocker = pool.begin().await.unwrap();
        sqlx::query("SELECT user_id FROM password_credentials WHERE user_id=$1::uuid FOR UPDATE")
            .bind(&account.user_id)
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        let pool_for_request = pool.clone();
        let access = account.access_token.clone();
        let token = challenge.token.clone();
        let request =
            tokio::spawn(async move { call_confirm(&pool_for_request, &access, &token).await });
        wait_for_credential_lock(&pool).await;
        sqlx::query(mutation)
            .bind(digest.as_slice())
            .execute(&pool)
            .await
            .unwrap();
        blocker.commit().await.unwrap();
        assert_eq!(request.await.unwrap(), StatusCode::UNAUTHORIZED);
    }
    pool.close().await;
}
