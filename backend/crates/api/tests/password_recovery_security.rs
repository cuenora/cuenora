//! Recovery security regressions; disposable PostgreSQL only.
use cuenora_api::{
    login_store::{issue_initial_session, lookup_credentials},
    password_recovery_store as recovery,
    registration_policy::{Email, hash_password},
    registration_store::persist_registration,
    session_store,
    session_tokens::{InitialTokens, generate},
};
use sqlx::PgPool;
use std::time::Duration;
async fn seed(pool: &PgPool, name: &str) -> (String, Email, InitialTokens, String) {
    let email = Email::parse(&format!("{name}@example.test")).unwrap();
    let hash = hash_password("independent synthetic old password 42").unwrap();
    persist_registration(pool, &email, &hash).await.unwrap();
    let c = lookup_credentials(pool, &email).await.unwrap().unwrap();
    let t = generate().unwrap();
    assert!(issue_initial_session(pool, &c, &t).await.unwrap());
    let id =
        sqlx::query_scalar("SELECT user_id::text FROM session_access_tokens WHERE token_hash=$1")
            .bind(t.access.digest().as_slice())
            .fetch_one(pool)
            .await
            .unwrap();
    let new = hash_password("independent synthetic new password 42").unwrap();
    (id, email, t, new)
}
async fn snap(p: &PgPool, id: &str) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT to_jsonb(u) FROM users u WHERE id=$1::uuid),(SELECT to_jsonb(c) FROM password_credentials c WHERE user_id=$1::uuid),(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s WHERE user_id=$1::uuid),(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM session_access_tokens a WHERE user_id=$1::uuid),(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM session_refresh_tokens r WHERE user_id=$1::uuid),(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM password_recovery_challenges c WHERE user_id=$1::uuid),(SELECT jsonb_agg(to_jsonb(n) ORDER BY id) FROM password_reset_notifications n WHERE user_id=$1::uuid))::text)").bind(id).fetch_one(p).await.unwrap()
}
async fn waiting(p: &PgPool, pattern: &str) {
    tokio::time::timeout(Duration::from_secs(5),async{loop{let n:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE $1").bind(pattern).fetch_one(p).await.unwrap(); if n>0{break} tokio::time::sleep(Duration::from_millis(10)).await;}}).await.expect("expected actual row lock wait");
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn eligibility_and_overflow_are_nonmutating() {
    let p = pool().await;
    for (name, change) in [
        (
            "future",
            "created_at=clock_timestamp()+interval '1 minute',expires_at=clock_timestamp()+interval '31 minutes'",
        ),
        (
            "expired",
            "created_at=clock_timestamp()-interval '31 minutes',expires_at=clock_timestamp()",
        ),
        ("revision", "credential_revision=credential_revision+1"),
        ("email", "email='moved@example.test'"),
    ] {
        let (id, e, _, h) = seed(&p, name).await;
        let c = recovery::issue(&p, &e).await.unwrap().unwrap();
        sqlx::query(&format!(
            "UPDATE password_recovery_challenges SET {change} WHERE user_id=$1::uuid"
        ))
        .bind(&id)
        .execute(&p)
        .await
        .unwrap();
        let before = snap(&p, &id).await;
        assert_eq!(
            recovery::confirm(&p, &c.token, &h).await,
            Err(recovery::StoreError::InvalidChallenge),
            "{name}"
        );
        assert_eq!(snap(&p, &id).await, before, "{name}");
    }
    let (id, e, _, h) = seed(&p, "overflow").await;
    sqlx::query("UPDATE password_credentials SET credential_revision=9223372036854775807 WHERE user_id=$1::uuid").bind(&id).execute(&p).await.unwrap();
    let c = recovery::issue(&p, &e).await.unwrap().unwrap();
    let before = snap(&p, &id).await;
    assert_eq!(
        recovery::confirm(&p, &c.token, &h).await,
        Err(recovery::StoreError::Unavailable)
    );
    assert_eq!(snap(&p, &id).await, before);
    p.close().await;
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn credential_waiter_rechecks_expiry() {
    let p = pool().await;
    let (id, e, _, h) = seed(&p, "waitexpiry").await;
    let c = recovery::issue(&p, &e).await.unwrap().unwrap();
    sqlx::query("UPDATE password_recovery_challenges SET expires_at=clock_timestamp()+interval '1500 milliseconds' WHERE user_id=$1::uuid").bind(&id).execute(&p).await.unwrap();
    let before = snap(&p, &id).await;
    let mut b = p.begin().await.unwrap();
    sqlx::query("SELECT user_id FROM password_credentials WHERE user_id=$1::uuid FOR UPDATE")
        .bind(&id)
        .fetch_one(&mut *b)
        .await
        .unwrap();
    let q = p.clone();
    let w = tokio::spawn(async move { recovery::confirm(&q, &c.token, &h).await });
    waiting(&p, "%FROM password_credentials c JOIN users u%").await;
    tokio::time::sleep(Duration::from_millis(1700)).await;
    b.commit().await.unwrap();
    assert_eq!(
        w.await.unwrap(),
        Err(recovery::StoreError::InvalidChallenge)
    );
    assert_eq!(snap(&p, &id).await, before);
    p.close().await;
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn reset_before_login_and_refresh_rejects_waiters() {
    let p = pool().await;
    let (id, e, t, h) = seed(&p, "resetfirst").await;
    let c = recovery::issue(&p, &e).await.unwrap().unwrap();
    let old = lookup_credentials(&p, &e).await.unwrap().unwrap();
    let sid: String =
        sqlx::query_scalar("SELECT id::text FROM user_sessions WHERE user_id=$1::uuid")
            .bind(&id)
            .fetch_one(&p)
            .await
            .unwrap();
    let mut b = p.begin().await.unwrap();
    sqlx::query("SELECT id FROM user_sessions WHERE id=$1::uuid FOR UPDATE")
        .bind(&sid)
        .fetch_one(&mut *b)
        .await
        .unwrap();
    let q = p.clone();
    let r = tokio::spawn(async move { recovery::confirm(&q, &c.token, &h).await });
    waiting(&p, "%SELECT id FROM user_sessions WHERE user_id=%").await;
    let q = p.clone();
    let login =
        tokio::spawn(async move { issue_initial_session(&q, &old, &generate().unwrap()).await });
    waiting(&p, "%WITH current_credential AS MATERIALIZED%").await;
    let q = p.clone();
    let refresh =
        tokio::spawn(async move { session_store::rotate(&q, t.refresh.expose_secret()).await });
    waiting(&p, "%refresh_family_lock%").await;
    b.commit().await.unwrap();
    r.await.unwrap().unwrap();
    assert!(!login.await.unwrap().unwrap());
    assert!(refresh.await.unwrap().unwrap().is_none());
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM user_sessions WHERE user_id=$1::uuid AND revoked_at IS NULL",
    )
    .bind(&id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(active, 0);
    let fresh = lookup_credentials(&p, &e).await.unwrap().unwrap();
    assert!(
        issue_initial_session(&p, &fresh, &generate().unwrap())
            .await
            .unwrap()
    );
    p.close().await;
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn login_and_refresh_before_reset_are_revoked() {
    let p = pool().await;
    let (id, e, t, h) = seed(&p, "loginfirst").await;
    let c = recovery::issue(&p, &e).await.unwrap().unwrap();
    let old = lookup_credentials(&p, &e).await.unwrap().unwrap();
    let mut b = p.begin().await.unwrap();
    sqlx::query("SELECT user_id FROM password_credentials WHERE user_id=$1::uuid FOR UPDATE")
        .bind(&id)
        .fetch_one(&mut *b)
        .await
        .unwrap();
    let q = p.clone();
    let login =
        tokio::spawn(async move { issue_initial_session(&q, &old, &generate().unwrap()).await });
    waiting(&p, "%WITH current_credential AS MATERIALIZED%").await;
    let q = p.clone();
    let reset = tokio::spawn(async move { recovery::confirm(&q, &c.token, &h).await });
    waiting(&p, "%FROM password_credentials c JOIN users u%").await;
    b.commit().await.unwrap();
    assert!(login.await.unwrap().unwrap());
    reset.await.unwrap().unwrap();
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM user_sessions WHERE user_id=$1::uuid AND revoked_at IS NULL",
    )
    .bind(&id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(active, 0);
    assert!(
        session_store::rotate(&p, t.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    let (id, e, t, h) = seed(&p, "refreshfirst").await;
    let c = recovery::issue(&p, &e).await.unwrap().unwrap();
    let mut b = p.begin().await.unwrap();
    sqlx::query("SELECT id FROM user_sessions WHERE user_id=$1::uuid FOR UPDATE")
        .bind(&id)
        .fetch_one(&mut *b)
        .await
        .unwrap();
    let q = p.clone();
    let refresh =
        tokio::spawn(async move { session_store::rotate(&q, t.refresh.expose_secret()).await });
    waiting(&p, "%refresh_family_lock%").await;
    let q = p.clone();
    let reset = tokio::spawn(async move { recovery::confirm(&q, &c.token, &h).await });
    waiting(&p, "%SELECT id FROM user_sessions WHERE user_id=%").await;
    b.commit().await.unwrap();
    let tokens = refresh.await.unwrap().unwrap().unwrap();
    reset.await.unwrap().unwrap();
    let before = snap(&p, &id).await;
    assert!(
        session_store::rotate(&p, tokens.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snap(&p, &id).await, before);
    p.close().await;
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn exhausted_notification_is_durable_and_never_reclaimed() {
    let p = pool().await;
    let (id, e, _, h) = seed(&p, "notifyexhaust").await;
    let c = recovery::issue(&p, &e).await.unwrap().unwrap();
    recovery::confirm(&p, &c.token, &h).await.unwrap();
    sqlx::query("UPDATE password_reset_notifications SET delivered_at=clock_timestamp() WHERE user_id<>$1::uuid AND delivered_at IS NULL").bind(&id).execute(&p).await.unwrap();
    for attempt in 1..=5 {
        let n = recovery::claim_notification(&p).await.unwrap().unwrap();
        assert_eq!(n.attempt, attempt);
        assert!(!n.idempotency_key.contains(&c.token));
        recovery::finish_notification(&p, &n.id, n.attempt, false)
            .await
            .unwrap();
        sqlx::query("UPDATE password_reset_notifications SET available_at=clock_timestamp() WHERE id=$1::uuid").bind(&n.id).execute(&p).await.unwrap();
    }
    let before = snap(&p, &id).await;
    assert!(recovery::claim_notification(&p).await.unwrap().is_none());
    assert_eq!(snap(&p, &id).await, before);
    let failed: bool = sqlx::query_scalar(
        "SELECT failed_at IS NOT NULL FROM password_reset_notifications WHERE user_id=$1::uuid",
    )
    .bind(&id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert!(failed);
    p.close().await;
}
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use cuenora_api::{
    auth_throttling::ClientIpPolicy, email_verification::VerificationTransport, registration_http,
    session_tokens::TokenLifetimes,
};
use std::{future::Future, net::SocketAddr, pin::Pin, sync::Arc};
use tower::ServiceExt;
struct Capture(tokio::sync::mpsc::UnboundedSender<String>);
impl VerificationTransport for Capture {
    fn send<'a>(
        &'a self,
        _: &'a str,
        t: &'a str,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async move {
            self.0.send(t.to_owned()).unwrap();
            Ok(())
        })
    }
}
async fn post(
    app: axum::Router,
    path: &str,
    body: serde_json::Value,
    access: &str,
) -> (StatusCode, serde_json::Value) {
    let mut req = Request::post(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {access}"))
        .body(Body::from(body.to_string()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo("127.0.0.1:2000".parse::<SocketAddr>().unwrap()));
    let r = app.oneshot(req).await.unwrap();
    let s = r.status();
    let b = to_bytes(r.into_body(), 8192).await.unwrap();
    (
        s,
        if b.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&b).unwrap()
        },
    )
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn old_verification_revision_and_invalid_password_preserve_state() {
    let p = pool().await;
    let (id, e, t, h) = seed(&p, "verifyrevision").await;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let app = registration_http::app_with_verification(
        Some(p.clone()),
        ClientIpPolicy::default(),
        TokenLifetimes::default(),
        Some(Arc::new(Capture(tx))),
    );
    assert_eq!(
        post(
            app.clone(),
            "/v1/auth/email-verification/request",
            serde_json::json!({}),
            t.access.expose_secret()
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let ev = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    let c = recovery::issue(&p, &e).await.unwrap().unwrap();
    let before = snap(&p, &id).await;
    assert_eq!(
        post(
            app.clone(),
            "/v1/auth/password-recovery/confirm",
            serde_json::json!({"token":c.token,"new_password":"short"}),
            ""
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(snap(&p, &id).await, before);
    recovery::confirm(&p, &c.token, &h).await.unwrap();
    let credentials = lookup_credentials(&p, &e).await.unwrap().unwrap();
    let fresh = generate().unwrap();
    assert!(
        issue_initial_session(&p, &credentials, &fresh)
            .await
            .unwrap()
    );
    let before = snap(&p, &id).await;
    let ev_before:String=sqlx::query_scalar("SELECT md5(jsonb_agg(to_jsonb(c) ORDER BY id)::text) FROM email_verification_challenges c WHERE user_id=$1::uuid").bind(&id).fetch_one(&p).await.unwrap();
    let (s, v) = post(
        app,
        "/v1/auth/email-verification/confirm",
        serde_json::json!({"token":ev}),
        fresh.access.expose_secret(),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "invalid_challenge");
    assert_eq!(snap(&p, &id).await, before);
    let ev_after:String=sqlx::query_scalar("SELECT md5(jsonb_agg(to_jsonb(c) ORDER BY id)::text) FROM email_verification_challenges c WHERE user_id=$1::uuid").bind(&id).fetch_one(&p).await.unwrap();
    assert_eq!(ev_before, ev_after);
    p.close().await;
}

// Blocked login/reset/refresh operations and the lock observer need spare
// connections; production admission and connection limits are unchanged.
async fn pool() -> PgPool {
    assert_eq!(std::env::var("CUENORA_TEST_DISPOSABLE").as_deref(), Ok("1"));
    let port: u16 = std::env::var("CUENORA_TEST_PGPORT")
        .unwrap()
        .parse()
        .unwrap();
    let password = std::env::var("CUENORA_TEST_PGPASSWORD").unwrap();
    // The Compose harness generates token_urlsafe credentials, not arbitrary URLs.
    assert!(
        password
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    );
    let url = format!("postgres://cuenora:{password}@127.0.0.1:{port}/cuenora");
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(16)
        .connect(&url)
        .await
        .unwrap()
}
