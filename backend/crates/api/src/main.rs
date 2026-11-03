#[cfg(test)]
use axum::Router;
use cuenora_api::{auth_throttling::ClientIpPolicy, config::AppConfig, registration_http};
#[cfg(test)]
use sqlx::PgPool;

#[cfg(test)]
fn app(pool: Option<PgPool>) -> Router {
    registration_http::app(pool)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = AppConfig::from_env()
        .map_err(|error| std::io::Error::other(format!("Invalid {}", error.0)))?;
    let pool = config
        .database
        .pool(config.database_url.as_deref())
        .map_err(|_| std::io::Error::other("Invalid CUENORA_DATABASE_URL"))?;
    let policy = ClientIpPolicy::from_trusted_proxy_ips(config.trusted_proxy_ips.as_deref())
        .map_err(|_| std::io::Error::other("Invalid CUENORA_TRUSTED_PROXY_IPS"))?;
    let transport: Option<
        std::sync::Arc<dyn cuenora_api::email_verification::VerificationTransport>,
    > = match config.mail {
        Some(mail) => Some(std::sync::Arc::new(
            cuenora_api::verification_mail::ResendSender::new(mail).map_err(|_| {
                std::io::Error::other("Email verification configuration unavailable")
            })?,
        )),
        None => None,
    };
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    println!("Listening on http://{}", listener.local_addr()?);
    axum::serve(
        listener,
        registration_http::app_with_verification(pool, policy, config.lifetimes, transport)
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
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
                    .uri("/v1/health/live")
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
