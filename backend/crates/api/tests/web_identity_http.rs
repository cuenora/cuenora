use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use cuenora_api::registration_http::app;
use tower::ServiceExt;

#[tokio::test]
async fn sign_in_and_verification_pages_are_enabled_only_by_the_browser_script() {
    for path in ["/auth/sign-in", "/auth/verify-email"] {
        let response = app(None)
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        let policy = response.headers()["content-security-policy"]
            .to_str()
            .unwrap();
        assert!(policy.contains("connect-src 'self'"));
        assert!(policy.contains("form-action 'none'"));
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        let html = std::str::from_utf8(&bytes).unwrap();
        assert!(html.contains("/auth/assets/login.js"));
        assert!(html.contains("id=\"login-form\""));
        assert!(html.contains("id=\"verify-button\" type=\"button\" disabled"));
        assert!(html.contains("id=\"request-button\" type=\"button\" disabled"));
        assert!(html.contains("id=\"logout-button\" type=\"button\" disabled"));
        assert!(
            html.contains(
                "id=\"email\" type=\"email\" autocomplete=\"username\" required disabled"
            )
        );
        assert!(html.contains(
            "id=\"password\" type=\"password\" autocomplete=\"current-password\" required disabled"
        ));
        assert!(!html.contains("Preview only"));
        assert!(!html.contains("access_token"));
        assert!(!html.contains("refresh_token"));
        assert!(!html.contains("<input type=\"hidden\""));
    }
}

#[tokio::test]
async fn login_asset_uses_existing_same_origin_auth_apis_and_no_browser_storage() {
    let response = app(None)
        .oneshot(
            Request::get("/auth/assets/login.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/javascript; charset=utf-8"
    );
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    let script = std::str::from_utf8(&bytes).unwrap();
    for endpoint in [
        "/v1/auth/login",
        "/v1/auth/email-verification/request",
        "/v1/auth/email-verification/confirm",
        "/v1/auth/logout",
    ] {
        assert!(script.contains(endpoint), "missing {endpoint}");
    }
    assert!(script.contains("history.replaceState"));
    assert!(script.contains("pagehide"));
    assert!(script.contains("pageshow"));
    assert!(!script.contains("localStorage"));
    assert!(!script.contains("sessionStorage"));
    assert!(!script.contains("console."));
}

#[tokio::test]
async fn login_route_rejects_posts_and_head_has_no_body() {
    let post = app(None)
        .oneshot(Request::post("/auth/sign-in").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(post.status(), StatusCode::METHOD_NOT_ALLOWED);

    let head = app(None)
        .oneshot(Request::head("/auth/sign-in").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(head.status(), StatusCode::OK);
    assert!(to_bytes(head.into_body(), 65536).await.unwrap().is_empty());
}
