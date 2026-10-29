//! Bounded, process-local abuse admission. Never logs client identifiers.
use crate::{api_error::ApiError, registration_policy::Email, request_id::RequestId};
use axum::{
    Extension,
    extract::{ConnectInfo, Request, State},
    http::{Method, header},
    middleware::Next,
    response::Response,
};
use std::{
    collections::{HashMap, hash_map::RandomState},
    hash::BuildHasher,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_KEYS: usize = 8192;

/// No implicit private-network or forwarded-header trust.
#[derive(Clone, Default)]
pub struct ClientIpPolicy {
    trusted_peers: Vec<IpAddr>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidProxyConfiguration;

impl ClientIpPolicy {
    /// None means direct TCP clients; Some requires exact immediate proxy IPs.
    pub fn from_trusted_proxy_ips(value: Option<&str>) -> Result<Self, InvalidProxyConfiguration> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        if value.len() > 2048 {
            return Err(InvalidProxyConfiguration);
        }
        let peers: Result<Vec<IpAddr>, _> = value
            .split(',')
            .take(33)
            .map(|s| s.trim().parse::<IpAddr>().map(normalize))
            .collect();
        let trusted_peers = peers.map_err(|_| InvalidProxyConfiguration)?;
        if trusted_peers.is_empty()
            || trusted_peers.len() > 32
            || trusted_peers
                .iter()
                .any(|ip| ip.is_unspecified() || ip.is_multicast())
        {
            return Err(InvalidProxyConfiguration);
        }
        Ok(Self { trusted_peers })
    }

    fn client(&self, request: &Request) -> Result<IpAddr, ApiError> {
        let peer = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or(ApiError::ProtectionUnavailable)?;
        let peer = normalize(peer.0.ip());
        if self.trusted_peers.is_empty() {
            return Ok(peer);
        }
        if !self.trusted_peers.contains(&peer) {
            return Err(ApiError::ProtectionUnavailable);
        }
        let mut values = request.headers().get_all("cf-connecting-ip").iter();
        let value = values.next().ok_or(ApiError::ProtectionUnavailable)?;
        if values.next().is_some() {
            return Err(ApiError::ProtectionUnavailable);
        }
        let value = value
            .to_str()
            .map_err(|_| ApiError::ProtectionUnavailable)?;
        let ip = value
            .parse::<IpAddr>()
            .map_err(|_| ApiError::ProtectionUnavailable)?;
        if ip.is_unspecified() || ip.is_multicast() {
            return Err(ApiError::ProtectionUnavailable);
        }
        Ok(normalize(ip))
    }
}

fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
    }
}

fn client_key(ip: IpAddr) -> IpAddr {
    match normalize(ip) {
        IpAddr::V6(v6) => IpAddr::V6(Ipv6Addr::from(u128::from(v6) & (u128::MAX << 64))),
        ip => ip,
    }
}

#[derive(Hash, PartialEq, Eq)]
enum Key {
    Client(IpAddr, u8),
    Login(u64),
    Reauthentication(u64),
}

struct Window {
    started: Instant,
    attempts: u32,
}

struct Windows {
    entries: HashMap<Key, Window>,
    last_cleanup: Instant,
}

impl Windows {
    fn admit(&mut self, key: Key, budget: u32, now: Instant) -> Result<(), ApiError> {
        if now.saturating_duration_since(self.last_cleanup) >= Duration::from_secs(1) {
            self.entries
                .retain(|_, w| now.saturating_duration_since(w.started) < WINDOW);
            self.last_cleanup = now;
        }
        if !self.entries.contains_key(&key) && self.entries.len() >= MAX_KEYS {
            return Err(ApiError::ProtectionUnavailable);
        }
        let window = self.entries.entry(key).or_insert(Window {
            started: now,
            attempts: 0,
        });
        let elapsed = now.saturating_duration_since(window.started);
        if elapsed >= WINDOW {
            *window = Window {
                started: now,
                attempts: 0,
            };
        }
        if window.attempts >= budget {
            let remaining = WINDOW.saturating_sub(now.saturating_duration_since(window.started));
            let seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0);
            return Err(ApiError::RateLimited {
                retry_after: seconds.max(1),
            });
        }
        window.attempts += 1;
        Ok(())
    }
}

pub(crate) struct Throttle {
    policy: ClientIpPolicy,
    windows: Mutex<Windows>,
    // Only a process-keyed hash is retained; this is not a password hash or an
    // anonymity guarantee. Hash collisions conservatively share a budget.
    email_hasher: RandomState,
}

impl Throttle {
    pub(crate) fn new(policy: ClientIpPolicy) -> Self {
        Self {
            policy,
            windows: Mutex::new(Windows {
                entries: HashMap::new(),
                last_cleanup: Instant::now(),
            }),
            email_hasher: RandomState::new(),
        }
    }

