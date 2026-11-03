//! Explicit application database configuration; never includes secrets in errors.
use crate::session_tokens::TokenLifetimes;
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{net::SocketAddr, time::Duration};

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidDatabaseConfiguration;

/// Does not connect at startup: liveness is independent of database readiness.
/// SQLx follows libpq defaults for omitted URL fields; supply an explicit local URL.
pub fn pool_from_url(url: Option<&str>) -> Result<Option<PgPool>, InvalidDatabaseConfiguration> {
    DatabaseSettings::default().pool(url)
}

/// Safe configuration errors carry a variable name, never its value.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidConfiguration(pub &'static str);

#[derive(Clone, Copy, Debug)]
pub struct DatabaseSettings {
    pub max_connections: u32,
    pub acquire_timeout_ms: u64,
    pub statement_timeout_ms: u64,
    pub lock_timeout_ms: u64,
}
impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            max_connections: 4,
            acquire_timeout_ms: 3000,
            statement_timeout_ms: 5000,
            lock_timeout_ms: 2000,
        }
    }
}
impl DatabaseSettings {
    pub fn pool(self, url: Option<&str>) -> Result<Option<PgPool>, InvalidDatabaseConfiguration> {
        let Some(url) = url else {
            return Ok(None);
        };
        if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
            return Err(InvalidDatabaseConfiguration);
        }
        let options: PgConnectOptions = url.parse().map_err(|_| InvalidDatabaseConfiguration)?;
        let options = options
            .options([
                ("statement_timeout", self.statement_timeout_ms.to_string()),
                ("lock_timeout", self.lock_timeout_ms.to_string()),
            ])
            .disable_statement_logging();
        Ok(Some(
            PgPoolOptions::new()
                .max_connections(self.max_connections)
                .acquire_timeout(Duration::from_millis(self.acquire_timeout_ms))
                .connect_lazy_with(options),
        ))
    }
}

/// Immutable startup configuration. Intentionally does not implement Debug:
/// database URLs may contain credentials. No dotenv loading or global state.
pub struct AppConfig {
    pub mail: Option<crate::verification_mail::ResendConfig>,
    pub listen_addr: SocketAddr,
    pub public_api_url: String,
    pub database_url: Option<String>,
    pub trusted_proxy_ips: Option<String>,
    pub database: DatabaseSettings,
    pub lifetimes: TokenLifetimes,
}
impl AppConfig {
    pub fn from_env() -> Result<Self, InvalidConfiguration> {
        Self::from_lookup(|name| match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(_) => Err(InvalidConfiguration(name)),
        })
    }
    pub fn from_lookup(
        mut get: impl FnMut(&'static str) -> Result<Option<String>, InvalidConfiguration>,
    ) -> Result<Self, InvalidConfiguration> {
        let listen_addr = get("CUENORA_LISTEN_ADDR")?
            .unwrap_or_else(|| "127.0.0.1:3000".into())
            .parse::<SocketAddr>()
            .map_err(|_| InvalidConfiguration("CUENORA_LISTEN_ADDR"))?;
        if listen_addr.port() == 0 {
            return Err(InvalidConfiguration("CUENORA_LISTEN_ADDR"));
        }
        let public_api_url =
            get("CUENORA_PUBLIC_API_URL")?.unwrap_or_else(|| "http://127.0.0.1:3000".into());
        // Reuse the HTTP URI parser already present through Axum; no new dependency.
        validate_public_url(&public_api_url)?;
        let database_url = get("CUENORA_DATABASE_URL")?;
        if let Some(url) = &database_url
            && (!(url.starts_with("postgres://") || url.starts_with("postgresql://"))
                || url.parse::<PgConnectOptions>().is_err())
        {
            return Err(InvalidConfiguration("CUENORA_DATABASE_URL"));
        }
        let trusted_proxy_ips = get("CUENORA_TRUSTED_PROXY_IPS")?;
        crate::auth_throttling::ClientIpPolicy::from_trusted_proxy_ips(
            trusted_proxy_ips.as_deref(),
        )
        .map_err(|_| InvalidConfiguration("CUENORA_TRUSTED_PROXY_IPS"))?;
        let mut number = |name, default, min, max| -> Result<u64, InvalidConfiguration> {
            let Some(raw) = get(name)? else {
                return Ok(default);
            };
            if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
                return Err(InvalidConfiguration(name));
            }
            let value = raw.parse::<u64>().map_err(|_| InvalidConfiguration(name))?;
            if value < min || value > max {
                return Err(InvalidConfiguration(name));
            }
            Ok(value)
        };
        let database = DatabaseSettings {
            max_connections: number("CUENORA_DB_MAX_CONNECTIONS", 4, 1, 100)? as u32,
            acquire_timeout_ms: number("CUENORA_DB_ACQUIRE_TIMEOUT_MS", 3000, 1, 60000)?,
            statement_timeout_ms: number("CUENORA_DB_STATEMENT_TIMEOUT_MS", 5000, 1, 60000)?,
            lock_timeout_ms: number("CUENORA_DB_LOCK_TIMEOUT_MS", 2000, 1, 60000)?,
        };
        if database.lock_timeout_ms > database.statement_timeout_ms {
            return Err(InvalidConfiguration("CUENORA_DB_LOCK_TIMEOUT_MS"));
        }
        let access = number("CUENORA_ACCESS_TOKEN_TTL_SECONDS", 900, 60, 3600)?;
        let refresh = number("CUENORA_REFRESH_TOKEN_TTL_SECONDS", 2592000, 3600, 7776000)?;
        let lifetimes = TokenLifetimes::new(access, refresh)
            .ok_or(InvalidConfiguration("CUENORA_REFRESH_TOKEN_TTL_SECONDS"))?;
        let key = get("CUENORA_RESEND_API_KEY")?;
        let from = get("CUENORA_EMAIL_FROM")?;
        let mail = match (key, from) {
            (None, None) => None,
            (Some(key), Some(from)) => Some(
                crate::verification_mail::ResendConfig::new(key, from).map_err(|_| {
                    InvalidConfiguration("CUENORA_RESEND_API_KEY/CUENORA_EMAIL_FROM")
                })?,
            ),
            _ => {
                return Err(InvalidConfiguration(
                    "CUENORA_RESEND_API_KEY/CUENORA_EMAIL_FROM",
                ));
            }
        };
        Ok(Self {
            mail,
            listen_addr,
            public_api_url,
            database_url,
            trusted_proxy_ips,
            database,
            lifetimes,
        })
    }
}

