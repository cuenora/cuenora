//! Middleware acceptance through HTTP; DB groups use disposable PostgreSQL only.
mod common;
#[path = "common/http.rs"]
mod http;

use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Request, StatusCode, header},
    middleware,
    routing::get,
};
use cuenora_api::{
    auth_identity::{AuthenticatedIdentity, require_identity},
    registration_http::app,
    session_store::{revoke, rotate},
    session_tokens::generate,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;

fn protected(pool: Option<PgPool>) -> Router {
    let private = Router::new()
        .route(
            "/test/identity",
            get(
                |Extension(id): Extension<AuthenticatedIdentity>| async move {
                    Json(json!({"user_id":id.user_id(), "session_id":id.session_id()}))
                },
            ),
        )
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ));
    app(pool).merge(private)
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

async fn send(router: Router, path: &str, values: &[&str]) -> Reply {
    let mut request = Request::get(path).body(Body::empty()).unwrap();
    // Untrusted owner hints must have no effect on the identity extension.
    request
        .headers_mut()
        .insert("x-user-id", HeaderValue::from_static("forged"));
    for value in values {
        request
            .headers_mut()
            .append(header::AUTHORIZATION, value.parse().unwrap());
    }
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body =
        String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec()).unwrap();
    let body = http::checked_body(&headers, body);
    Reply {
        status,
        headers,
        body,
    }
}

fn rejected(reply: Reply, missing: bool) {
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.headers[header::CONTENT_TYPE], "application/json");
    assert_eq!(reply.headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        reply.headers[header::WWW_AUTHENTICATE],
        if missing {
            "Bearer"
        } else {
            "Bearer error=\"invalid_token\""
        }
    );
    assert!(!reply.headers.contains_key(header::SET_COOKIE));
    assert_eq!(reply.body, http::error_body("invalid_access_token"));
}

fn unavailable(reply: Reply) {
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(reply.headers[header::CONTENT_TYPE], "application/json");
    assert!(!reply.headers.contains_key(header::WWW_AUTHENTICATE));
    assert_eq!(reply.body, http::error_body("authentication_unavailable"));
}

#[tokio::test]
async fn missing_malformed_and_ambiguous_headers_never_reach_handler() {
    let router = protected(None);
    rejected(
        send(router.clone(), "/test/identity?access_token=ignored", &[]).await,
        true,
    );
    let token = format!("Bearer {}", generate().unwrap().access.expose_secret());
    for value in [
        "",
        "Basic abc",
        "Bearer",
        "Bearer ",
        "Bearer ca1_short",
        "Bearer cr1_abc",
        "Bearer\tca1_abc",
        "Bearer ca1_abc extra",
    ] {
        rejected(
            send(router.clone(), "/test/identity", &[value]).await,
            false,
        );
    }
    rejected(
        send(router.clone(), "/test/identity", &[&token, &token]).await,
        false,
    );
    rejected(
        send(
            router.clone(),
            "/test/identity",
            &[&format!("{token}, {token}")],
        )
        .await,
        false,
    );
    let invalid = format!("Bearer ca1_{}", "F".repeat(64));
    rejected(
        send(router.clone(), "/test/identity", &[&invalid]).await,
        false,
    );
    let long = format!("{token}0");
    rejected(send(router, "/test/identity", &[&long]).await, false);
}

#[tokio::test]
async fn storage_failures_are_closed_and_public_routes_stay_public() {
    let token = generate().unwrap();
    let bearer = format!("Bearer {}", token.access.expose_secret());
    unavailable(send(protected(None), "/test/identity", &[&bearer]).await);
    let pool =
        cuenora_api::config::pool_from_url(Some("postgres://synthetic:secret@127.0.0.1:1/cuenora"))
            .unwrap()
            .unwrap();
    pool.close().await;
    unavailable(send(protected(Some(pool)), "/test/identity", &[&bearer]).await);
    let router = protected(None);
    let live = send(router.clone(), "/health/live", &["invalid"]).await;
    assert_eq!(live.status, StatusCode::OK);
    assert_eq!(live.body, "OK");
    assert!(!live.headers.contains_key(header::WWW_AUTHENTICATE));
    for route in ["/auth/register", "/auth/login", "/auth/refresh"] {
        let request = Request::post(route)
            .header(header::AUTHORIZATION, "invalid")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let reply = router.clone().oneshot(request).await.unwrap();
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        assert!(!reply.headers().contains_key(header::WWW_AUTHENTICATE));
    }
}

