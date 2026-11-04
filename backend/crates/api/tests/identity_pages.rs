use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use cuenora_api::registration_http::app;
use tower::ServiceExt;

#[tokio::test]
async fn identity_preview_is_inert_self_contained_and_available_without_database() {
    for path in ["/auth/preview"] {
        let response = app(None)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        assert!(
            response.headers()["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("form-action 'none'")
        );
        let body = to_bytes(response.into_body(), 65536).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("Preview only"));
        assert!(!html.contains("<form"));
        assert!(html.contains("<main id=\"content\" tabindex=\"-1\">"));
        assert!(html.find("<h1").unwrap() < html.find("<h2").unwrap_or(usize::MAX));
        assert!(!html.contains("https://"));
    }
}

#[tokio::test]
async fn identity_assets_and_existing_health_route_remain_available() {
    for (path, content_type) in [
        ("/auth/assets/logo.svg", "image/svg+xml"),
        ("/auth/assets/identity.css", "text/css; charset=utf-8"),
        ("/auth/assets/identity.js", "text/javascript; charset=utf-8"),
        ("/v1/health/live", "text/plain; charset=utf-8"),
    ] {
        let response = app(None)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], content_type);
        let body = to_bytes(response.into_body(), 65536).await.unwrap();
        if path.ends_with("live") {
            assert_eq!(&body[..], b"OK");
        }
    }
}

#[tokio::test]
async fn page_post_is_rejected_and_head_has_no_body() {
    for (method, expected) in [
        ("POST", StatusCode::METHOD_NOT_ALLOWED),
        ("HEAD", StatusCode::OK),
    ] {
        let response = app(None)
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/auth/reset-password")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if method == "HEAD" {
            assert!(
                to_bytes(response.into_body(), 65536)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

#[tokio::test]
async fn brand_fonts_are_embedded_and_css_uses_same_origin_faces() {
    for path in [
        "/auth/assets/fonts/bodoni-moda.ttf",
        "/auth/assets/fonts/ibm-plex-mono-regular.ttf",
        "/auth/assets/fonts/ibm-plex-mono-medium.ttf",
    ] {
        let response = app(None)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()["content-type"], "font/ttf");
        assert_eq!(response.headers()["cache-control"], "no-store");
        let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(&bytes[..4], &[0, 1, 0, 0]);
    }
    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/auth/assets/identity.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    let css = std::str::from_utf8(&bytes).unwrap();
    assert!(css.contains("@font-face"));
    assert!(css.contains("font-display: swap"));
    assert!(!css.contains("https://"));
}

#[tokio::test]
async fn bundled_font_licenses_ship_with_the_binary() {
    for path in [
        "/auth/assets/fonts/bodoni-moda-license.txt",
        "/auth/assets/fonts/ibm-plex-mono-license.txt",
    ] {
        let response = app(None)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        assert!(
            std::str::from_utf8(&bytes)
                .unwrap()
                .contains("SIL OPEN FONT LICENSE")
        );
    }
}

#[tokio::test]
async fn recovery_forms_are_progressively_enabled_but_not_natively_submittable() {
    for path in ["/auth/reset-password", "/auth/forgot-password"] {
        let response = app(None)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let policy = response.headers()["content-security-policy"]
            .to_str()
            .unwrap();
        assert!(policy.contains("connect-src 'self'"));
        assert!(policy.contains("form-action 'none'"));
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        assert_eq!(response.headers()["cache-control"], "no-store");
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        let html = std::str::from_utf8(&bytes).unwrap();
        assert!(html.contains("id=\"recovery-form\""));
        assert!(html.contains("/auth/assets/recovery.js"));
        assert!(html.contains("disabled"));
        assert!(!html.contains("Preview only"));
        assert!(!html.contains("name=\"new_password\""));
    }
}
