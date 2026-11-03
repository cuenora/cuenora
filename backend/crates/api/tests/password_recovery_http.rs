//! Recovery public interface against fake mail and isolated PostgreSQL only.
mod common;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use cuenora_api::{
    auth_throttling::ClientIpPolicy, password_recovery::RecoveryTransport,
    registration_http::app_with_mail, session_tokens::TokenLifetimes,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{Notify, mpsc};
use tower::ServiceExt;
const REQUEST: &str = "/v1/auth/password-recovery/request";
const CONFIRM: &str = "/v1/auth/password-recovery/confirm";
const OLD: &str = "Original-test!password-2048";
const NEW: &str = "Replacement-test!password-4096";
struct Mail {
    tx: mpsc::UnboundedSender<(String, String)>,
    hold: Option<Arc<Notify>>,
}
impl RecoveryTransport for Mail {
    fn send<'a>(
        &'a self,
        email: &'a str,
        token: &'a str,
        _key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async move {
            let _ = self.tx.send((email.into(), token.into()));
            if let Some(hold) = &self.hold {
                hold.notified().await;
            }
            Ok(())
        })
    }
    fn notify_reset<'a>(
        &'a self,
        _email: &'a str,
        _key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}
fn router(pool: Option<PgPool>, mail: Option<Mail>) -> Router {
    app_with_mail(
        pool,
        ClientIpPolicy::default(),
        TokenLifetimes::default(),
        None,
        mail.map(|m| Arc::new(m) as Arc<dyn RecoveryTransport>),
    )
}
async fn send(app: Router, path: &str, body: &str, client: u16) -> (StatusCode, Value) {
    let r = app
        .layer(Extension(ConnectInfo(
            format!("192.0.{}.{}:1", client / 255, client % 255)
                .parse::<std::net::SocketAddr>()
                .unwrap(),
        )))
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    if path.starts_with("/v1/auth/password-recovery/") {
        assert_eq!(r.headers()["cache-control"], "no-store");
    }
    assert!(r.headers().contains_key("x-request-id"));
    let bytes = to_bytes(r.into_body(), 16384).await.unwrap();
    let v = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, v)
}
async fn seed(app: &Router, email: &str) -> Value {
    assert_eq!(
        send(
            app.clone(),
            "/v1/auth/register",
            &json!({"email":email,"password":OLD}).to_string(),
            400
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let (s, pair) = send(
        app.clone(),
        "/v1/auth/login",
        &json!({"email":email,"password":OLD}).to_string(),
        401,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    pair
}
#[tokio::test]
async fn strict_input_and_disabled_delivery_use_safe_errors() {
    let app = router(None, None);
    for (i, b) in [
        "null",
        "[]",
        "[\"x@example.test\"]",
        "{}",
        "{\"email\":\"x@example.test\",\"email\":\"x@example.test\"}",
        "{\"email\":\"invalid\"}",
        "{\"email\":\"x@example.test\",\"redirect\":\"https://bad.test\"}",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            send(app.clone(), REQUEST, b, i as u16 + 1).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let (s, v) = send(app.clone(), REQUEST, "{\"email\":\"x@example.test\"}", 20).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(v["code"], "password_recovery_unavailable");
    assert_eq!(
        send(
            app.clone(),
            CONFIRM,
            &json!({"token":"ev1_".to_owned()+&"a".repeat(64),"new_password":NEW}).to_string(),
            21
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            app.clone(),
            CONFIRM,
            &json!({"token":"pr1_".to_owned()+&"a".repeat(64),"new_password":"short"}).to_string(),
            22
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            app,
            &format!("{REQUEST}?email=secret"),
            "{\"email\":\"x@example.test\"}",
            23
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}
#[tokio::test]
async fn ip_throttle_is_purpose_separated_and_body_is_bounded() {
    let app = router(None, None);
    for _ in 0..5 {
        assert_eq!(
            send(app.clone(), REQUEST, "{}", 50).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        send(app.clone(), REQUEST, "{}", 50).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        send(app.clone(), CONFIRM, "{}", 50).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(app.clone(), "/v1/auth/email-verification/request", "{}", 50)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(app, REQUEST, &"x".repeat(8193), 51).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn known_unknown_canonical_and_cooldown_have_identical_acceptance() {
    let pool = common::pool().await;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let app = router(Some(pool.clone()), Some(Mail { tx, hold: None }));
    let email = "Recovery.Case+tag@EXAMPLE.test";
    seed(&app, email).await;
    let unchanged_credentials = common::snapshot(&pool).await;
    for (i, e) in [
        "missing@example.test",
        "recovery.case+tag@example.test",
        email,
    ]
    .iter()
    .enumerate()
    {
        let (s, v) = send(
            app.clone(),
            REQUEST,
            &json!({"email":e}).to_string(),
            100 + i as u16,
        )
        .await;
        assert_eq!(s, StatusCode::ACCEPTED);
        assert_eq!(v, json!({"status":"accepted"}));
    }
    let (recipient, token) = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(common::snapshot(&pool).await, unchanged_credentials);
    assert_eq!(recipient, "Recovery.Case+tag@example.test");
    assert!(token.starts_with("pr1_"));
    assert!(
        tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .is_err()
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM password_recovery_challenges WHERE user_id=(SELECT id FROM users WHERE email_key='recovery.case+tag@example.test')").fetch_one(&pool).await.unwrap(),1);
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn reset_revokes_old_access_and_refresh_and_requires_normal_new_login() {
    let pool = common::pool().await;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let app = router(Some(pool.clone()), Some(Mail { tx, hold: None }));
    let email = "reset-flow@example.test";
    let pair = seed(&app, email).await;
    assert_eq!(
        send(
            app.clone(),
            REQUEST,
            &json!({"email":email}).to_string(),
            110
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let (_, token) = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    let body = json!({"token":token,"new_password":NEW}).to_string();
    let (s, v) = send(app.clone(), CONFIRM, &body, 111).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert!(v.is_null());
    assert_eq!(
        send(app.clone(), CONFIRM, &body, 112).await.1["code"],
        "invalid_challenge"
    );
    assert_eq!(
        send(
            app.clone(),
            "/v1/auth/login",
            &json!({"email":email,"password":OLD}).to_string(),
            113
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            app.clone(),
            "/v1/auth/refresh",
            &json!({"refresh_token":pair["refresh_token"]}).to_string(),
            114
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .clone()
        .layer(Extension(ConnectInfo(
            "192.0.1.15:1".parse::<std::net::SocketAddr>().unwrap(),
        )))
        .oneshot(
            Request::post("/v1/auth/email-verification/request")
                .header(
                    "authorization",
                    format!("Bearer {}", pair["access_token"].as_str().unwrap()),
                )
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        send(
            app,
            "/v1/auth/login",
            &json!({"email":email,"password":NEW}).to_string(),
            116
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE email_verified_at IS NOT NULL"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn provider_wait_does_not_hold_http_or_database_locks() {
    let pool = common::pool().await;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let hold = Arc::new(Notify::new());
    let app = router(
        Some(pool.clone()),
        Some(Mail {
            tx,
            hold: Some(hold.clone()),
        }),
    );
    let email = "delayed-flow@example.test";
    seed(&app, email).await;
    assert_eq!(
        send(
            app.clone(),
            REQUEST,
            &json!({"email":email}).to_string(),
            130
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let (_, token) = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    let (s, _) = tokio::time::timeout(
        Duration::from_secs(5),
        send(
            app,
            CONFIRM,
            &json!({"token":token,"new_password":NEW}).to_string(),
            131,
        ),
    )
    .await
    .unwrap();
    assert_eq!(s, StatusCode::NO_CONTENT);
    hold.notify_waiters();
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn accepted_queue_is_bounded_before_identity_lookup() {
    let pool = common::pool().await;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let hold = Arc::new(Notify::new());
    let app = router(
        Some(pool),
        Some(Mail {
            tx,
            hold: Some(hold.clone()),
        }),
    );
    for (i, email) in ["queue-one@example.test", "queue-two@example.test"]
        .iter()
        .enumerate()
    {
        seed(&app, email).await;
        assert_eq!(
            send(
                app.clone(),
                REQUEST,
                &json!({"email":email}).to_string(),
                150 + i as u16
            )
            .await
            .0,
            StatusCode::ACCEPTED
        );
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
    }
    for i in 0..100 {
        assert_eq!(
            send(
                app.clone(),
                REQUEST,
                &json!({"email":format!("unknown-{i}@example.test")}).to_string(),
                200 + i
            )
            .await
            .0,
            StatusCode::ACCEPTED
        );
    }
    assert_eq!(
        send(app, REQUEST, "{\"email\":\"other@example.test\"}", 350)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    hold.notify_waiters();
}
