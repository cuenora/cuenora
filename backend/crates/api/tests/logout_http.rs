//! Session revocation acceptance through the public HTTP boundary.
mod common;
#[path = "common/http.rs"]
mod http;
#[path = "common/transport.rs"]
mod transport;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const PASSWORD: &str = "synthetic logout password for acceptance 42";

async fn post(
    router: Router,
    path: &str,
    input: Value,
    access: Option<&str>,
) -> (StatusCode, String) {
    let mut request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-user-id", "untrusted-owner");
    if let Some(access) = access {
        request = request.header(header::AUTHORIZATION, format!("Bearer {access}"));
    }
    let reply = router
        .oneshot(request.body(Body::from(input.to_string())).unwrap())
        .await
        .unwrap();
    let status = reply.status();
    let headers = reply.headers().clone();
    assert!(!headers.contains_key(header::SET_COOKIE));
    let bytes = to_bytes(reply.into_body(), 16384).await.unwrap();
    let body = http::checked_body(&headers, String::from_utf8(bytes.to_vec()).unwrap());
    if path.contains("logout") {
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    }
    (status, body)
}

async fn login(pool: &PgPool, address: &str) -> Value {
    let app = transport::app(Some(pool.clone()));
    let input = json!({"email":address,"password":PASSWORD});
    assert_eq!(
        post(app.clone(), "/auth/register", input.clone(), None)
            .await
            .0,
        StatusCode::ACCEPTED
    );
    let (status, body) = post(app, "/auth/login", input, None).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_str(&body).unwrap()
}

async fn snapshot(pool: &PgPool) -> String {
    let identity = common::snapshot(pool).await;
    let sessions: String = sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s),(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM session_access_tokens a),(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM session_refresh_tokens r))::text)")
        .fetch_one(pool).await.unwrap();
    format!("{identity}:{sessions}")
}

async fn refresh_status(pool: &PgPool, tokens: &Value) -> StatusCode {
    post(
        transport::app(Some(pool.clone())),
        "/auth/refresh",
        json!({"refresh_token":tokens["refresh_token"]}),
        None,
    )
    .await
    .0
}

