//! Verification HTTP behavior against disposable PostgreSQL and in-memory mail.
mod common;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use cuenora_api::{
    auth_throttling::ClientIpPolicy,
    email_verification::VerificationTransport,
    registration_http::app_with_verification,
    session_tokens::{TokenLifetimes, generate},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{Notify, mpsc};
use tower::ServiceExt;
const REQUEST: &str = "/v1/auth/email-verification/request";
const CONFIRM: &str = "/v1/auth/email-verification/confirm";
struct Mail {
    messages: mpsc::UnboundedSender<(String, String, String)>,
    fail: bool,
    hold: Option<Arc<Notify>>,
}
impl VerificationTransport for Mail {
    fn send<'a>(
        &'a self,
        email: &'a str,
        token: &'a str,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async move {
            let _ = self.messages.send((email.into(), token.into(), key.into()));
            if let Some(hold) = &self.hold {
                hold.notified().await;
            }
            if self.fail { Err(()) } else { Ok(()) }
        })
    }
}
fn router(pool: Option<PgPool>, mail: Option<Mail>) -> Router {
    app_with_verification(
        pool,
        ClientIpPolicy::default(),
        TokenLifetimes::default(),
        mail.map(|m| Arc::new(m) as Arc<dyn VerificationTransport>),
    )
}
async fn send(
    app: Router,
    bearer: Option<&str>,
    path: &str,
    body: &str,
    client: u16,
) -> (StatusCode, Value) {
    let mut request = Request::post(path).header("content-type", "application/json");
    if let Some(bearer) = bearer {
        request = request.header("authorization", format!("Bearer {bearer}"));
    }
    let response = app
        .layer(Extension(ConnectInfo(
            format!("192.0.{}.{}:1", client / 255, client % 255)
                .parse::<std::net::SocketAddr>()
                .unwrap(),
        )))
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(response.headers().contains_key("x-request-id"));
    let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    if body.get("code").is_some() {
        assert!(body["details"].is_null());
        assert!(body["request_id"].is_string());
    }
    (status, body)
}
async fn seed(pool: &PgPool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("Case{}@example.test", &tokens.access.expose_secret()[4..24]);
    let owner: String = sqlx::query_scalar(
        "INSERT INTO users(email_original,email_address) VALUES($1,$1) RETURNING id::text",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO password_credentials(user_id,password_hash) VALUES($1::uuid,'$argon2id$synthetic-only')")
        .bind(&owner).execute(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&owner)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at) VALUES($1::uuid,$2::uuid,$3,clock_timestamp()+interval '15 minutes')")
        .bind(&session).bind(&owner).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    (owner, tokens.access.expose_secret().into())
}
async fn snapshot(pool: &PgPool) -> String {
    let tables = [
        "users",
        "password_credentials",
        "user_sessions",
        "session_access_tokens",
        "session_refresh_tokens",
        "email_verification_challenges",
        "accounts",
        "transactions",
        "account_movements",
        "transaction_audit",
    ];
    let rows=tables.iter().map(|table|format!("(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]') FROM {table} t)")).collect::<Vec<_>>().join(",");
    let digest: String =
        sqlx::query_scalar(&format!("SELECT md5(jsonb_build_array({rows})::text)"))
            .fetch_one(pool)
            .await
            .unwrap();
    format!("{}:{digest}", common::snapshot(pool).await)
}
async fn receive(
    rx: &mut mpsc::UnboundedReceiver<(String, String, String)>,
) -> (String, String, String) {
    tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap()
}
fn mail(fail: bool) -> (Mail, mpsc::UnboundedReceiver<(String, String, String)>) {
    let (messages, rx) = mpsc::unbounded_channel();
    (
        Mail {
            messages,
            fail,
            hold: None,
        },
        rx,
    )
}
async fn verified(pool: &PgPool, user: &str) -> bool {
    sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid")
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap()
}
#[tokio::test]
async fn authentication_precedes_verification_payload_parsing() {
    for path in [REQUEST, CONFIRM] {
        let (status, body) = send(router(None, None), None, path, "{", 1).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "invalid_access_token");
    }
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn requested_mail_does_not_verify_and_same_account_can_confirm_once() {
    let pool = common::pool().await;
    let (user, token) = seed(&pool).await;
    let (mail, mut rx) = mail(false);
    let app = router(Some(pool.clone()), Some(mail));
    let (status, body) = send(app.clone(), Some(&token), REQUEST, "{}", 1).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body, json!({"status":"accepted"}));
    let (email, challenge, key) = receive(&mut rx).await;
    let stored_email: String =
        sqlx::query_scalar("SELECT email_address FROM users WHERE id=$1::uuid")
            .bind(&user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(email == stored_email);
    assert!(email.starts_with("Case"));
    assert!(challenge.starts_with("ev1_"));
    assert!(!key.contains(&challenge));
    assert!(!verified(&pool, &user).await);
    let before_sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM user_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    let (status, body) = send(
        app.clone(),
        Some(&token),
        CONFIRM,
        &json!({"token":challenge}).to_string(),
        1,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_null());
    assert!(verified(&pool, &user).await);
    let after_sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM user_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before_sessions, after_sessions);
    let before = snapshot(&pool).await;
    let (status, body) = send(
        app,
        Some(&token),
        CONFIRM,
        &json!({"token":challenge}).to_string(),
        1,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_challenge");
    assert_eq!(snapshot(&pool).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn malformed_wrong_account_and_revoked_tokens_never_mutate() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool).await;
    let (other, other_token) = seed(&pool).await;
    let (mail, mut rx) = mail(false);
    let app = router(Some(pool.clone()), Some(mail));
    assert_eq!(
        send(app.clone(), Some(&token), REQUEST, "{}", 1).await.0,
        StatusCode::ACCEPTED
    );
    let (_, challenge, _) = receive(&mut rx).await;
    for (bearer, payload) in [
        (&other_token, json!({"token":challenge})),
        (
            &token,
            json!({"token":"pr1_0000000000000000000000000000000000000000000000000000000000000000"}),
        ),
        (
            &token,
            json!({"token":"ev1_0000000000000000000000000000000000000000000000000000000000000000"}),
        ),
    ] {
        let before = snapshot(&pool).await;
        let (status, body) =
            send(app.clone(), Some(bearer), CONFIRM, &payload.to_string(), 1).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "invalid_challenge");
        assert_eq!(snapshot(&pool).await, before);
    }
    for (path, payload) in [
        (REQUEST, "{\"user_id\":\"spoofed\"}"),
        (CONFIRM, "{\"token\":\"x\",\"token\":\"y\"}"),
        (REQUEST, "null"),
        (CONFIRM, "{}"),
        (REQUEST, "[]"),
    ] {
        let before = snapshot(&pool).await;
        assert_eq!(
            send(app.clone(), Some(&token), path, payload, 2).await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let before = snapshot(&pool).await;
    assert_eq!(
        send(
            app.clone(),
            Some(&token),
            &format!("{CONFIRM}?token=bad"),
            "{}",
            2
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE user_id=$1::uuid")
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    assert_eq!(
        send(
            app,
            Some(&other_token),
            CONFIRM,
            &json!({"token":challenge}).to_string(),
            2
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(snapshot(&pool).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn provider_failure_and_suppression_keep_generic_acceptance_without_verification() {
    let pool = common::pool().await;
    let (user, token) = seed(&pool).await;
    let (mail, mut rx) = mail(true);
    let app = router(Some(pool.clone()), Some(mail));
    let first = send(app.clone(), Some(&token), REQUEST, "{}", 1).await;
    assert_eq!(first.0, StatusCode::ACCEPTED);
    let _ = receive(&mut rx).await;
    let second = send(app.clone(), Some(&token), REQUEST, "{}", 1).await;
    assert_eq!(first, second);
    assert!(!verified(&pool, &user).await);
    assert!(
        tokio::time::timeout(Duration::from_millis(150), rx.recv())
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM email_verification_challenges WHERE user_id=$1::uuid",
    )
    .bind(&user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    sqlx::query("UPDATE users SET email_verified_at=clock_timestamp() WHERE id=$1::uuid")
        .bind(&user)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        send(app.clone(), Some(&token), REQUEST, "{}", 1).await,
        first
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(150), rx.recv())
            .await
            .is_err()
    );
    let disabled = router(Some(pool.clone()), None);
    let (status, body) = send(disabled, Some(&token), REQUEST, "{}", 1).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "email_verification_unavailable");
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn verification_routes_have_separate_bounded_ip_budgets() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool).await;
    let (mail, _rx) = mail(false);
    let app = router(Some(pool), Some(mail));
    for _ in 0..5 {
        assert_eq!(
            send(app.clone(), Some(&token), REQUEST, "{}", 1).await.0,
            StatusCode::ACCEPTED
        );
    }
    assert_eq!(
        send(app.clone(), Some(&token), REQUEST, "{}", 1).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    for _ in 0..20 {
        assert_eq!(
            send(app.clone(), Some(&token), CONFIRM, "{\"token\":\"bad\"}", 1)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        send(app, Some(&token), CONFIRM, "{\"token\":\"bad\"}", 1)
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn queue_capacity_is_bounded_and_full_queue_returns_safe_unavailable() {
    let pool = common::pool().await;
    let (mut mail, mut rx) = mail(false);
    let hold = Arc::new(Notify::new());
    mail.hold = Some(hold.clone());
    let app = router(Some(pool.clone()), Some(mail));
    for client in 1..=2 {
        let (_, token) = seed(&pool).await;
        assert_eq!(
            send(app.clone(), Some(&token), REQUEST, "{}", client)
                .await
                .0,
            StatusCode::ACCEPTED
        );
        let _ = receive(&mut rx).await;
    }
    for client in 3..=102 {
        let (_, token) = seed(&pool).await;
        assert_eq!(
            send(app.clone(), Some(&token), REQUEST, "{}", client)
                .await
                .0,
            StatusCode::ACCEPTED
        );
    }
    let (_, token) = seed(&pool).await;
    let before = snapshot(&pool).await;
    let (status, body) = send(app, Some(&token), REQUEST, "{}", 103).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "email_verification_unavailable");
    assert_eq!(snapshot(&pool).await, before);
    hold.notify_waiters();
}
