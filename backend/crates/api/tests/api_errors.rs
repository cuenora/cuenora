//! Error envelopes and diagnostic IDs through the public HTTP boundary.
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::registration_http::app;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn errors_have_safe_typed_envelopes_and_matching_server_ids() {
    for (method, path, body, status, code) in [
        (
            "POST",
            "/auth/register",
            "{",
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
        (
            "POST",
            "/auth/login",
            "{}",
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
        (
            "POST",
            "/auth/refresh",
            "{}",
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
        (
            "GET",
            "/missing?secret=private",
            "",
            StatusCode::NOT_FOUND,
            "not_found",
        ),
        (
            "GET",
            "/auth/register",
            "",
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
        ),
    ] {
        let response = app(None)
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-request-id", "untrusted-secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        let id = response
            .headers()
            .get("x-request-id")
            .expect("server ID")
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(id.len(), 32);
        assert!(
            id.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        assert_ne!(id, "untrusted-secret");
        if status == StatusCode::METHOD_NOT_ALLOWED {
            assert_eq!(response.headers()[header::ALLOW], "POST");
        }
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 4);
        assert_eq!(value["code"], code);
        assert!(value["message"].as_str().is_some_and(|v| !v.is_empty()));
        assert!(value["details"].is_null());
        assert_eq!(value["request_id"], id);
        assert!(
            !String::from_utf8(body.to_vec())
                .unwrap()
                .contains("private")
        );
    }
}

#[tokio::test]
async fn success_keeps_its_body_and_gets_a_fresh_id_ignoring_duplicate_client_ids() {
    let router = app(None);
    let mut ids = std::collections::HashSet::new();
    for _ in 0..32 {
        let response = router
            .clone()
            .oneshot(
                Request::get("/health/live")
                    .header("x-request-id", "first")
                    .header("x-request-id", "second")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let id = response
            .headers()
            .get("x-request-id")
            .expect("server ID")
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(id.len(), 32);
        assert!(ids.insert(id));
        assert_eq!(to_bytes(response.into_body(), 100).await.unwrap(), "OK");
    }
}

#[tokio::test]
async fn all_existing_transport_failures_keep_their_status_code_and_security_headers() {
    for (path, unavailable) in [
        ("/auth/register", "registration_unavailable"),
        ("/auth/login", "login_unavailable"),
        ("/auth/refresh", "refresh_unavailable"),
    ] {
        for (media, body, status, code) in [
            (
                "text/plain",
                "{}".to_owned(),
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_request",
            ),
            (
                "application/json",
                "x".repeat(8193),
                StatusCode::PAYLOAD_TOO_LARGE,
                "invalid_request",
            ),
            (
                "application/json",
                if path == "/auth/refresh" {
                    "{\"refresh_token\":\"bad\"}".to_owned()
                } else {
                    "{\"email\":\"test@example.test\",\"password\":\"violet meadows beneath winter skies\"}".to_owned()
                },
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable,
            ),
        ] {
            let response = app(None)
                .oneshot(
                    Request::post(path)
                        .header(header::CONTENT_TYPE, media)
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
            assert!(!response.headers().contains_key(header::WWW_AUTHENTICATE));
            assert!(!response.headers().contains_key(header::SET_COOKIE));
            if path == "/auth/refresh" {
                assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            }
            let id = response.headers()["x-request-id"]
                .to_str()
                .unwrap()
                .to_owned();
            let value: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap())
                    .unwrap();
            assert_eq!(value["request_id"], id);
            assert_eq!(value["code"], code);
            assert!(value["details"].is_null());
        }
    }
}

#[tokio::test]
async fn final_composed_context_reuses_typed_id_and_covers_auth_rejections() {
    use axum::{Extension, Json, Router, middleware, routing::get};
    use cuenora_api::{
        auth_identity::require_identity,
        request_id::{RequestId, with_request_context},
    };
    async fn forbidden() -> &'static str {
        panic!("unauthenticated handler")
    }
    let private = Router::new().route("/private", get(forbidden)).route_layer(
        middleware::from_fn_with_state(None::<sqlx::PgPool>, require_identity),
    );
    let echo = Router::new().route(
        "/context",
        get(
            |Extension(id): Extension<RequestId>, headers: axum::http::HeaderMap| async move {
                assert_eq!(headers["x-request-id"], id.as_str());
                Json(serde_json::json!({"id":id.as_str()}))
            },
        ),
    );
    let router = with_request_context(with_request_context(app(None).merge(private).merge(echo)));
    for token in [
        None,
        Some("Basic private-secret"),
        Some(&format!("Bearer ca1_{}", "a".repeat(64))),
    ] {
        let mut request = Request::get("/private");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, token);
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        if token.is_some_and(|t| t.starts_with("Bearer")) {
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert!(!response.headers().contains_key(header::WWW_AUTHENTICATE));
        } else {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(
                response.headers()[header::WWW_AUTHENTICATE],
                if token.is_none() {
                    "Bearer"
                } else {
                    "Bearer error=\"invalid_token\""
                }
            );
        }
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(value["request_id"], id);
    }
    let mut invalid_hint = Request::get("/context").body(Body::empty()).unwrap();
    invalid_hint.headers_mut().append(
        "x-request-id",
        axum::http::HeaderValue::from_bytes(b"\xff").unwrap(),
    );
    invalid_hint.headers_mut().append(
        "x-request-id",
        axum::http::HeaderValue::from_static("duplicate"),
    );
    let invalid_response = router.clone().oneshot(invalid_hint).await.unwrap();
    assert_eq!(invalid_response.status(), StatusCode::OK);
    let invalid_id = invalid_response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(invalid_id.len(), 32);
    let invalid_body: Value =
        serde_json::from_slice(&to_bytes(invalid_response.into_body(), 4096).await.unwrap())
            .unwrap();
    assert_eq!(invalid_body["id"], invalid_id);
    let response = router
        .oneshot(
            Request::get("/context")
                .header("x-request-id", "forged")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn head_and_options_keep_http_semantics() {
    for (path, status) in [
        ("/health/live", StatusCode::OK),
        ("/auth/register", StatusCode::METHOD_NOT_ALLOWED),
        ("/missing", StatusCode::NOT_FOUND),
    ] {
        let response = app(None)
            .oneshot(Request::head(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert!(response.headers().contains_key("x-request-id"));
        assert!(
            to_bytes(response.into_body(), 4096)
                .await
                .unwrap()
                .is_empty()
        );
    }
    let response = app(None)
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()[header::ALLOW], "GET,HEAD");
}
