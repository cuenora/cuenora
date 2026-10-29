use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use cuenora_api::registration_http::app;
use tower::ServiceExt;

#[tokio::test]
async fn all_legacy_paths_are_removed_and_versioned_health_is_available() {
    let router = app(None);
    for path in [
        "/health/live",
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
        "/auth/logout-all",
    ] {
        for method in ["GET", "POST"] {
            let reply = router
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
            assert_eq!(reply.status(), StatusCode::NOT_FOUND, "{method} {path}");
        }
    }
    let reply = router
        .oneshot(Request::get("/v1/health/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(reply.into_body(), 1024).await.unwrap().as_ref(),
        b"OK"
    );
}
