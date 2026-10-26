use axum::Router;
use cuenora_api::{config::pool_from_url, registration_http};
use sqlx::PgPool;

fn app(pool: Option<PgPool>) -> Router {
    registration_http::app(pool)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let url = match std::env::var("CUENORA_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err(std::io::Error::other("Invalid CUENORA_DATABASE_URL")),
    };
    let pool = pool_from_url(url.as_deref())
        .map_err(|_| std::io::Error::other("Invalid CUENORA_DATABASE_URL"))?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app(pool)).await
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn liveness_returns_ok() {
        let response = super::app(None)
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/plain; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(body.as_ref(), b"OK");
    }
}
