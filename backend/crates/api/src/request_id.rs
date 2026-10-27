//! Server-generated diagnostic correlation, not authentication or idempotency.
use crate::api_error::ApiError;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::{
    Router,
    extract::Request,
    http::HeaderValue,
    middleware::{self, Next},
    response::Response,
};

/// Opaque server identity for one request. Never constructed from HTTP headers.
#[derive(Clone, Debug)]
pub struct RequestId(String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn attach(&self, response: &mut Response) {
        // Generated lowercase hex is always a valid header value.
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(self.as_str()).expect("hex request ID"),
        );
    }
}

fn ensure_with(
    request: &mut Request,
    fill: impl FnOnce(&mut [u8]) -> Result<(), ()>,
) -> Result<RequestId, ()> {
    // Nested server layers share one ID; incoming headers are never consulted.
    if let Some(id) = request.extensions().get::<RequestId>() {
        return Ok(id.clone());
    }
    let mut bytes = [0u8; 16];
    fill(&mut bytes)?;
    let id = RequestId(bytes.iter().map(|byte| format!("{byte:02x}")).collect());
    request.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(id.as_str()).expect("hex request ID"),
    );
    request.extensions_mut().insert(id.clone());
    Ok(id)
}

fn fill_id(bytes: &mut [u8]) -> Result<(), ()> {
    OsRng.try_fill_bytes(bytes).map_err(|_| ())
}

pub(crate) fn ensure(request: &mut Request) -> Result<RequestId, ()> {
    ensure_with(request, fill_id)
}

async fn context_with(
    mut request: Request,
    next: Next,
    fill: impl FnOnce(&mut [u8]) -> Result<(), ()>,
) -> Response {
    let Ok(id) = ensure_with(&mut request, fill) else {
        return ApiError::without_request_id();
    };
    let mut response = next.run(request).await;
    id.attach(&mut response);
    response
}

async fn context(request: Request, next: Next) -> Response {
    context_with(request, next, fill_id).await
}

/// Wrap the FINAL composed router so successes, rejections and fallbacks share
/// one ID. Safe to nest: existing server-generated extensions are reused.
pub fn with_request_context(router: Router) -> Router {
    router.layer(middleware::from_fn(context))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn entropy_failure_never_reaches_handler_or_returns_a_partial_id() {
        async fn forbidden() -> &'static str {
            panic!("handler reached")
        }
        let router = Router::new()
            .route("/", get(forbidden))
            .layer(middleware::from_fn(
                |request: axum::extract::Request, next: Next| async move {
                    context_with(request, next, |bytes| {
                        bytes.fill(42);
                        Err(())
                    })
                    .await
                },
            ));
        let head = router
            .clone()
            .oneshot(Request::head("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(head.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(head.headers()["cache-control"], "no-store");
        assert!(!head.headers().contains_key("x-request-id"));
        assert!(to_bytes(head.into_body(), 4096).await.unwrap().is_empty());
        let response = router
            .oneshot(
                Request::get("/")
                    .header("x-request-id", "forged")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(!response.headers().contains_key("x-request-id"));
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"code":"request_id_unavailable", "message":"The request is temporarily unavailable.", "details":null, "request_id":null})
        );
    }
}
