//! Explicit application database configuration; never includes secrets in errors.
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidDatabaseConfiguration;

/// Does not connect at startup: liveness is independent of database readiness.
/// SQLx follows libpq defaults for omitted URL fields; supply an explicit local URL.
pub fn pool_from_url(url: Option<&str>) -> Result<Option<PgPool>, InvalidDatabaseConfiguration> {
    let Some(url) = url else {
        return Ok(None);
    };
    if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        return Err(InvalidDatabaseConfiguration);
    }
    let options: PgConnectOptions = url.parse().map_err(|_| InvalidDatabaseConfiguration)?;
    let options = options
        .options([("statement_timeout", "5000"), ("lock_timeout", "2000")])
        .disable_statement_logging();
    Ok(Some(
        PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(3))
            .connect_lazy_with(options),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn configuration_is_optional_bounded_and_errors_are_safe() {
        assert!(pool_from_url(None).unwrap().is_none());
        let pool = pool_from_url(Some("postgres://synthetic:secret@127.0.0.1:1/cuenora"))
            .unwrap()
            .unwrap();
        assert_eq!(pool.options().get_max_connections(), 4);
        assert_eq!(
            pool.options().get_acquire_timeout(),
            std::time::Duration::from_secs(3)
        );
        pool.close().await;
        for invalid in [
            "",
            "not a url:secret",
            "mysql://synthetic:secret@127.0.0.1/cuenora",
        ] {
            assert_eq!(
                pool_from_url(Some(invalid)).err(),
                Some(InvalidDatabaseConfiguration)
            );
        }
        assert_eq!(
            format!("{:?}", InvalidDatabaseConfiguration),
            "InvalidDatabaseConfiguration"
        );
    }
}