#[tokio::test]
async fn logout_transport_is_strict_and_global_requires_bearer() {
    let app = transport::app(None);
    let (status, body) = post(
        app.clone(),
        "/auth/logout",
        json!({"refresh_token":"bad","user_id":"forged"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, http::error_body("invalid_request"));
    assert_eq!(
        post(app, "/auth/logout-all", json!({"password":PASSWORD}), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn logout_is_idempotent_family_scoped_and_accepts_retained_generations() {
    let pool = common::pool().await;
    let original = login(&pool, "logout-a@example.test").await;
    let sibling = login(&pool, "logout-a@example.test").await;
    let other = login(&pool, "logout-b@example.test").await;
    let successor =
        cuenora_api::session_store::rotate(&pool, original["refresh_token"].as_str().unwrap())
            .await
            .unwrap()
            .unwrap();
    sqlx::query("UPDATE session_refresh_tokens SET created_at=clock_timestamp()-interval '32 days', expires_at=clock_timestamp()-interval '1 day' WHERE token_hash=$1")
        .bind(sha2::Sha256::digest(original["refresh_token"].as_str().unwrap().as_bytes()).as_slice()).execute(&pool).await.unwrap();
    let app = transport::app(Some(pool.clone()));
    let input = json!({"refresh_token":original["refresh_token"]});
    assert_eq!(
        post(app.clone(), "/auth/logout", input.clone(), None).await,
        (StatusCode::NO_CONTENT, String::new())
    );
    let before = snapshot(&pool).await;
    for input in [
        input,
        json!({"refresh_token":"cr1_invalid"}),
        json!({"refresh_token":cuenora_api::session_tokens::generate().unwrap().refresh.expose_secret()}),
    ] {
        assert_eq!(
            post(app.clone(), "/auth/logout", input, None).await,
            (StatusCode::NO_CONTENT, String::new())
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    assert!(
        cuenora_api::session_store::rotate(&pool, successor.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(refresh_status(&pool, &sibling).await, StatusCode::OK);
    assert_eq!(refresh_status(&pool, &other).await, StatusCode::OK);
    // Even the old, unexpired access generation is no longer usable.
    assert_eq!(
        post(
            app,
            "/auth/logout-all",
            json!({"password":PASSWORD}),
            original["access_token"].as_str()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn global_requires_current_password_revokes_self_and_preserves_other_users() {
    let pool = common::pool().await;
    let a = login(&pool, "global-a@example.test").await;
    let sibling = login(&pool, "global-a@example.test").await;
    let b = login(&pool, "global-b@example.test").await;
    let app = transport::app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    for (input, access, expected) in [
        (
            json!({"password":"wrong"}),
            a["access_token"].as_str(),
            StatusCode::UNAUTHORIZED,
        ),
        (
            json!({"password":PASSWORD,"user_id":"forged"}),
            a["access_token"].as_str(),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"password":PASSWORD}),
            a["refresh_token"].as_str(),
            StatusCode::UNAUTHORIZED,
        ),
    ] {
        assert_eq!(
            post(app.clone(), "/auth/logout-all", input, access).await.0,
            expected
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    assert_eq!(
        post(
            app.clone(),
            "/auth/logout-all",
            json!({"password":PASSWORD}),
            a["access_token"].as_str()
        )
        .await,
        (StatusCode::NO_CONTENT, String::new())
    );
    for token in [&a, &sibling] {
        assert_eq!(refresh_status(&pool, token).await, StatusCode::UNAUTHORIZED);
        assert_eq!(
            post(
                app.clone(),
                "/auth/logout-all",
                json!({"password":PASSWORD}),
                token["access_token"].as_str()
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(refresh_status(&pool, &b).await, StatusCode::OK);
    let fresh = login(&pool, "global-a@example.test").await;
    assert_eq!(refresh_status(&pool, &fresh).await, StatusCode::OK);
    pool.close().await;
}

use sha2::Digest;

async fn wait_lock(pool: &PgPool, marker: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE $1)")
                .bind(format!("%{marker}%")).fetch_one(pool).await.unwrap();
            if waiting { break; }
            tokio::task::yield_now().await;
        }
    }).await.expect("operation must reach the expected database barrier");
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn global_serializes_with_login_and_refresh_at_a_deterministic_cutoff() {
    let pool = common::pool().await;
    // Observations must not compete for the four application DB connections.
    let observer = common::pool().await;
    for login_first in [false, true] {
        let email = if login_first {
            "cutoff-before@example.test"
        } else {
            "cutoff-after@example.test"
        };
        let a = login(&pool, email).await;
        let table = if login_first {
            "session_access_tokens"
        } else {
            "user_sessions"
        };
        let event = if login_first { "INSERT" } else { "UPDATE" };
        sqlx::raw_sql(&format!("CREATE FUNCTION logout_barrier() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(411); RETURN NEW; END $$; CREATE TRIGGER logout_barrier BEFORE {event} ON {table} FOR EACH ROW EXECUTE FUNCTION logout_barrier()"))
            .execute(&pool).await.unwrap();
        let mut blocker = observer.begin().await.unwrap();
        sqlx::query("SELECT pg_advisory_xact_lock(411)")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let app = transport::app(Some(pool.clone()));
        let login_app = app.clone();
        let login_future = async move {
            post(
                login_app,
                "/auth/login",
                json!({"email":email,"password":PASSWORD}),
                None,
            )
            .await
        };
        let logout_app = app.clone();
        let access = a["access_token"].as_str().unwrap().to_owned();
        let logout_future = async move {
            post(
                logout_app,
                "/auth/logout-all",
                json!({"password":PASSWORD}),
                Some(&access),
            )
            .await
        };
        let (login_task, logout_task, refresh_task) = if login_first {
            let login_task = tokio::spawn(login_future);
            wait_lock(&observer, "WITH current_credential").await;
            let logout_task = tokio::spawn(logout_future);
            wait_lock(&observer, "global_credential_lock").await;
            (login_task, logout_task, None)
        } else {
            let logout_task = tokio::spawn(logout_future);
            wait_lock(&observer, "UPDATE user_sessions").await;
            let login_task = tokio::spawn(login_future);
            wait_lock(&observer, "WITH current_credential").await;
            let refresh_pool = pool.clone();
            let old = a.clone();
            let refresh_task =
                tokio::spawn(async move { refresh_status(&refresh_pool, &old).await });
            wait_lock(&observer, "refresh_family_lock").await;
            (login_task, logout_task, Some(refresh_task))
        };
        blocker.commit().await.unwrap();
        assert_eq!(
            logout_task.await.unwrap(),
            (StatusCode::NO_CONTENT, String::new())
        );
        let (status, body) = login_task.await.unwrap();
        assert_eq!(status, StatusCode::OK);
        let new_login: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            refresh_status(&pool, &new_login).await,
            if login_first {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::OK
            }
        );
        if let Some(task) = refresh_task {
            assert_eq!(task.await.unwrap(), StatusCode::UNAUTHORIZED);
        }
        sqlx::raw_sql(&format!(
            "DROP TRIGGER logout_barrier ON {table}; DROP FUNCTION logout_barrier()"
        ))
        .execute(&pool)
        .await
        .unwrap();
    }
    observer.close().await;
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn failed_global_revocation_is_atomic_and_retry_can_succeed() {
    let pool = common::pool().await;
    let a = login(&pool, "atomic-logout@example.test").await;
    let sibling = login(&pool, "atomic-logout@example.test").await;
    sqlx::raw_sql("CREATE FUNCTION reject_logout() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic revocation failure'; END $$; CREATE TRIGGER reject_logout BEFORE UPDATE ON user_sessions FOR EACH ROW EXECUTE FUNCTION reject_logout()")
        .execute(&pool).await.unwrap();
    let app = transport::app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    let (status, body) = post(
        app.clone(),
        "/auth/logout-all",
        json!({"password":PASSWORD}),
        a["access_token"].as_str(),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, http::error_body("authentication_unavailable"));
    assert_eq!(snapshot(&pool).await, before);
    sqlx::raw_sql("DROP TRIGGER reject_logout ON user_sessions; DROP FUNCTION reject_logout()")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        post(
            app,
            "/auth/logout-all",
            json!({"password":PASSWORD}),
            a["access_token"].as_str()
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        refresh_status(&pool, &sibling).await,
        StatusCode::UNAUTHORIZED
    );
    pool.close().await;
}

#[tokio::test]
async fn logout_limits_and_existing_public_transport_contracts_are_preserved() {
    let app = transport::app(None);
    for (path, budget) in [("/auth/logout", 60), ("/auth/logout-all", 20)] {
        for _ in 0..budget {
            assert_ne!(
                post(app.clone(), path, json!({}), None).await.0,
                StatusCode::TOO_MANY_REQUESTS
            );
        }
        assert_eq!(
            post(app.clone(), path, json!({}), None).await.0,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    // Adding logout payloads must not change sibling registration/login/refresh.
    for path in ["/auth/register", "/auth/login", "/auth/refresh"] {
        for input in [
            json!({"password":PASSWORD}),
            json!({"refresh_token":"bad","user_id":"forged"}),
        ] {
            let (status, body) = post(app.clone(), path, input, None).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body, http::error_body("invalid_request"));
        }
    }
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn global_reauthentication_has_a_user_budget_across_client_ips() {
    let pool = common::pool().await;
    let a = login(&pool, "reauth-budget@example.test").await;
    let app = cuenora_api::registration_http::app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    for attempt in 0..11 {
        let routed = app
            .clone()
            .layer(axum::Extension(axum::extract::ConnectInfo(
                format!("192.0.2.{}:1", attempt + 1)
                    .parse::<std::net::SocketAddr>()
                    .unwrap(),
            )));
        let (status, _) = post(
            routed,
            "/auth/logout-all",
            json!({"password":"incorrect"}),
            a["access_token"].as_str(),
        )
        .await;
        assert_eq!(
            status,
            if attempt < 10 {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn stale_password_and_revoked_caller_reject_without_writes() {
    let pool = common::pool().await;
    let observer = common::pool().await;
    for change_password in [true, false] {
        let email = if change_password {
            "stale-password@example.test"
        } else {
            "stale-session@example.test"
        };
        let a = login(&pool, email).await;
        let sibling = login(&pool, email).await;
        let mut blocker = observer.begin().await.unwrap();
        sqlx::query("SELECT user_id FROM password_credentials WHERE user_id=(SELECT id FROM users WHERE email_key=$1) FOR UPDATE")
            .bind(email).execute(&mut *blocker).await.unwrap();
        let app = transport::app(Some(pool.clone()));
        let task_app = app.clone();
        let access = a["access_token"].as_str().unwrap().to_owned();
        let task = tokio::spawn(async move {
            post(
                task_app,
                "/auth/logout-all",
                json!({"password":PASSWORD}),
                Some(&access),
            )
            .await
        });
        wait_lock(&observer, "global_credential_lock").await;
        if change_password {
            sqlx::query("UPDATE password_credentials SET password_changed_at=password_changed_at+interval '1 second' WHERE user_id=(SELECT id FROM users WHERE email_key=$1)")
                .bind(email).execute(&mut *blocker).await.unwrap();
        } else {
            assert_eq!(
                post(
                    app,
                    "/auth/logout",
                    json!({"refresh_token":a["refresh_token"]}),
                    None
                )
                .await
                .0,
                StatusCode::NO_CONTENT
            );
        }
        // The only permitted mutation is the independently committed fixture change.
        blocker.commit().await.unwrap();
        let expected = snapshot(&observer).await;
        assert_eq!(
            task.await.unwrap(),
            (
                StatusCode::UNAUTHORIZED,
                http::error_body("invalid_credentials")
            )
        );
        assert_eq!(snapshot(&observer).await, expected);
        assert_eq!(refresh_status(&pool, &sibling).await, StatusCode::OK);
    }
    observer.close().await;
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn refresh_before_global_revokes_successor_and_cancel_rolls_back() {
    let pool = common::pool().await;
    let observer = common::pool().await;
    let a = login(&pool, "refresh-first@example.test").await;
    sqlx::raw_sql("CREATE FUNCTION logout_probe_barrier() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(9411); RETURN NEW; END $$; CREATE TRIGGER logout_probe_barrier BEFORE INSERT ON session_refresh_tokens FOR EACH ROW EXECUTE FUNCTION logout_probe_barrier()")
        .execute(&pool).await.unwrap();
    let mut blocker = observer.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(9411)")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let app = transport::app(Some(pool.clone()));
    let refresh_app = app.clone();
    let old = a.clone();
    let refresh = tokio::spawn(async move {
        post(
            refresh_app,
            "/auth/refresh",
            json!({"refresh_token":old["refresh_token"]}),
            None,
        )
        .await
    });
    wait_lock(&observer, "INSERT INTO session_refresh_tokens").await;
    let global_app = app.clone();
    let access = a["access_token"].as_str().unwrap().to_owned();
    let global = tokio::spawn(async move {
        post(
            global_app,
            "/auth/logout-all",
            json!({"password":PASSWORD}),
            Some(&access),
        )
        .await
    });
    wait_lock(&observer, "ORDER BY id FOR UPDATE").await;
    blocker.commit().await.unwrap();
    let (status, body) = refresh.await.unwrap();
    assert_eq!(status, StatusCode::OK);
    let successor: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(global.await.unwrap().0, StatusCode::NO_CONTENT);
    assert_eq!(
        refresh_status(&pool, &successor).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(
            app,
            "/auth/logout-all",
            json!({"password":PASSWORD}),
            successor["access_token"].as_str()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    sqlx::raw_sql("DROP TRIGGER logout_probe_barrier ON session_refresh_tokens; DROP FUNCTION logout_probe_barrier()")
        .execute(&pool).await.unwrap();

    let a = login(&pool, "cancel-global@example.test").await;
    let _sibling = login(&pool, "cancel-global@example.test").await;
    sqlx::raw_sql("CREATE FUNCTION logout_probe_barrier() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(9411); RETURN NEW; END $$; CREATE TRIGGER logout_probe_barrier BEFORE UPDATE ON user_sessions FOR EACH ROW EXECUTE FUNCTION logout_probe_barrier()")
        .execute(&pool).await.unwrap();
    let mut blocker = observer.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(9411)")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let before = snapshot(&observer).await;
    let app = transport::app(Some(pool.clone()));
    let access = a["access_token"].as_str().unwrap().to_owned();
    let global = tokio::spawn(async move {
        post(
            app,
            "/auth/logout-all",
            json!({"password":PASSWORD}),
            Some(&access),
        )
        .await
    });
    wait_lock(&observer, "UPDATE user_sessions SET revoked_at").await;
    global.abort();
    assert!(global.await.unwrap_err().is_cancelled());
    blocker.commit().await.unwrap();
    // Acquire every family lock to observe completion of the cancelled SQL operation and rollback.
    let mut barrier = observer.begin().await.unwrap();
    sqlx::query("SELECT id FROM user_sessions ORDER BY id FOR UPDATE")
        .execute(&mut *barrier)
        .await
        .unwrap();
    barrier.commit().await.unwrap();
    assert_eq!(snapshot(&observer).await, before);
    sqlx::raw_sql(
        "DROP TRIGGER logout_probe_barrier ON user_sessions; DROP FUNCTION logout_probe_barrier()",
    )
    .execute(&pool)
    .await
    .unwrap();
    observer.close().await;
    pool.close().await;
}