async fn login(pool: &PgPool, email: &str, verified: bool) -> Value {
    let router = app(Some(pool.clone()));
    let input = json!({"email":email,"password":"synthetic identity password 42"}).to_string();
    for (route, expected) in [
        ("/auth/register", StatusCode::ACCEPTED),
        ("/auth/login", StatusCode::OK),
    ] {
        if verified && route == "/auth/login" {
            sqlx::query("UPDATE users SET email_verified_at=clock_timestamp() WHERE email_key=$1")
                .bind(email)
                .execute(pool)
                .await
                .unwrap();
        }
        let response = router
            .clone()
            .oneshot(
                Request::post(route)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(input.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if route == "/auth/login" {
            return serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap())
                .unwrap();
        }
    }
    unreachable!()
}

async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM users u), (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY user_id), '[]') FROM password_credentials c), (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s), (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM session_access_tokens a), (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM session_refresh_tokens r))::text)").fetch_one(pool).await.unwrap()
}

async fn identity(pool: &PgPool, tokens: &Value) -> Value {
    let digest = Sha256::digest(tokens["access_token"].as_str().unwrap().as_bytes());
    let (user, session): (String, String) = sqlx::query_as(
        "SELECT user_id::text, session_id::text FROM session_access_tokens WHERE token_hash=$1",
    )
    .bind(digest.as_slice())
    .fetch_one(pool)
    .await
    .unwrap();
    json!({"user_id":user,"session_id":session})
}

