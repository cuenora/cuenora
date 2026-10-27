//! Existing handler tests run below abuse budgets with realistic TCP metadata.
pub fn app(pool: Option<sqlx::PgPool>) -> axum::Router {
    cuenora_api::registration_http::app(pool).layer(axum::Extension(axum::extract::ConnectInfo(
        "192.0.2.1:1".parse::<std::net::SocketAddr>().unwrap(),
    )))
}
