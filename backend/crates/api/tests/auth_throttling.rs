//! Abuse limits through the public HTTP router, without live storage.
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use cuenora_api::registration_http::app;
use serde_json::Value;
use std::net::SocketAddr;
use tower::ServiceExt;

async fn post(
    router: Router,
    path: &str,
    peer: &str,
    body: &str,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    send(router, path, Some(peer), body, &[]).await
}

async fn send(
    router: Router,
    path: &str,
    peer: Option<&str>,
    body: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(peer) = peer {
        request = request.extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    }
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body =
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
    (status, headers, body)
}

#[tokio::test]
async fn each_sensitive_route_throttles_before_payload_or_storage_work() {
    for (path, budget) in [
        ("/auth/register", 5),
        ("/auth/login", 20),
        ("/auth/refresh", 60),
    ] {
        let router = app(None);
        for _ in 0..budget {
            assert_eq!(
                post(router.clone(), path, "192.0.2.1:1", "{").await.0,
                StatusCode::BAD_REQUEST
            );
        }
        let (status, headers, body) = post(router.clone(), path, "192.0.2.1:2", "{").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{path}");
        assert_eq!(body["code"], "rate_limited");
        assert_eq!(body["message"], "Too many requests. Try again later.");
        assert_eq!(body["details"], Value::Null);
        assert_eq!(
            body["request_id"],
            headers["x-request-id"].to_str().unwrap()
        );
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(
            (1..=60).contains(
                &headers[header::RETRY_AFTER]
                    .to_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            )
        );
        assert_eq!(
            post(router, path, "192.0.2.2:1", "{").await.0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn login_identifier_limit_is_shared_across_ips_and_email_case() {
    let router = app(None);
    for n in 1..=10 {
        let body = if n % 2 == 0 {
            r#"{"email":"Test@EXAMPLE.test","password":"wrong"}"#
        } else {
            r#"{"email":"test@example.test","password":"wrong"}"#
        };
        assert_eq!(
            post(
                router.clone(),
                "/auth/login",
                &format!("192.0.2.{n}:1"),
                body
            )
            .await
            .0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    let (status, _, body) = post(
        router,
        "/auth/login",
        "192.0.2.11:1",
        r#"{"email":"test@example.test","password":"wrong"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], "rate_limited");
}

#[tokio::test]
async fn proxy_identity_requires_trusted_peer_and_exactly_one_valid_cloudflare_ip() {
    use cuenora_api::{auth_throttling::ClientIpPolicy, registration_http::app_with_client_policy};
    let router = app_with_client_policy(
        None,
        ClientIpPolicy::from_trusted_proxy_ips(Some("192.0.2.10, ::1")).unwrap(),
    );
    for (peer, headers) in [
        (None, vec![("cf-connecting-ip", "198.51.100.1")]),
        (
            Some("192.0.2.11:1"),
            vec![("cf-connecting-ip", "198.51.100.1")],
        ),
        (Some("192.0.2.10:1"), vec![]),
        (
            Some("192.0.2.10:1"),
            vec![("x-forwarded-for", "198.51.100.1")],
        ),
        (Some("192.0.2.10:1"), vec![("cf-connecting-ip", "invalid")]),
        (
            Some("192.0.2.10:1"),
            vec![("cf-connecting-ip", "198.51.100.1, 198.51.100.2")],
        ),
        (
            Some("192.0.2.10:1"),
            vec![
                ("cf-connecting-ip", "198.51.100.1"),
                ("cf-connecting-ip", "198.51.100.2"),
            ],
        ),
        (Some("192.0.2.10:1"), vec![("cf-connecting-ip", "0.0.0.0")]),
    ] {
        for path in ["/auth/register", "/auth/login", "/auth/refresh"] {
            let (status, headers, body) = send(router.clone(), path, peer, "{", &headers).await;
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(body["code"], "protection_unavailable");
            assert_eq!(
                body["message"],
                "Request protection is temporarily unavailable."
            );
            assert_eq!(body["details"], Value::Null);
            assert_eq!(
                body["request_id"],
                headers["x-request-id"].to_str().unwrap()
            );
            assert_eq!(headers[header::CACHE_CONTROL], "no-store");
            assert!(!headers.contains_key(header::RETRY_AFTER));
        }
    }
    for n in 0..6 {
        let (status, _, _) = send(
            router.clone(),
            "/auth/register",
            Some("192.0.2.10:1"),
            "{",
            &[
                ("cf-connecting-ip", "198.51.100.1"),
                ("x-forwarded-for", "203.0.113.50"),
            ],
        )
        .await;
        assert_eq!(
            status,
            if n < 5 {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
    }
    assert_eq!(
        send(
            router,
            "/auth/register",
            Some("[::1]:1"),
            "{",
            &[("cf-connecting-ip", "198.51.100.2")]
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn direct_clients_cannot_rotate_forwarded_headers_or_ipv6_aliases_to_bypass_limits() {
    let router = app(None);
    assert_eq!(
        send(router.clone(), "/auth/login", None, "{", &[]).await.2["code"],
        "protection_unavailable"
    );
    for n in 1..=6 {
        let (status, _, _) = send(
            router.clone(),
            "/auth/register",
            Some("192.0.2.1:1"),
            "{",
            &[
                ("cf-connecting-ip", &format!("198.51.100.{n}")),
                ("x-forwarded-for", "203.0.113.1"),
            ],
        )
        .await;
        assert_eq!(
            status,
            if n <= 5 {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
    }
    assert_eq!(
        post(
            router.clone(),
            "/auth/register",
            "[::ffff:192.0.2.1]:1",
            "{"
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        post(router.clone(), "/auth/login", "192.0.2.1:1", "{")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let ipv6 = app(None);
    for n in 1..=6 {
        assert_eq!(
            post(
                ipv6.clone(),
                "/auth/register",
                &format!("[2001:db8:1:2::{n}]:1"),
                "{"
            )
            .await
            .0,
            if n <= 5 {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
    }
    assert_eq!(
        post(ipv6, "/auth/register", "[2001:db8:1:3::1]:1", "{")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        post(app(None), "/auth/register", "192.0.2.1:1", "{")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    for (method, path, expected) in [
        ("GET", "/health/live", StatusCode::OK),
        ("GET", "/auth/register", StatusCode::METHOD_NOT_ALLOWED),
        ("POST", "/missing", StatusCode::NOT_FOUND),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert!(!response.headers().contains_key(header::RETRY_AFTER));
    }
}

#[tokio::test]
async fn concurrent_router_clones_share_one_atomic_budget() {
    let router = app(None);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..32 {
        let router = router.clone();
        tasks.spawn(async move { post(router, "/auth/login", "192.0.2.1:1", "{").await.0 });
    }
    let mut admitted = 0;
    let mut rejected = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            StatusCode::BAD_REQUEST => admitted += 1,
            StatusCode::TOO_MANY_REQUESTS => rejected += 1,
            other => panic!("unexpected {other}"),
        }
    }
    assert_eq!((admitted, rejected), (20, 12));
}

mod common;

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn throttled_valid_writes_preserve_all_identity_and_session_rows() {
    let pool = common::pool().await;
    let snapshot = || async {
        sqlx::query_scalar::<_, String>("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM users t), (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY user_id), '[]') FROM password_credentials t), (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM user_sessions t), (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY token_hash), '[]') FROM session_access_tokens t), (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY token_hash), '[]') FROM session_refresh_tokens t))::text)").fetch_one(&pool).await.unwrap()
    };
    // Seed a real credential and live refresh generation to make accidental
    // registration, login or refresh execution observably mutate the snapshot.
    let setup = app(Some(pool.clone()));
    let credentials =
        r#"{"email":"throttling@example.test","password":"violet streams beside winter hills"}"#;
    assert_eq!(
        post(setup.clone(), "/auth/register", "192.0.2.1:1", credentials)
            .await
            .0,
        StatusCode::ACCEPTED
    );
    let (status, _, tokens) = post(setup, "/auth/login", "192.0.2.1:1", credentials).await;
    assert_eq!(status, StatusCode::OK);
    let refresh = serde_json::json!({"refresh_token":tokens["refresh_token"]}).to_string();
    let identity_before = common::snapshot(&pool).await;
    let before = snapshot().await;
    for (path, budget, input) in [
        (
            "/auth/register",
            5,
            r#"{"email":"fresh-throttled@example.test","password":"violet streams beside winter hills"}"#,
        ),
        ("/auth/login", 20, credentials),
        ("/auth/refresh", 60, refresh.as_str()),
    ] {
        let router = app(Some(pool.clone()));
        for _ in 0..budget {
            assert_eq!(
                post(router.clone(), path, "192.0.2.1:1", "{").await.0,
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            post(router, path, "192.0.2.1:1", input).await.0,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(snapshot().await, before);
    }
    // Identifier admission also rejects a correct password from a new IP.
    let router = app(Some(pool.clone()));
    for n in 1..=10 {
        assert_eq!(
            post(
                router.clone(),
                "/auth/login",
                &format!("192.0.2.{n}:1"),
                r#"{"email":"throttling@example.test","password":"wrong"}"#
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
    let before = snapshot().await;
    assert_eq!(
        post(router, "/auth/login", "192.0.2.20:1", credentials)
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(snapshot().await, before);
    assert_eq!(common::snapshot(&pool).await, identity_before);
    pool.close().await;
}