async fn accepted(router: Router, tokens: &Value, expected: &Value) {
    // HTTP auth schemes are case insensitive; the separator permits 1+ SP.
    let bearer = format!("bEaReR   {}", tokens["access_token"].as_str().unwrap());
    let reply = send(
        router,
        "/test/identity?user_id=forged&session_id=forged",
        &[&bearer],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        serde_json::from_str::<Value>(&reply.body).unwrap(),
        *expected
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL harness"]
async fn valid_pending_verified_and_independent_sessions_derive_identity_without_writes() {
    let pool = common::pool().await;
    let a = login(&pool, "identity-a@example.test", false).await;
    let b = login(&pool, "identity-b@example.test", true).await;
    let a2 = login(&pool, "identity-a@example.test", false).await;
    let expected_a = identity(&pool, &a).await;
    let expected_b = identity(&pool, &b).await;
    let expected_a2 = identity(&pool, &a2).await;
    assert_ne!(expected_a["user_id"], expected_b["user_id"]);
    assert_eq!(expected_a["user_id"], expected_a2["user_id"]);
    assert_ne!(expected_a["session_id"], expected_a2["session_id"]);
    let before = snapshot(&pool).await;
    let identity_before = common::snapshot(&pool).await;
    let router = protected(Some(pool.clone()));
    // A valid secret must not authenticate through an alternate transport.
    for channel in ["cookie", "query", "body"] {
        let secret = a["access_token"].as_str().unwrap();
        let path = if channel == "query" {
            format!("/test/identity?access_token={secret}")
        } else {
            "/test/identity".to_string()
        };
        let mut request = Request::get(path);
        if channel == "cookie" {
            request = request.header(header::COOKIE, format!("access_token={secret}"));
        }
        let body = if channel == "body" {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(json!({"access_token":secret}).to_string())
        } else {
            Body::empty()
        };
        let response = router
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec())
            .unwrap();
        let body = http::checked_body(&headers, body);
        rejected(
            Reply {
                status,
                headers,
                body,
            },
            true,
        );
    }
    assert_eq!(snapshot(&pool).await, before);
    for (tokens, expected) in [
        (&a, &expected_a),
        (&b, &expected_b),
        (&a2, &expected_a2),
        (&a, &expected_a),
    ] {
        accepted(router.clone(), tokens, expected).await;
    }
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(common::snapshot(&pool).await, identity_before);
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL harness"]
async fn unknown_expired_refresh_and_revoked_credentials_cannot_authenticate() {
    let pool = common::pool().await;
    let a = login(&pool, "reject-a@example.test", false).await;
    let b = login(&pool, "reject-b@example.test", true).await;
    let expired = login(&pool, "reject-expired@example.test", false).await;
    let digest = Sha256::digest(expired["access_token"].as_str().unwrap().as_bytes());
    sqlx::query("UPDATE session_access_tokens SET created_at=clock_timestamp()-interval '2 hours', expires_at=clock_timestamp()-interval '1 hour' WHERE token_hash=$1").bind(digest.as_slice()).execute(&pool).await.unwrap();
    let unknown = generate().unwrap();
    let router = protected(Some(pool.clone()));
    let before = snapshot(&pool).await;
    for token in [
        unknown.access.expose_secret(),
        a["refresh_token"].as_str().unwrap(),
        expired["access_token"].as_str().unwrap(),
    ] {
        rejected(
            send(
                router.clone(),
                "/test/identity",
                &[&format!("Bearer {token}")],
            )
            .await,
            false,
        );
    }
    rejected(send(router.clone(), "/test/identity", &[]).await, true);
    assert_eq!(snapshot(&pool).await, before);
    // Rotation keeps old access alive until its own expiry, with the same identity.
    let successor = rotate(&pool, a["refresh_token"].as_str().unwrap())
        .await
        .unwrap()
        .unwrap();
    accepted(router.clone(), &a, &identity(&pool, &a).await).await;
    let new_access = json!({"access_token":successor.access.expose_secret()});
    accepted(router.clone(), &new_access, &identity(&pool, &a).await).await;
    // Replay revokes the family, including both still-unexpired access generations.
    assert!(
        rotate(&pool, a["refresh_token"].as_str().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    let before = snapshot(&pool).await;
    for token in [
        a["access_token"].as_str().unwrap(),
        successor.access.expose_secret(),
    ] {
        rejected(
            send(
                router.clone(),
                "/test/identity",
                &[&format!("Bearer {token}")],
            )
            .await,
            false,
        );
    }
    accepted(router.clone(), &b, &identity(&pool, &b).await).await;
    assert_eq!(snapshot(&pool).await, before);
    assert!(
        revoke(&pool, b["refresh_token"].as_str().unwrap())
            .await
            .unwrap()
    );
    let before = snapshot(&pool).await;
    rejected(
        send(
            router,
            "/test/identity",
            &[&format!("Bearer {}", b["access_token"].as_str().unwrap())],
        )
        .await,
        false,
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL harness"]
async fn database_query_failure_never_admits_a_handler_or_changes_data() {
    let pool = common::pool().await;
    let tokens = login(&pool, "unavailable-identity@example.test", false).await;
    let before = snapshot(&pool).await;
    // A connection without application tables deterministically fails the query,
    // without modifying the schema, role permissions or another pool's settings.
    let options = pool
        .connect_options()
        .as_ref()
        .clone()
        .options([("search_path", "pg_catalog")]);
    let broken = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy_with(options);
    unavailable(
        send(
            protected(Some(broken.clone())),
            "/test/identity",
            &[&format!(
                "Bearer {}",
                tokens["access_token"].as_str().unwrap()
            )],
        )
        .await,
    );
    assert_eq!(snapshot(&pool).await, before);
    broken.close().await;
}
