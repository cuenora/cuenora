//! Public refresh contract; DB cases run only through the disposable harness.
mod common;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode, header},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::time::Duration;
use tower::ServiceExt;

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

async fn post(router: Router, path: &str, body: String, media: Option<&str>) -> Reply {
    let mut request = Request::post(path);
    if let Some(media) = media {
        request = request.header(header::CONTENT_TYPE, media);
    }
    let response = router
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = String::from_utf8(
        to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    Reply {
        status,
        headers,
        body,
    }
}

async fn refresh(router: Router, secret: &str) -> Reply {
    post(
        router,
        "/auth/refresh",
        json!({"refresh_token": secret}).to_string(),
        Some("application/json"),
    )
    .await
}

fn error(reply: Reply, status: StatusCode, code: &str) {
    assert_eq!(reply.status, status);
    assert_eq!(reply.headers[header::CONTENT_TYPE], "application/json");
    assert_eq!(reply.headers[header::CACHE_CONTROL], "no-store");
    assert!(!reply.headers.contains_key(header::SET_COOKIE));
    assert_eq!(reply.body, json!({"error": code}).to_string());
}

fn success(reply: Reply) -> Value {
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers[header::CONTENT_TYPE], "application/json");
    assert_eq!(reply.headers[header::CACHE_CONTROL], "no-store");
    assert!(!reply.headers.contains_key(header::SET_COOKIE));
    let value: Value = serde_json::from_str(&reply.body).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 5);
    assert_eq!(value["token_type"], "Bearer");
    assert_eq!(value["expires_in"], 900);
    assert_eq!(value["refresh_expires_in"], 2_592_000);
    for (field, prefix) in [("access_token", "ca1_"), ("refresh_token", "cr1_")] {
        let secret = value[field].as_str().unwrap();
        assert!(secret.starts_with(prefix) && secret.len() == 68);
    }
    value
}

async fn login(pool: &PgPool, address: &str, verified: bool) -> Value {
    let router = app(Some(pool.clone()));
    let input =
        json!({"email":address,"password":"synthetic refresh http password 42"}).to_string();
    let reply = post(
        router.clone(),
        "/auth/register",
        input.clone(),
        Some("application/json"),
    )
    .await;
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    if verified {
        sqlx::query("UPDATE users SET email_verified_at=clock_timestamp() WHERE email_key=$1")
            .bind(address)
            .execute(pool)
            .await
            .unwrap();
    }
    success(post(router, "/auth/login", input, Some("application/json")).await)
}

fn digest(value: &Value, field: &str) -> Vec<u8> {
    Sha256::digest(value[field].as_str().unwrap().as_bytes()).to_vec()
}

