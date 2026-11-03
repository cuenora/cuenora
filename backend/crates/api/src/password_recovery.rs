//! Anonymous recovery admission; raw secrets live only in bounded volatile mail jobs.
use crate::{
    api_error::ApiError,
    auth_throttling::{Throttle, protect},
    password_recovery_store as store,
    registration_policy::{Email, hash_password, validate_password},
    request_id::RequestId,
    verification_mail::ResendSender,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{StatusCode, Uri},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::{
    collections::{HashMap, VecDeque, hash_map::RandomState},
    future::Future,
    hash::BuildHasher,
    pin::Pin,
    sync::{Arc, Mutex as SyncMutex},
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, Semaphore, mpsc};

/// Adapters must never log/persist raw tokens; notification carries no credential.
pub trait RecoveryTransport: Send + Sync {
    fn send<'a>(
        &'a self,
        email: &'a str,
        token: &'a str,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>>;
    fn notify_reset<'a>(
        &'a self,
        email: &'a str,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>>;
}
impl RecoveryTransport for ResendSender {
    fn send<'a>(
        &'a self,
        email: &'a str,
        token: &'a str,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async move { self.send_recovery(email, token, key).await.map_err(|_| ()) })
    }
    fn notify_reset<'a>(
        &'a self,
        email: &'a str,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async move { self.send_password_reset(email, key).await.map_err(|_| ()) })
    }
}
#[derive(Clone)]
struct RecoveryState {
    pool: Option<PgPool>,
    jobs: Option<mpsc::Sender<Email>>,
    admission: Arc<Semaphore>,
    mailboxes: Arc<Mailboxes>,
}
struct Mailboxes {
    hasher: RandomState,
    entries: SyncMutex<HashMap<u64, MailWindow>>,
}
struct MailWindow {
    sends: VecDeque<Instant>,
}
impl Mailboxes {
    fn new() -> Self {
        Self {
            hasher: RandomState::new(),
            entries: SyncMutex::new(HashMap::new()),
        }
    }
    // Known and unknown canonical identities share the exact same suppression path.
    fn admit(&self, email: &Email) -> Result<bool, ApiError> {
        let key = self.hasher.hash_one(email.address().to_ascii_lowercase());
        let now = Instant::now();
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| ApiError::ProtectionUnavailable)?;
        entries.retain(|_, window| {
            window
                .sends
                .retain(|sent| now.saturating_duration_since(*sent) < Duration::from_secs(3600));
            !window.sends.is_empty()
        });
        if let Some(window) = entries.get_mut(&key) {
            if window.sends.len() >= 5
                || window.sends.back().is_some_and(|last| {
                    now.saturating_duration_since(*last) < Duration::from_secs(60)
                })
            {
                return Ok(false);
            }
            window.sends.push_back(now);
        } else {
            if entries.len() >= 8192 {
                return Err(ApiError::ProtectionUnavailable);
            }
            entries.insert(
                key,
                MailWindow {
                    sends: VecDeque::from([now]),
                },
            );
        }
        Ok(true)
    }
}
pub(crate) fn routes(
    pool: Option<PgPool>,
    transport: Option<Arc<dyn RecoveryTransport>>,
    throttle: Arc<Throttle>,
) -> Router {
    let jobs = match (pool.clone(), transport) {
        (Some(pool), Some(transport)) => {
            let (sender, receiver) = mpsc::channel::<Email>(100);
            let receiver = Arc::new(Mutex::new(receiver));
            for _ in 0..2 {
                let (pool, transport, receiver) =
                    (pool.clone(), transport.clone(), receiver.clone());
                tokio::spawn(async move {
                    loop {
                        let email = receiver.lock().await.recv().await;
                        let Some(email) = email else {
                            break;
                        };
                        if let Ok(Some(message)) = store::issue(&pool, &email).await {
                            let _ = tokio::time::timeout(
                                Duration::from_secs(60),
                                transport.send(
                                    &message.email,
                                    &message.token,
                                    &message.idempotency_key,
                                ),
                            )
                            .await;
                        }
                    }
                });
            }
            tokio::spawn(async move {
                loop {
                    // Atomic leased claims are safe across API processes. Events survive restart.
                    if let Ok(Some(event)) = store::claim_notification(&pool).await {
                        let delivered = matches!(
                            tokio::time::timeout(
                                Duration::from_secs(60),
                                transport.notify_reset(&event.email, &event.idempotency_key)
                            )
                            .await,
                            Ok(Ok(()))
                        );
                        let _ =
                            store::finish_notification(&pool, &event.id, event.attempt, delivered)
                                .await;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
            Some(sender)
        }
        _ => None,
    };
    Router::new()
        .route("/v1/auth/password-recovery/request", post(request))
        .route("/v1/auth/password-recovery/confirm", post(confirm))
        .route_layer(DefaultBodyLimit::max(8192))
        .route_layer(middleware::from_fn_with_state(throttle, protect))
        .layer(middleware::from_fn(
            |request: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(request).await;
                response.headers_mut().insert(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-store"),
                );
                response
            },
        ))
        .with_state(RecoveryState {
            pool,
            jobs,
            admission: Arc::new(Semaphore::new(2)),
            mailboxes: Arc::new(Mailboxes::new()),
        })
}
struct Object<T>(T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = Object<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(Object)
            }
        }
        deserializer.deserialize_map(Visitor(std::marker::PhantomData))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestInput {
    email: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirmation {
    token: String,
    new_password: String,
}
fn accepted() -> Response {
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"status":"accepted"})),
    )
        .into_response()
}
async fn request(
    State(state): State<RecoveryState>,
    Extension(id): Extension<RequestId>,
    uri: Uri,
    payload: Result<Json<Object<RequestInput>>, JsonRejection>,
) -> Response {
    if uri.query().is_some() {
        return ApiError::InvalidRequest.response(&id);
    }
    let input = match payload {
        Ok(Json(Object(input))) => input,
        Err(e) => return ApiError::invalid_json(e.status()).response(&id),
    };
    let email = match Email::parse(&input.email) {
        Ok(e) => e,
        Err(_) => return ApiError::InvalidRequest.response(&id),
    };
    let Some(jobs) = &state.jobs else {
        return ApiError::PasswordRecoveryUnavailable.response(&id);
    };
    // Capacity is reserved before any identity lookup or mailbox suppression.
    let slot = match jobs.try_reserve() {
        Ok(s) => s,
        Err(_) => return ApiError::PasswordRecoveryUnavailable.response(&id),
    };
    match state.mailboxes.admit(&email) {
        Ok(true) => {
            slot.send(email);
            accepted()
        }
        Ok(false) => accepted(),
        Err(e) => e.response(&id),
    }
}
async fn confirm(
    State(state): State<RecoveryState>,
    Extension(id): Extension<RequestId>,
    uri: Uri,
    payload: Result<Json<Object<Confirmation>>, JsonRejection>,
) -> Response {
    if uri.query().is_some() {
        return ApiError::InvalidRequest.response(&id);
    }
    let input = match payload {
        Ok(Json(Object(input))) => input,
        Err(e) => return ApiError::invalid_json(e.status()).response(&id),
    };
    if !store::valid_token(&input.token) {
        return ApiError::InvalidRecoveryChallenge.response(&id);
    }
    if validate_password(&input.new_password).is_err() {
        return ApiError::InvalidRequest.response(&id);
    }
    let Some(pool) = &state.pool else {
        return ApiError::PasswordRecoveryUnavailable.response(&id);
    };
    let Ok(permit) = state.admission.try_acquire_owned() else {
        return ApiError::PasswordRecoveryUnavailable.response(&id);
    };
    let password = input.new_password;
    // Keep the permit inside blocking work on cancellation and through commit on success.
    let Ok((hash, _permit)) =
        tokio::task::spawn_blocking(move || (hash_password(&password), permit)).await
    else {
        return ApiError::PasswordRecoveryUnavailable.response(&id);
    };
    let hash = match hash {
        Ok(hash) => hash,
        Err(_) => return ApiError::PasswordRecoveryUnavailable.response(&id),
    };
    match store::confirm(pool, &input.token, &hash).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(store::StoreError::InvalidChallenge) => {
            ApiError::InvalidRecoveryChallenge.response(&id)
        }
        Err(store::StoreError::Unavailable) => ApiError::PasswordRecoveryUnavailable.response(&id),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mailbox_hour_boundary_does_not_bypass_last_send_cooldown() {
        let mailboxes = Mailboxes::new();
        let email = Email::parse("boundary@example.test").unwrap();
        let key = mailboxes
            .hasher
            .hash_one(email.address().to_ascii_lowercase());
        let now = Instant::now();
        mailboxes.entries.lock().unwrap().insert(
            key,
            MailWindow {
                sends: [3600, 240, 180, 120, 1]
                    .map(|seconds| now - Duration::from_secs(seconds))
                    .into(),
            },
        );
        assert!(!mailboxes.admit(&email).unwrap());
    }
    #[test]
    fn mailbox_rolling_quota_reclaims_only_expired_slots() {
        let mailboxes = Mailboxes::new();
        let email = Email::parse("quota@example.test").unwrap();
        let key = mailboxes
            .hasher
            .hash_one(email.address().to_ascii_lowercase());
        let now = Instant::now();
        mailboxes.entries.lock().unwrap().insert(
            key,
            MailWindow {
                sends: [3599, 240, 180, 120, 61]
                    .map(|seconds| now - Duration::from_secs(seconds))
                    .into(),
            },
        );
        assert!(!mailboxes.admit(&email).unwrap());
        mailboxes
            .entries
            .lock()
            .unwrap()
            .get_mut(&key)
            .unwrap()
            .sends[0] = now - Duration::from_secs(3601);
        assert!(mailboxes.admit(&email).unwrap());
        assert!(!mailboxes.admit(&email).unwrap());
        let mut entries = mailboxes.entries.lock().unwrap();
        entries.clear();
        for candidate in (0..).filter(|candidate| *candidate != key).take(8192) {
            entries.insert(
                candidate,
                MailWindow {
                    sends: VecDeque::from([now]),
                },
            );
        }
        drop(entries);
        assert!(matches!(
            mailboxes.admit(&email),
            Err(ApiError::ProtectionUnavailable)
        ));
    }
    #[test]
    fn canonical_mailbox_suppression_is_separate_from_login_and_bounded() {
        let mailboxes = Mailboxes::new();
        let a = Email::parse("Case+tag@EXAMPLE.test").unwrap();
        let b = Email::parse("case+tag@example.test").unwrap();
        assert!(mailboxes.admit(&a).unwrap());
        assert!(!mailboxes.admit(&b).unwrap());
        assert!(
            mailboxes
                .admit(&Email::parse("case@example.test").unwrap())
                .unwrap()
        );
    }
}
