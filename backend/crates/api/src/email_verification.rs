//! Session-bound verification with a bounded, volatile mail queue.
use crate::{
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    auth_throttling::{Throttle, protect},
    email_verification_store::{self as store, StoreError},
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
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::{Mutex, Semaphore, mpsc};

/// Test adapters implement the same boundary without a live provider or real mail.
/// Implementations must not persist or log raw tokens and must bound delivery time.
pub trait VerificationTransport: Send + Sync {
    fn send<'a>(
        &'a self,
        email: &'a str,
        token: &'a str,
        idempotency_key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>>;
}
impl VerificationTransport for ResendSender {
    fn send<'a>(
        &'a self,
        email: &'a str,
        token: &'a str,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async move {
            ResendSender::send(self, email, token, key)
                .await
                .map_err(|_| ())
        })
    }
}

#[derive(Clone)]
struct VerificationState {
    pool: Option<PgPool>,
    jobs: Option<mpsc::Sender<AuthenticatedIdentity>>,
    confirmation_slots: Arc<Semaphore>,
}

pub(crate) fn routes(
    pool: Option<PgPool>,
    transport: Option<Arc<dyn VerificationTransport>>,
    throttle: Arc<Throttle>,
) -> Router {
    let jobs = match (pool.clone(), transport) {
        (Some(pool), Some(transport)) => {
            let (sender, receiver) = mpsc::channel::<AuthenticatedIdentity>(100);
            let receiver = Arc::new(Mutex::new(receiver));
            for _ in 0..2 {
                let (pool, transport, receiver) =
                    (pool.clone(), transport.clone(), receiver.clone());
                tokio::spawn(async move {
                    loop {
                        let identity = receiver.lock().await.recv().await;
                        let Some(identity) = identity else {
                            break;
                        };
                        if let Ok(Some(message)) = store::issue(&pool, &identity).await {
                            // Provider acceptance is not delivery or verification. Never log payloads.
                            let _ = tokio::time::timeout(
                                std::time::Duration::from_secs(60),
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
            Some(sender)
        }
        _ => None,
    };
    Router::new()
        .route("/v1/auth/email-verification/request", post(request))
        .route("/v1/auth/email-verification/confirm", post(confirm))
        .route_layer(DefaultBodyLimit::max(8192))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ))
        .route_layer(middleware::from_fn_with_state(throttle, protect))
        .with_state(VerificationState {
            pool,
            jobs,
            confirmation_slots: Arc::new(Semaphore::new(2)),
        })
}

// Serde structs also accept JSON sequences by default. This wrapper accepts only
// maps while preserving duplicate-field rejection in the inner derived type.
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
struct Empty {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirmation {
    token: String,
}

fn error(error: StoreError) -> ApiError {
    match error {
        StoreError::InvalidChallenge => ApiError::InvalidChallenge,
        StoreError::InvalidSession => ApiError::InvalidAccessToken { missing: false },
        StoreError::Unavailable => ApiError::EmailVerificationUnavailable,
    }
}
async fn request(
    State(state): State<VerificationState>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    uri: Uri,
    payload: Result<Json<Object<Empty>>, JsonRejection>,
) -> Response {
    if uri.query().is_some() {
        return ApiError::InvalidRequest.response(&id);
    }
    if let Err(rejection) = payload {
        return ApiError::invalid_json(rejection.status()).response(&id);
    }
    let Some(jobs) = &state.jobs else {
        return ApiError::EmailVerificationUnavailable.response(&id);
    };
    match jobs.try_reserve() {
        Ok(slot) => {
            slot.send(identity);
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({"status":"accepted"})),
            )
                .into_response()
        }
        Err(_) => ApiError::EmailVerificationUnavailable.response(&id),
    }
}
async fn confirm(
    State(state): State<VerificationState>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    uri: Uri,
    payload: Result<Json<Object<Confirmation>>, JsonRejection>,
) -> Response {
    if uri.query().is_some() {
        return ApiError::InvalidRequest.response(&id);
    }
    let input = match payload {
        Ok(Json(Object(input))) => input,
        Err(rejection) => return ApiError::invalid_json(rejection.status()).response(&id),
    };
    let Ok(_permit) = state.confirmation_slots.try_acquire() else {
        return ApiError::EmailVerificationUnavailable.response(&id);
    };
    let Some(pool) = &state.pool else {
        return ApiError::EmailVerificationUnavailable.response(&id);
    };
    match store::confirm(pool, &identity, &input.token).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(failure) => error(failure).response(&id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verification_payloads_require_objects_and_reject_duplicates() {
        for input in ["[]", "null", "[{}]", "{\"user_id\":\"x\"}"] {
            assert!(serde_json::from_str::<Object<Empty>>(input).is_err());
        }
        for input in ["[\"token\"]", "null", "{\"token\":\"a\",\"token\":\"b\"}"] {
            assert!(serde_json::from_str::<Object<Confirmation>>(input).is_err());
        }
        assert!(serde_json::from_str::<Object<Empty>>("{}").is_ok());
        assert!(serde_json::from_str::<Object<Confirmation>>("{\"token\":\"test\"}").is_ok());
    }
}