fn validate_public_url(raw: &str) -> Result<(), InvalidConfiguration> {
    // HTTP URI parsing rejects whitespace/control characters; an origin/base path
    // cannot contain credentials, queries or fragments. JSON quoting handles YAML.
    let uri = raw
        .parse::<axum::http::Uri>()
        .map_err(|_| InvalidConfiguration("CUENORA_PUBLIC_API_URL"))?;
    if !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.host().is_none()
        || uri.authority().is_some_and(|a| a.as_str().contains('@'))
        || raw.contains(['?', '#', '\\'])
        || !raw.is_ascii()
    {
        return Err(InvalidConfiguration("CUENORA_PUBLIC_API_URL"));
    }
    let authority = uri
        .authority()
        .ok_or(InvalidConfiguration("CUENORA_PUBLIC_API_URL"))?
        .as_str();
    let invalid = || InvalidConfiguration("CUENORA_PUBLIC_API_URL");
    let port = if authority.starts_with('[') {
        let end = authority.find(']').ok_or_else(invalid)?;
        authority[1..end]
            .parse::<std::net::Ipv6Addr>()
            .map_err(|_| invalid())?;
        let suffix = &authority[end + 1..];
        if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':').ok_or_else(invalid)?)
        }
    } else {
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(host, port)| (host, Some(port)));
        if host.is_empty() || idna::domain_to_ascii_strict(host).is_err() {
            return Err(invalid());
        }
        port
    };
    if let Some(port) = port
        && (port.is_empty()
            || !port.bytes().all(|b| b.is_ascii_digit())
            || port.parse::<u16>().ok().filter(|p| *p > 0).is_none())
    {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(values: &[(&str, &str)]) -> Result<AppConfig, InvalidConfiguration> {
        AppConfig::from_lookup(|key| {
            Ok(values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, v)| v.to_string()))
        })
    }
    #[test]
    fn environment_defaults_overrides_and_safe_rejections() {
        let c = config(&[]).unwrap();
        assert_eq!(c.listen_addr.to_string(), "127.0.0.1:3000");
        assert_eq!(c.public_api_url, "http://127.0.0.1:3000");
        assert!(c.database_url.is_none());
        assert_eq!(c.lifetimes.access_seconds(), 900);
        assert_eq!(c.lifetimes.refresh_seconds(), 2592000);
        assert_eq!(c.database.max_connections, 4);
        assert_eq!(c.database.acquire_timeout_ms, 3000);
        assert_eq!(c.database.statement_timeout_ms, 5000);
        assert_eq!(c.database.lock_timeout_ms, 2000);
        let c = config(&[
            ("CUENORA_LISTEN_ADDR", "0.0.0.0:4000"),
            ("CUENORA_PUBLIC_API_URL", "https://api.example.test/base"),
            ("CUENORA_ACCESS_TOKEN_TTL_SECONDS", "120"),
            ("CUENORA_REFRESH_TOKEN_TTL_SECONDS", "7200"),
            ("CUENORA_DB_MAX_CONNECTIONS", "8"),
            ("CUENORA_DB_ACQUIRE_TIMEOUT_MS", "200"),
            ("CUENORA_DB_STATEMENT_TIMEOUT_MS", "100"),
            ("CUENORA_DB_LOCK_TIMEOUT_MS", "50"),
        ])
        .unwrap();
        assert_eq!(c.listen_addr.to_string(), "0.0.0.0:4000");
        assert_eq!(c.lifetimes.access_seconds(), 120);
        assert_eq!(c.lifetimes.refresh_seconds(), 7200);
        assert_eq!(c.database.max_connections, 8);
        assert_eq!(c.database.acquire_timeout_ms, 200);
        assert_eq!(c.database.statement_timeout_ms, 100);
        assert_eq!(c.database.lock_timeout_ms, 50);
        for key in [
            "CUENORA_LISTEN_ADDR",
            "CUENORA_PUBLIC_API_URL",
            "CUENORA_DATABASE_URL",
            "CUENORA_TRUSTED_PROXY_IPS",
            "CUENORA_ACCESS_TOKEN_TTL_SECONDS",
            "CUENORA_REFRESH_TOKEN_TTL_SECONDS",
            "CUENORA_DB_MAX_CONNECTIONS",
            "CUENORA_DB_ACQUIRE_TIMEOUT_MS",
            "CUENORA_DB_STATEMENT_TIMEOUT_MS",
            "CUENORA_DB_LOCK_TIMEOUT_MS",
        ] {
            for invalid in ["", "secret-marker", "-1", " 10", "18446744073709551616"] {
                let error = config(&[(key, invalid)]).err().unwrap();
                assert_eq!(error.0, key);
                assert!(!format!("{error:?}").contains("secret-marker"));
            }
        }
        for url in [
            "https://user:secret@example.test",
            "https://example.test/?secret=x",
            "https://example.test/#x",
            "ftp://example.test",
        ] {
            assert!(config(&[("CUENORA_PUBLIC_API_URL", url)]).is_err());
        }
        assert!(
            config(&[
                ("CUENORA_ACCESS_TOKEN_TTL_SECONDS", "3600"),
                ("CUENORA_REFRESH_TOKEN_TTL_SECONDS", "3600")
            ])
            .is_err()
        );
        assert!(config(&[("CUENORA_DB_LOCK_TIMEOUT_MS", "5001")]).is_err());
    }
    #[test]
    fn malformed_public_authorities_are_rejected() {
        for url in [
            "https://api.example.test:99999",
            "https://api.example.test:abc",
            "https://api.example.test:-1",
            "https://:443/foo",
            "https://[]/base",
            "https://[notipv6]/base",
            "https://api.example.test:",
            "https://api.example.test:0",
        ] {
            assert!(
                config(&[("CUENORA_PUBLIC_API_URL", url)]).is_err(),
                "accepted {url}"
            );
        }
        for url in [
            "https://api.example.test:443/base",
            "http://127.0.0.1:3000",
            "https://[::1]:443/base",
            "https://api.example.test/base's",
        ] {
            assert!(
                config(&[("CUENORA_PUBLIC_API_URL", url)]).is_ok(),
                "rejected {url}"
            );
        }
    }
    #[test]
    fn resend_settings_are_optional_paired_and_never_echoed() {
        assert!(config(&[]).unwrap().mail.is_none());
        assert!(
            config(&[
                ("CUENORA_RESEND_API_KEY", "re_synthetic_key"),
                ("CUENORA_EMAIL_FROM", "verify@example.test")
            ])
            .unwrap()
            .mail
            .is_some()
        );
        for values in [
            vec![("CUENORA_RESEND_API_KEY", "re_synthetic_key")],
            vec![("CUENORA_EMAIL_FROM", "verify@example.test")],
            vec![
                ("CUENORA_RESEND_API_KEY", "secret marker"),
                ("CUENORA_EMAIL_FROM", "verify@example.test"),
            ],
            vec![
                ("CUENORA_RESEND_API_KEY", "re_synthetic_key"),
                ("CUENORA_EMAIL_FROM", "bad\r\nBcc: injected"),
            ],
        ] {
            let error = config(&values).err().unwrap();
            assert_eq!(error.0, "CUENORA_RESEND_API_KEY/CUENORA_EMAIL_FROM");
            assert!(!format!("{error:?}").contains("secret marker"));
        }
    }
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