async fn family(pool: &PgPool, value: &Value) -> String {
    sqlx::query_scalar("SELECT session_id::text FROM session_refresh_tokens WHERE token_hash=$1")
        .bind(digest(value, "refresh_token"))
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM users u), (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY user_id), '[]') FROM password_credentials c), (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s), (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM session_access_tokens a), (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM session_refresh_tokens r))::text)")
        .fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn malformed_json_authority_fields_media_and_size_are_bounded() {
    for input in [
        "{",
        "null",
        "{}",
        r#"{"refresh_token":3}"#,
        r#"{"refresh_token":"secret","user_id":"forged"}"#,
        r#"{"refresh_token":"a","refresh_token":"b"}"#,
    ] {
        error(
            post(
                app(None),
                "/auth/refresh",
                input.into(),
                Some("application/json"),
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
    }
    for media in [None, Some("text/plain")] {
        error(
            post(
                app(None),
                "/auth/refresh",
                r#"{"refresh_token":"secret"}"#.into(),
                media,
            )
            .await,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_request",
        );
    }
    error(
        refresh(app(None), &"x".repeat(8192)).await,
        StatusCode::PAYLOAD_TOO_LARGE,
        "invalid_request",
    );
}

#[tokio::test]
async fn absent_database_is_generic_and_liveness_is_unchanged() {
    let tokens = generate().unwrap();
    error(
        refresh(app(None), tokens.refresh.expose_secret()).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "refresh_unavailable",
    );
    let response = app(None)
        .oneshot(Request::get("/health/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/plain; charset=utf-8"
    );
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        b"OK"
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn pending_and_verified_users_refresh_old_families_with_only_digest_persistence() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    for (email, verified) in [
        ("pending-refresh-http@example.test", false),
        ("verified-refresh-http@example.test", true),
    ] {
        let original = login(&pool, email, verified).await;
        let id = family(&pool, &original).await;
        sqlx::query("UPDATE user_sessions SET created_at=clock_timestamp()-interval '90 days' WHERE id=$1::uuid")
            .bind(&id).execute(&pool).await.unwrap();
        let identity = common::snapshot(&pool).await;
        let next =
            success(refresh(router.clone(), original["refresh_token"].as_str().unwrap()).await);
        assert!(next["refresh_token"] != original["refresh_token"]);
        assert!(next["access_token"] != original["access_token"]);
        assert_eq!(family(&pool, &next).await, id);
        let row = sqlx::query("SELECT r.used_at=n.created_at AND r.replaced_by=n.id AND n.expires_at-n.created_at=interval '30 days' AND a.expires_at-a.created_at=interval '15 minutes' AND a.created_at=n.created_at AS valid, (SELECT count(*) FROM session_access_tokens WHERE token_hash=$4) AS original_access FROM session_refresh_tokens r JOIN session_refresh_tokens n ON n.id=r.replaced_by JOIN session_access_tokens a ON a.token_hash=$3 WHERE r.token_hash=$1 AND n.token_hash=$2")
            .bind(digest(&original,"refresh_token")).bind(digest(&next,"refresh_token")).bind(digest(&next,"access_token")).bind(digest(&original,"access_token"))
            .fetch_one(&pool).await.unwrap();
        assert!(row.get::<bool, _>("valid"));
        assert_eq!(row.get::<i64, _>("original_access"), 1);
        assert_eq!(common::snapshot(&pool).await, identity);
        let storage: String = sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(a)) FROM session_access_tokens a), (SELECT jsonb_agg(to_jsonb(r)) FROM session_refresh_tokens r))::text")
            .fetch_one(&pool).await.unwrap();
        for value in [&original, &next] {
            for key in ["access_token", "refresh_token"] {
                assert!(!storage.contains(value[key].as_str().unwrap()));
            }
        }
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn invalid_expired_revoked_and_replayed_tokens_have_generic_scoped_results() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    let original = login(&pool, "replay-refresh-http@example.test", false).await;
    let sibling = login(&pool, "replay-refresh-http@example.test", false).await;
    let other = login(&pool, "other-refresh-http@example.test", false).await;
    let expired = login(&pool, "expired-refresh-http@example.test", false).await;
    sqlx::query("UPDATE session_refresh_tokens SET created_at=clock_timestamp()-interval '31 days', expires_at=clock_timestamp() WHERE token_hash=$1")
        .bind(digest(&expired,"refresh_token")).execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    let unknown = generate().unwrap();
    for secret in [
        "",
        "cr1_invalid",
        "cr1_0123' OR 1=1",
        original["access_token"].as_str().unwrap(),
        unknown.refresh.expose_secret(),
        expired["refresh_token"].as_str().unwrap(),
    ] {
        error(
            refresh(router.clone(), secret).await,
            StatusCode::UNAUTHORIZED,
            "invalid_refresh_token",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let next = success(refresh(router.clone(), original["refresh_token"].as_str().unwrap()).await);
    error(
        refresh(router.clone(), original["refresh_token"].as_str().unwrap()).await,
        StatusCode::UNAUTHORIZED,
        "invalid_refresh_token",
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM user_sessions WHERE id=$1::uuid")
            .bind(family(&pool, &original).await)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(revoked);
    let revoked_state = snapshot(&pool).await;
    error(
        refresh(router.clone(), next["refresh_token"].as_str().unwrap()).await,
        StatusCode::UNAUTHORIZED,
        "invalid_refresh_token",
    );
    assert_eq!(snapshot(&pool).await, revoked_state);
    success(refresh(router.clone(), sibling["refresh_token"].as_str().unwrap()).await);
    success(refresh(router, other["refresh_token"].as_str().unwrap()).await);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn storage_failure_is_generic_atomic_and_retry_can_succeed() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    let original = login(&pool, "failure-refresh-http@example.test", false).await;
    let before = snapshot(&pool).await;
    sqlx::raw_sql("CREATE FUNCTION reject_http_refresh() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END $$; CREATE TRIGGER reject_http_refresh BEFORE INSERT ON session_access_tokens FOR EACH ROW EXECUTE FUNCTION reject_http_refresh();")
        .execute(&pool).await.unwrap();
    let reply = refresh(router.clone(), original["refresh_token"].as_str().unwrap()).await;
    let after = snapshot(&pool).await;
    sqlx::raw_sql("DROP TRIGGER reject_http_refresh ON session_access_tokens; DROP FUNCTION reject_http_refresh();")
        .execute(&pool).await.unwrap();
    error(
        reply,
        StatusCode::SERVICE_UNAVAILABLE,
        "refresh_unavailable",
    );
    assert_eq!(after, before);
    success(refresh(router.clone(), original["refresh_token"].as_str().unwrap()).await);
    pool.close().await;
    error(
        refresh(router, original["refresh_token"].as_str().unwrap()).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "refresh_unavailable",
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn concurrent_refresh_replay_and_saturation_do_not_take_password_admission() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    let original = login(&pool, "race-refresh-http@example.test", false).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM user_sessions WHERE id=$1::uuid FOR UPDATE")
        .bind(family(&pool, &original).await)
        .execute(&mut *gate)
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let router = router.clone();
        let secret = original["refresh_token"].as_str().unwrap().to_owned();
        tasks.push(tokio::spawn(async move { refresh(router, &secret).await }));
    }
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE '%refresh_family_lock%'")
                .fetch_one(&pool).await.unwrap();
            if waiting >= 2 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    let busy = tokio::time::timeout(
        Duration::from_secs(1),
        refresh(router.clone(), original["refresh_token"].as_str().unwrap()),
    )
    .await
    .unwrap();
    error(busy, StatusCode::SERVICE_UNAVAILABLE, "refresh_unavailable");
    let login_reply = post(
        router.clone(),
        "/auth/login",
        json!({"email":"race-refresh-http@example.test","password":"wrong exact password"})
            .to_string(),
        Some("application/json"),
    )
    .await;
    assert_eq!(login_reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(login_reply.body, r#"{"error":"invalid_credentials"}"#);
    gate.commit().await.unwrap();
    let a = tasks.remove(0).await.unwrap();
    let b = tasks.remove(0).await.unwrap();
    let (winner, loser) = if a.status == StatusCode::OK {
        (a, b)
    } else {
        (b, a)
    };
    let next = success(winner);
    error(loser, StatusCode::UNAUTHORIZED, "invalid_refresh_token");
    let before = snapshot(&pool).await;
    error(
        refresh(router, next["refresh_token"].as_str().unwrap()).await,
        StatusCode::UNAUTHORIZED,
        "invalid_refresh_token",
    );
    assert_eq!(snapshot(&pool).await, before);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn cancellation_releases_refresh_slots_and_preserves_password_routes() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    let original = login(&pool, "cancel-refresh-http@example.test", false).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM user_sessions WHERE id=$1::uuid FOR UPDATE")
        .bind(family(&pool, &original).await)
        .execute(&mut *gate)
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let router = router.clone();
        let secret = original["refresh_token"].as_str().unwrap().to_owned();
        tasks.push(tokio::spawn(async move { refresh(router, &secret).await }));
    }
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE '%refresh_family_lock%'")
                .fetch_one(&pool).await.unwrap();
            if waiting == 2 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    error(
        refresh(router.clone(), original["refresh_token"].as_str().unwrap()).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "refresh_unavailable",
    );
    let live = router
        .clone()
        .oneshot(Request::get("/health/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(live.status(), StatusCode::OK);
    let registered = post(router.clone(), "/auth/register", json!({"email":"cancel-admission-http@example.test","password":"synthetic separate admission password 42"}).to_string(), Some("application/json")).await;
    assert_eq!(registered.status, StatusCode::ACCEPTED);
    let before = snapshot(&pool).await;
    for task in tasks {
        task.abort();
        assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    }
    // HTTP admission recovers immediately; SQLx can finish a query before rollback.
    error(
        tokio::time::timeout(
            Duration::from_secs(1),
            refresh(router.clone(), "cr1_invalid"),
        )
        .await
        .unwrap(),
        StatusCode::UNAUTHORIZED,
        "invalid_refresh_token",
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE '%refresh_family_lock%'")
                .fetch_one(&pool).await.unwrap();
            if waiting == 0 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
    gate.commit().await.unwrap();
    success(
        tokio::time::timeout(
            Duration::from_secs(1),
            refresh(router.clone(), original["refresh_token"].as_str().unwrap()),
        )
        .await
        .unwrap(),
    );
    let second = login(&pool, "postcancel-refresh-http@example.test", false).await;
    success(refresh(router, second["refresh_token"].as_str().unwrap()).await);
    pool.close().await;
}
