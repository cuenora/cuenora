//! Public HTTP tests; ignored cases run only through disposable PostgreSQL.
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::registration_http::app;
use sqlx::Row;
use tower::ServiceExt;
mod common;

const PASSWORD: &str = "  violet meadows through winter 水 \0";

async fn request(
    router: Router,
    body: String,
    content_type: Option<&str>,
) -> (StatusCode, String, String) {
    let mut builder = Request::post("/auth/register");
    if let Some(value) = content_type {
        builder = builder.header(header::CONTENT_TYPE, value);
    }
    let response = router
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap())
        .unwrap_or("")
        .to_owned();
    let body = String::from_utf8(
        to_bytes(response.into_body(), 16384)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    (status, content_type, body)
}

fn input(email: &str, password: &str) -> String {
    serde_json::json!({"email":email,"password":password}).to_string()
}

fn accepted(response: &(StatusCode, String, String)) {
    assert_eq!(
        response,
        &(
            StatusCode::ACCEPTED,
            "application/json".into(),
            "{\"status\":\"accepted\"}".into()
        )
    );
}

#[tokio::test]
async fn rejects_invalid_json_policy_and_authority_fields_without_echoing_credentials() {
    for body in [
        "{".to_owned(), "null".to_owned(), "{}".to_owned(),
        "{\"email\":2,\"password\":\"sensitive\"}".to_owned(),
        input("bad@@example.test", PASSWORD), input("josé@example.test", PASSWORD),
        input("valid@example.test", "short"), input("valid@example.test", &"水".repeat(129)),
        input("valid@example.test", "PASSWORDPASSWORD"),
        serde_json::json!({"email":"valid@example.test","password":PASSWORD,"user_id":"forged"}).to_string(),
        serde_json::json!({"email":"valid@example.test","password":PASSWORD,"email_verified_at":"2026-10-26"}).to_string(),
        "{\"email\":\"a@example.test\",\"email\":\"b@example.test\",\"password\":\"sensitive\"}".to_owned(),
    ] {
        assert_eq!(request(app(None), body, Some("application/json")).await,
            (StatusCode::BAD_REQUEST, "application/json".into(), "{\"error\":\"invalid_request\"}".into()));
    }
    for content_type in [None, Some("text/plain")] {
        assert_eq!(
            request(
                app(None),
                input("valid@example.test", PASSWORD),
                content_type
            )
            .await
            .0,
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
    }
    let huge = input("valid@example.test", &"x".repeat(8192));
    assert_eq!(
        request(app(None), huge, Some("application/json")).await,
        (
            StatusCode::PAYLOAD_TOO_LARGE,
            "application/json".into(),
            "{\"error\":\"invalid_request\"}".into()
        )
    );
}

#[tokio::test]
async fn missing_database_is_safe_and_does_not_change_liveness() {
    assert_eq!(
        request(
            app(None),
            input("valid@example.test", PASSWORD),
            Some("application/json")
        )
        .await,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "application/json".into(),
            "{\"error\":\"registration_unavailable\"}".into()
        )
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
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn registers_pending_identity_and_preserves_duplicate_credentials() {
    let pool = common::pool().await;
    let statement_timeout: String = sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(&pool)
        .await
        .unwrap();
    let lock_timeout: String = sqlx::query_scalar("SHOW lock_timeout")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(statement_timeout, "5s");
    assert_eq!(lock_timeout, "2s");
    let router = app(Some(pool.clone()));
    let first = request(
        router.clone(),
        input("O'Connor+http@mañana.Example", PASSWORD),
        Some("application/json"),
    )
    .await;
    accepted(&first);
    let row = sqlx::query("SELECT id::text, email_original, email_address, password_hash, email_verified_at IS NULL AND profile_time_zone IS NULL AS pending FROM users JOIN password_credentials ON id = user_id WHERE email_key = 'o''connor+http@xn--maana-pta.example'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        row.get::<String, _>("email_original"),
        "O'Connor+http@mañana.Example"
    );
    assert_eq!(
        row.get::<String, _>("email_address"),
        "O'Connor+http@xn--maana-pta.example"
    );
    assert_eq!(row.get::<String, _>("id").len(), 36);
    assert!(row.get::<bool, _>("pending"));
    let hash: String = row.get("password_hash");
    Argon2::default()
        .verify_password(PASSWORD.as_bytes(), &PasswordHash::new(&hash).unwrap())
        .unwrap();
    assert!(
        Argon2::default()
            .verify_password(b"altered password", &PasswordHash::new(&hash).unwrap())
            .is_err()
    );
    sqlx::query("UPDATE users SET email_verified_at = '2026-10-26T12:00:00Z', profile_time_zone = 'Europe/Stockholm' WHERE id::text = $1")
        .bind(row.get::<String,_>("id")).execute(&pool).await.unwrap();
    let before = common::snapshot(&pool).await;
    let duplicate = request(
        router.clone(),
        input(
            "O'CONNOR+HTTP@xn--maana-pta.example",
            "different rivers beside violet mountains",
        ),
        Some("application/json"),
    )
    .await;
    assert_eq!(first, duplicate);
    assert_eq!(common::snapshot(&pool).await, before);
    accepted(
        &request(
            router,
            input("second-http@example.test", PASSWORD),
            Some("application/json"),
        )
        .await,
    );
    let second: String = sqlx::query_scalar("SELECT password_hash FROM password_credentials JOIN users ON user_id = id WHERE email_key = 'second-http@example.test'").fetch_one(&pool).await.unwrap();
    assert_ne!(hash, second);
    let side_effects: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM accounts) + (SELECT count(*) FROM transactions) + (SELECT count(*) FROM account_movements) + (SELECT count(*) FROM user_sessions) + (SELECT count(*) FROM session_refresh_tokens) + (SELECT count(*) FROM transaction_audit)").fetch_one(&pool).await.unwrap();
    assert_eq!(side_effects, 0);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn accepts_password_bounds_and_escaped_unicode_and_keeps_tags_distinct() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    for (email, password) in [
        ("edge.http@example.test", "水".repeat(15)),
        ("edgehttp@example.test", "水".repeat(128)),
        ("edgehttp+tag@example.test", PASSWORD.into()),
    ] {
        // Exercise JSON escape expansion, not only raw UTF-8 serialization.
        let body = input(email, &password).replace('水', "\\u6c34");
        accepted(
            &request(
                router.clone(),
                body,
                Some("application/json; charset=utf-8"),
            )
            .await,
        );
        let hash: String = sqlx::query_scalar("SELECT password_hash FROM password_credentials JOIN users ON id = user_id WHERE email_key = $1").bind(email).fetch_one(&pool).await.unwrap();
        Argon2::default()
            .verify_password(password.as_bytes(), &PasswordHash::new(&hash).unwrap())
            .unwrap();
    }
    let before = common::snapshot(&pool).await;
    assert_eq!(
        request(
            router,
            input("reject-http@example.test", "too short"),
            Some("application/json")
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(common::snapshot(&pool).await, before);
    for body in [
        "{".to_owned(),
        input("bad@@example.test", PASSWORD),
        input("reject-http@example.test", "PASSWORDPASSWORD"),
        serde_json::json!({"email":"reject-http@example.test","password":PASSWORD,"user_id":"forged"}).to_string(),
    ] {
        assert_eq!(request(app(Some(pool.clone())), body, Some("application/json")).await.0, StatusCode::BAD_REQUEST);
        assert_eq!(common::snapshot(&pool).await, before);
    }
    for (body, content_type, status) in [
        (
            input("reject-http@example.test", PASSWORD),
            "text/plain",
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            "x".repeat(8193),
            "application/json",
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        assert_eq!(
            request(app(Some(pool.clone())), body, Some(content_type))
                .await
                .0,
            status
        );
        assert_eq!(common::snapshot(&pool).await, before);
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn concurrent_equivalent_requests_never_replace_or_orphan_credentials() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    let a = request(
        router.clone(),
        input("Race-http@bücher.example", PASSWORD),
        Some("application/json"),
    );
    let b = request(
        router,
        input("RACE-HTTP@xn--bcher-kva.example", PASSWORD),
        Some("application/json"),
    );
    let (a, b) = tokio::join!(a, b);
    accepted(&a);
    assert_eq!(a, b);
    let pairs: i64 = sqlx::query_scalar("SELECT count(*) FROM users JOIN password_credentials ON id = user_id WHERE email_key = 'race-http@xn--bcher-kva.example'").fetch_one(&pool).await.unwrap();
    assert_eq!(pairs, 1);
    let orphans: i64 = sqlx::query_scalar("SELECT count(*) FROM users LEFT JOIN password_credentials ON id = user_id WHERE user_id IS NULL").fetch_one(&pool).await.unwrap();
    assert_eq!(orphans, 0);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn database_failure_is_generic_and_rolls_back_every_row() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    accepted(
        &request(
            router.clone(),
            input("control-http@example.test", PASSWORD),
            Some("application/json"),
        )
        .await,
    );
    let before = common::snapshot(&pool).await;
    sqlx::raw_sql("CREATE FUNCTION reject_http_credential() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic private database detail'; END $$; CREATE TRIGGER reject_http_credential BEFORE INSERT ON password_credentials FOR EACH ROW EXECUTE FUNCTION reject_http_credential();").execute(&pool).await.unwrap();
    let failed = request(
        router.clone(),
        input("rollback-http@example.test", PASSWORD),
        Some("application/json"),
    )
    .await;
    assert_eq!(
        failed,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "application/json".into(),
            "{\"error\":\"registration_unavailable\"}".into()
        )
    );
    assert_eq!(common::snapshot(&pool).await, before);
    sqlx::raw_sql("DROP TRIGGER reject_http_credential ON password_credentials; DROP FUNCTION reject_http_credential();").execute(&pool).await.unwrap();
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE users IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let blocked = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        request(
            router.clone(),
            input("locked-http@example.test", PASSWORD),
            Some("application/json"),
        ),
    );
    let live = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        router
            .clone()
            .oneshot(Request::get("/health/live").body(Body::empty()).unwrap()),
    );
    let (blocked, live) = tokio::join!(blocked, live);
    assert_eq!(blocked.unwrap(), failed);
    let live = live.unwrap().unwrap();
    assert_eq!(live.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(live.into_body(), 1024).await.unwrap().as_ref(),
        b"OK"
    );
    lock.rollback().await.unwrap();
    assert_eq!(common::snapshot(&pool).await, before);
    accepted(
        &request(
            router.clone(),
            input("recovered-http@example.test", PASSWORD),
            Some("application/json"),
        )
        .await,
    );
    let before = common::snapshot(&pool).await;
    pool.close().await;
    assert_eq!(
        request(
            router,
            input("unavailable-http@example.test", PASSWORD),
            Some("application/json")
        )
        .await,
        failed
    );
    let reopened = common::pool().await;
    assert_eq!(common::snapshot(&reopened).await, before);
    reopened.close().await;
}

#[test]
fn binary_rejects_invalid_configuration_without_disclosing_the_url() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_cuenora-api"))
        .env_clear()
        .env("CUENORA_DATABASE_URL", "synthetic-private-invalid-url")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Invalid CUENORA_DATABASE_URL"));
    assert!(!stderr.contains("synthetic-private"));
    assert!(output.stdout.is_empty());
}