    fn admit(&self, key: Key, budget: u32) -> Result<(), ApiError> {
        self.windows
            .lock()
            .map_err(|_| ApiError::ProtectionUnavailable)?
            .admit(key, budget, Instant::now())
    }

    pub(crate) fn reauthenticate(&self, user_id: &str) -> Result<(), ApiError> {
        self.admit(
            Key::Reauthentication(self.email_hasher.hash_one(user_id)),
            10,
        )
    }

    pub(crate) fn login(&self, email: &Email) -> Result<(), ApiError> {
        let hash = self
            .email_hasher
            .hash_one(email.address().to_ascii_lowercase());
        self.admit(Key::Login(hash), 10)
    }
}

pub(crate) async fn protect(
    State(throttle): State<Arc<Throttle>>,
    Extension(id): Extension<RequestId>,
    request: Request,
    next: Next,
) -> Response {
    let (route, budget) = match (request.method(), request.uri().path()) {
        (&Method::POST, "/v1/auth/register") => (0, 5),
        (&Method::POST, "/v1/auth/login") => (1, 20),
        (&Method::POST, "/v1/auth/refresh") => (2, 60),
        (&Method::POST, "/v1/auth/logout") => (3, 60),
        (&Method::POST, "/v1/auth/logout-all") => (4, 20),
        _ => return next.run(request).await,
    };
    let result = throttle
        .policy
        .client(&request)
        .and_then(|ip| throttle.admit(Key::Client(client_key(ip), route), budget));
    match result {
        Ok(()) => next.run(request).await,
        Err(error) => {
            let mut response = error.response(&id);
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            );
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_expiry_retry_rounding_and_bounded_reclamation() {
        let now = Instant::now();
        let mut state = Windows {
            entries: HashMap::new(),
            last_cleanup: now,
        };
        state.admit(Key::Login(0), 1, now).unwrap();
        assert!(matches!(
            state.admit(Key::Login(0), 1, now + Duration::from_millis(1500)),
            Err(ApiError::RateLimited { retry_after: 59 })
        ));
        assert!(matches!(
            state.admit(Key::Login(0), 1, now + Duration::from_secs(59)),
            Err(ApiError::RateLimited { retry_after: 1 })
        ));
        for n in 1..MAX_KEYS as u64 {
            state.admit(Key::Login(n), 1, now).unwrap();
        }
        assert!(matches!(
            state.admit(Key::Login(MAX_KEYS as u64), 1, now),
            Err(ApiError::ProtectionUnavailable)
        ));
        assert_eq!(state.entries.len(), MAX_KEYS);
        // A full store must not evict an active, already-exhausted identity.
        assert!(matches!(
            state.admit(Key::Login(0), 1, now),
            Err(ApiError::RateLimited { .. })
        ));
        state
            .admit(Key::Login(MAX_KEYS as u64), 1, now + WINDOW)
            .unwrap();
        assert_eq!(state.entries.len(), 1);
        state.admit(Key::Login(0), 1, now + WINDOW).unwrap();
    }

    #[test]
    fn trusted_proxy_configuration_is_explicit_bounded_and_safe() {
        for invalid in [
            "",
            " ",
            "0.0.0.0/0",
            "0.0.0.0",
            "::",
            "224.0.0.1",
            "bad secret",
            "127.0.0.1,",
            "*",
        ] {
            assert_eq!(
                ClientIpPolicy::from_trusted_proxy_ips(Some(invalid)).err(),
                Some(InvalidProxyConfiguration)
            );
        }
        assert!(
            ClientIpPolicy::from_trusted_proxy_ips(Some(&vec!["127.0.0.1"; 33].join(","))).is_err()
        );
        assert!(ClientIpPolicy::from_trusted_proxy_ips(Some("127.0.0.1, ::1")).is_ok());
        assert!(ClientIpPolicy::from_trusted_proxy_ips(None).is_ok());
        assert_eq!(
            format!("{InvalidProxyConfiguration:?}"),
            "InvalidProxyConfiguration"
        );
    }

    #[test]
    fn poisoning_fails_closed() {
        let throttle = Arc::new(Throttle::new(ClientIpPolicy::default()));
        let copy = throttle.clone();
        assert!(
            std::thread::spawn(move || {
                let _guard = copy.windows.lock().unwrap();
                panic!("synthetic poison");
            })
            .join()
            .is_err()
        );
        assert!(matches!(
            throttle.login(&Email::parse("test@example.test").unwrap()),
            Err(ApiError::ProtectionUnavailable)
        ));
    }
}
