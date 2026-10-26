//! Local pending registration; no sessions, verification mail or financial authority.
use crate::{
    registration_policy::{Email, hash_password, validate_password},
    registration_store::persist_registration,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
struct RegistrationState {
    pool: Option<PgPool>,
    admission: Arc<Semaphore>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrationRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

#[derive(Serialize)]
struct Accepted {
    status: &'static str,
}

pub fn app(pool: Option<PgPool>) -> Router {
    router(RegistrationState {
        pool,
        admission: Arc::new(Semaphore::new(2)),
    })
}

fn router(state: RegistrationState) -> Router {
    Router::new()
        .route("/health/live", get(|| async { "OK" }))
        .route(
            "/auth/register",
            post(register).layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .with_state(state)
}

fn error(status: StatusCode, message: &'static str) -> Response {
    (status, Json(ErrorBody { error: message })).into_response()
}

fn unavailable() -> Response {
    error(StatusCode::SERVICE_UNAVAILABLE, "registration_unavailable")
}

async fn register(
    State(state): State<RegistrationState>,
    payload: Result<Json<RegistrationRequest>, JsonRejection>,
) -> Response {
    let input = match payload {
        Ok(Json(input)) => input,
        Err(rejection) => {
            let status = match rejection.status() {
                StatusCode::PAYLOAD_TOO_LARGE => StatusCode::PAYLOAD_TOO_LARGE,
                StatusCode::UNSUPPORTED_MEDIA_TYPE => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                _ => StatusCode::BAD_REQUEST,
            };
            return error(status, "invalid_request");
        }
    };
    let Ok(email) = Email::parse(&input.email) else {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    if validate_password(&input.password).is_err() {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    }
    let Some(pool) = state.pool else {
        return unavailable();
    };
    let Ok(permit) = state.admission.try_acquire_owned() else {
        return unavailable();
    };
    // Blocking work owns admission, even if the HTTP future is cancelled. Keep the
    // returned permit through persistence too; do not build an unbounded DB queue.
    let Ok((Ok(hash), _permit)) =
        blocking_with_permit(permit, move || hash_password(&input.password)).await
    else {
        return unavailable();
    };
    if persist_registration(&pool, &email, &hash).await.is_err() {
        return unavailable();
    }
    (StatusCode::ACCEPTED, Json(Accepted { status: "accepted" })).into_response()
}

async fn blocking_with_permit<T: Send + 'static>(
    permit: tokio::sync::OwnedSemaphorePermit,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<(T, tokio::sync::OwnedSemaphorePermit), tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || (work(), permit)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{Semaphore, oneshot};

    #[tokio::test]
    async fn saturated_registration_fails_fast_without_blocking_liveness() {
        use axum::{
            body::{Body, to_bytes},
            http::{Request, header},
        };
        use tower::ServiceExt;
        let pool =
            crate::config::pool_from_url(Some("postgres://synthetic:secret@127.0.0.1:1/cuenora"))
                .unwrap()
                .unwrap();
        let state = RegistrationState {
            pool: Some(pool.clone()),
            admission: Arc::new(Semaphore::new(2)),
        };
        let first = state.admission.clone().try_acquire_owned().unwrap();
        let second = state.admission.clone().try_acquire_owned().unwrap();
        let router = router(state);
        let response = tokio::time::timeout(std::time::Duration::from_secs(1), router.clone().oneshot(
            Request::post("/auth/register").header(header::CONTENT_TYPE,"application/json")
                .body(Body::from(r#"{"email":"capacity@example.test","password":"violet fields through winter snow"}"#)).unwrap()
        )).await.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
            br#"{"error":"registration_unavailable"}"#
        );
        let response = router
            .oneshot(Request::get("/health/live").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
            b"OK"
        );
        drop((first, second));
        pool.close().await;
    }

    #[tokio::test]
    async fn cancellation_keeps_admission_until_actual_blocking_work_finishes() {
        let admission = Arc::new(Semaphore::new(1));
        let permit = admission.clone().try_acquire_owned().unwrap();
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (finished_tx, finished_rx) = oneshot::channel();
        let task = tokio::spawn(blocking_with_permit(permit, move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            let _ = finished_tx.send(());
        }));
        started_rx.await.unwrap(); // On a single-thread async runtime, proves CPU work is off executor.
        assert!(admission.clone().try_acquire_owned().is_err());
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let available_during_work = admission.available_permits();
        release_tx.send(()).unwrap(); // Always unblock before a possibly failing assertion.
        finished_rx.await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while admission.available_permits() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(available_during_work, 0);
        assert_eq!(admission.available_permits(), 1);
    }
}
