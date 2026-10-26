//! Public login contract tests; ignored database cases use disposable PostgreSQL.
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::{
    registration_http::app,
    registration_policy::{Email, hash_password},
    registration_store::persist_registration,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::time::Duration;
use tower::ServiceExt;
mod common;

const PASSWORD: &str = "  violet meadows beside winter 水\0";

async fn request(
    router: Router,
    path: &str,
    body: String,
) -> (StatusCode, String, Vec<(String, String)>, String) {
    let response = router
        .oneshot(
            Request::post(path)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect();
    let body = String::from_utf8(
        to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    (status, content_type, headers, body)
}

fn input(email: &str, password: &str) -> String {
    serde_json::json!({"email": email, "password": password}).to_string()
}

async fn seed(pool: &PgPool, address: &str, password: &str, verified: bool) {
    let email = Email::parse(address).unwrap();
    let hash = hash_password(password).unwrap();
    persist_registration(pool, &email, &hash).await.unwrap();
    if verified {
        sqlx::query("UPDATE users SET email_verified_at=statement_timestamp(), profile_time_zone='Europe/Stockholm' WHERE email_key=lower($1 COLLATE \"C\")")
            .bind(email.address()).execute(pool).await.unwrap();
    }
}

async fn full_snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM users u), (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY user_id), '[]') FROM password_credentials c), (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s), (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM session_access_tokens a), (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM session_refresh_tokens r))::text)")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn wait_for_database_locks(pool: &PgPool, expected: i64) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query NOT LIKE '%pg_stat_activity%'")
                .fetch_one(pool).await.unwrap();
            if waiting >= expected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("database lock barrier was not observed");
}

async fn family_counts(pool: &PgPool) -> (i64, i64, i64) {
    let row = sqlx::query("SELECT (SELECT count(*) FROM user_sessions) AS sessions, (SELECT count(*) FROM session_access_tokens) AS access, (SELECT count(*) FROM session_refresh_tokens) AS refresh")
        .fetch_one(pool).await.unwrap();
    (row.get("sessions"), row.get("access"), row.get("refresh"))
}

fn digest(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn wrong_unknown_missing_and_overlong_credentials_are_safe_and_non_mutating() {
    let pool = common::pool().await;
    seed(&pool, "known-login@example.test", PASSWORD, false).await;
    let _identity_snapshot = common::snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let before = full_snapshot(&pool).await;
    let wrong = request(
        router.clone(),
        "/auth/login",
        input("known-login@example.test", "different exact password"),
    )
    .await;
    let missing = request(
        router.clone(),
        "/auth/login",
        input("unknown-login@example.test", PASSWORD),
    )
    .await;
    let no_credential = Email::parse("no-credential@example.test").unwrap();
    sqlx::query("INSERT INTO users(email_original,email_address) VALUES($1,$1)")
        .bind(no_credential.address())
        .execute(&pool)
        .await
        .unwrap();
    let before_no_credential = full_snapshot(&pool).await;
    let no_credential = request(
        router.clone(),
        "/auth/login",
        input("no-credential@example.test", PASSWORD),
    )
    .await;
    assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong.1, "application/json");
    assert_eq!(wrong.3, "{\"error\":\"invalid_credentials\"}");
    assert!(
        !wrong
            .2
            .iter()
            .any(|(key, _)| key == "set-cookie" || key == "cache-control")
    );
    assert_eq!(missing, wrong);
    assert_eq!(no_credential, wrong);
    assert_eq!(full_snapshot(&pool).await, before_no_credential);
    assert_ne!(before, before_no_credential);

    for oversized in ["水".repeat(129), "x".repeat(513)] {
        assert_eq!(
            request(
                router.clone(),
                "/auth/login",
                input("known-login@example.test", &oversized),
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(full_snapshot(&pool).await, before_no_credential);
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn pending_verified_and_legacy_credentials_receive_fresh_digest_only_sessions() {
    let pool = common::pool().await;
    let sessions_before: i64 = sqlx::query_scalar("SELECT count(*) FROM user_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    seed(&pool, "pending-login@example.test", PASSWORD, false).await;
    seed(&pool, "verified-login@example.test", PASSWORD, true).await;
    let legacy = Email::parse("legacy-login@example.test").unwrap();
    let salt = SaltString::encode_b64(b"synthetic-login-salt").unwrap();
    let short_legacy_hash = Argon2::default()
        .hash_password(b"old", &salt)
        .unwrap()
        .to_string();
    persist_registration(&pool, &legacy, &short_legacy_hash)
        .await
        .unwrap();
    let router = app(Some(pool.clone()));
    let profile_before: Value = sqlx::query_scalar("SELECT jsonb_build_array(jsonb_agg(jsonb_build_array(email_key,email_verified_at,profile_time_zone) ORDER BY email_key), jsonb_agg(to_jsonb(c) ORDER BY user_id)) FROM users u JOIN password_credentials c ON c.user_id=u.id")
        .fetch_one(&pool).await.unwrap();
    let mut issued_access = Vec::new();
    let mut issued_refresh = Vec::new();
    for (email, password, verified) in [
        ("pending-login@example.test", PASSWORD, false),
        ("verified-login@example.test", PASSWORD, true),
        ("legacy-login@example.test", "old", false),
    ] {
        let (status, content_type, headers, body) =
            request(router.clone(), "/auth/login", input(email, password)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type, "application/json");
        assert!(
            headers
                .iter()
                .any(|(key, value)| key == "cache-control" && value == "no-store")
        );
        assert!(!headers.iter().any(|(key, _)| key == "set-cookie"));
        let payload: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(payload.as_object().unwrap().len(), 5);
        assert_eq!(payload["token_type"], "Bearer");
        assert_eq!(payload["expires_in"], 900);
        assert_eq!(payload["refresh_expires_in"], 2_592_000);
        let access = payload["access_token"].as_str().unwrap();
        let refresh = payload["refresh_token"].as_str().unwrap();
        issued_access.push(access.to_owned());
        issued_refresh.push(refresh.to_owned());
        assert!(access.starts_with("ca1_"));
        assert!(refresh.starts_with("cr1_"));
        assert_ne!(access, refresh);
        let row = sqlx::query("SELECT u.email_verified_at IS NOT NULL AS verified, u.profile_time_zone, a.token_hash AS access_hash, r.token_hash AS refresh_hash, extract(epoch FROM (a.expires_at-a.created_at))::bigint AS access_seconds, extract(epoch FROM (r.expires_at-r.created_at))::bigint AS refresh_seconds, a.created_at=s.created_at AS access_start, r.created_at=s.created_at AS refresh_start FROM users u JOIN user_sessions s ON s.user_id=u.id JOIN session_access_tokens a ON a.session_id=s.id JOIN session_refresh_tokens r ON r.session_id=s.id WHERE u.email_key=lower($1 COLLATE \"C\") ORDER BY s.created_at DESC LIMIT 1")
            .bind(email).fetch_one(&pool).await.unwrap();
        assert_eq!(row.get::<bool, _>("verified"), verified);
        assert_eq!(row.get::<Vec<u8>, _>("access_hash"), digest(access));
        assert_eq!(row.get::<Vec<u8>, _>("refresh_hash"), digest(refresh));
        assert_eq!(row.get::<i64, _>("access_seconds"), 900);
        assert_eq!(row.get::<i64, _>("refresh_seconds"), 2_592_000);
        assert!(row.get::<bool, _>("access_start"));
        assert!(row.get::<bool, _>("refresh_start"));
    }
    let repeated: Value = serde_json::from_str(
        &request(
            router,
            "/auth/login",
            input("pending-login@example.test", PASSWORD),
        )
        .await
        .3,
    )
    .unwrap();
    assert_ne!(repeated["access_token"], issued_access[0]);
    assert_ne!(repeated["refresh_token"], issued_refresh[0]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM user_sessions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        sessions_before + 4
    );
    let profile_after: Value = sqlx::query_scalar("SELECT jsonb_build_array(jsonb_agg(jsonb_build_array(email_key,email_verified_at,profile_time_zone) ORDER BY email_key), jsonb_agg(to_jsonb(c) ORDER BY user_id)) FROM users u JOIN password_credentials c ON c.user_id=u.id")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(profile_after, profile_before);
    let financial_rows: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM accounts)+(SELECT count(*) FROM transactions)+(SELECT count(*) FROM account_movements)")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(financial_rows, 0);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn revision_race_and_mixed_requests_share_capacity_without_issuing_stale_tokens() {
    let pool = common::pool().await;
    seed(&pool, "race-http@example.test", PASSWORD, false).await;
    let router = app(Some(pool.clone()));
    let before = family_counts(&pool).await;
    let mut revision = pool.begin().await.unwrap();
    sqlx::query("UPDATE password_credentials SET password_changed_at=password_changed_at+interval '1 microsecond' WHERE user_id=(SELECT id FROM users WHERE email_key='race-http@example.test')")
        .execute(&mut *revision).await.unwrap();
    let task_router = router.clone();
    let stale = tokio::spawn(async move {
        request(
            task_router,
            "/auth/login",
            input("race-http@example.test", PASSWORD),
        )
        .await
    });
    wait_for_database_locks(&pool, 1).await;
    revision.commit().await.unwrap();
    let stale = stale.await.unwrap();
    assert_eq!(stale.0, StatusCode::UNAUTHORIZED);
    assert_eq!(stale.3, "{\"error\":\"invalid_credentials\"}");
    assert_eq!(family_counts(&pool).await, before);

    // Hold actual registration and login persistence, not synthetic permits.
    sqlx::raw_sql("CREATE FUNCTION auth_capacity_barrier() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(753159); RETURN NEW; END $$; CREATE TRIGGER auth_register_barrier BEFORE INSERT ON users FOR EACH ROW EXECUTE FUNCTION auth_capacity_barrier(); CREATE TRIGGER auth_login_barrier BEFORE INSERT ON user_sessions FOR EACH ROW EXECUTE FUNCTION auth_capacity_barrier();")
        .execute(&pool).await.unwrap();
    let mut barrier = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(753159)")
        .execute(&mut *barrier)
        .await
        .unwrap();
    let task_router = router.clone();
    let registration = tokio::spawn(async move {
        request(
            task_router,
            "/auth/register",
            input("capacity-created@example.test", PASSWORD),
        )
        .await
    });
    let task_router = router.clone();
    let login = tokio::spawn(async move {
        request(
            task_router,
            "/auth/login",
            input("race-http@example.test", PASSWORD),
        )
        .await
    });
    wait_for_database_locks(&pool, 2).await;
    let before_rejection = full_snapshot(&pool).await;
    let rejected_login = tokio::time::timeout(
        Duration::from_secs(1),
        request(
            router.clone(),
            "/auth/login",
            input("race-http@example.test", PASSWORD),
        ),
    )
    .await
    .unwrap();
    let rejected_registration = tokio::time::timeout(
        Duration::from_secs(1),
        request(
            router.clone(),
            "/auth/register",
            input("capacity-rejected@example.test", PASSWORD),
        ),
    )
    .await
    .unwrap();
    let live = tokio::time::timeout(
        Duration::from_secs(1),
        router.oneshot(Request::get("/health/live").body(Body::empty()).unwrap()),
    )
    .await
    .unwrap()
    .unwrap();
    let after_rejection = full_snapshot(&pool).await;
    // Release real work before assertions, so assertion failures cannot strand it.
    barrier.commit().await.unwrap();
    let registration = registration.await.unwrap();
    let login = login.await.unwrap();
    assert_eq!(rejected_login.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(rejected_login.3, "{\"error\":\"login_unavailable\"}");
    assert_eq!(rejected_registration.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        rejected_registration.3,
        "{\"error\":\"registration_unavailable\"}"
    );
    assert_eq!(before_rejection, after_rejection);
    assert_eq!(live.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(live.into_body(), 1024).await.unwrap().as_ref(),
        b"OK"
    );
    assert_eq!(registration.0, StatusCode::ACCEPTED);
    assert_eq!(login.0, StatusCode::OK);
    sqlx::raw_sql("DROP TRIGGER auth_register_barrier ON users; DROP TRIGGER auth_login_barrier ON user_sessions; DROP FUNCTION auth_capacity_barrier();")
        .execute(&pool).await.unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn unavailable_hash_and_database_failures_return_generic_service_unavailable() {
    let pool = common::pool().await;
    seed(&pool, "broken-login@example.test", PASSWORD, false).await;
    sqlx::query("UPDATE password_credentials SET password_hash='$argon2id$v=19$m=999999,t=2,p=1$c3ludGhldGljLXNhbHQwMQ$Z8raDknNUDP05Kevs6Eg0WgLPyMIZYQI8LWM5fx94lE' WHERE user_id=(SELECT id FROM users WHERE email_key='broken-login@example.test')")
        .execute(&pool).await.unwrap();
    let router = app(Some(pool.clone()));
    let before = full_snapshot(&pool).await;
    let hash_failure = request(
        router.clone(),
        "/auth/login",
        input("broken-login@example.test", PASSWORD),
    )
    .await;
    assert_eq!(hash_failure.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(hash_failure.3, "{\"error\":\"login_unavailable\"}");
    assert_eq!(full_snapshot(&pool).await, before);

    let valid_hash = hash_password(PASSWORD).unwrap();
    sqlx::query("UPDATE password_credentials SET password_hash=$1 WHERE user_id=(SELECT id FROM users WHERE email_key='broken-login@example.test')")
        .bind(valid_hash)
        .execute(&pool)
        .await
        .unwrap();
    let sessions_before_control: i64 = sqlx::query_scalar("SELECT count(*) FROM user_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    let control = request(
        router.clone(),
        "/auth/login",
        input("broken-login@example.test", PASSWORD),
    )
    .await;
    assert_eq!(control.0, StatusCode::OK);
    let session_count: i64 = sqlx::query_scalar("SELECT count(*) FROM user_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(session_count, sessions_before_control + 1);
    let before_trigger = full_snapshot(&pool).await;

    sqlx::raw_sql("CREATE FUNCTION reject_login_refresh() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic private database detail'; END $$; CREATE TRIGGER reject_login_refresh BEFORE INSERT ON session_refresh_tokens FOR EACH ROW EXECUTE FUNCTION reject_login_refresh();")
        .execute(&pool).await.unwrap();
    let database_failure = request(
        router.clone(),
        "/auth/login",
        input("broken-login@example.test", PASSWORD),
    )
    .await;
    assert_eq!(database_failure.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(database_failure.3, "{\"error\":\"login_unavailable\"}");
    assert!(
        !database_failure
            .3
            .contains("synthetic private database detail")
    );
    assert_eq!(full_snapshot(&pool).await, before_trigger);
    sqlx::raw_sql("DROP TRIGGER reject_login_refresh ON session_refresh_tokens; DROP FUNCTION reject_login_refresh();")
        .execute(&pool).await.unwrap();
    let unavailable = request(
        app(None),
        "/auth/login",
        input("any@example.test", PASSWORD),
    )
    .await;
    assert_eq!(unavailable.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(unavailable.3, "{\"error\":\"login_unavailable\"}");
    assert_eq!(full_snapshot(&pool).await, before_trigger);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn invalid_login_json_uses_shared_error_envelope_without_echoing_input() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    let before = full_snapshot(&pool).await;
    for body in [
        "{".to_owned(),
        "null".to_owned(),
        "{}".to_owned(),
        serde_json::json!({"email":"valid@example.test","password":"secret","user_id":"forged"})
            .to_string(),
    ] {
        let response = request(router.clone(), "/auth/login", body).await;
        assert_eq!(response.0, StatusCode::BAD_REQUEST);
        assert_eq!(response.1, "application/json");
        assert_eq!(response.3, "{\"error\":\"invalid_request\"}");
    }
    let oversized = request(
        router,
        "/auth/login",
        input("valid@example.test", &"x".repeat(8193)),
    )
    .await;
    assert_eq!(oversized.0, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(oversized.1, "application/json");
    assert_eq!(oversized.3, "{\"error\":\"invalid_request\"}");
    assert_eq!(full_snapshot(&pool).await, before);
    pool.close().await;
}
