//! Registration, opaque login and refresh; no verification mail or financial authority.
use crate::{
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    auth_throttling::{ClientIpPolicy, Throttle, protect},
    login_credentials::verify_password,
    login_store::{
        issue_initial_session, lookup_credentials, lookup_identity_credentials, revoke_all_sessions,
    },
    registration_policy::{Email, hash_password, validate_password},
    registration_store::persist_registration,
    request_id::{RequestId, with_request_context},
    session_store::{revoke, rotate},
    session_tokens::{ACCESS_TOKEN_LIFETIME_SECONDS, REFRESH_TOKEN_LIFETIME_SECONDS, generate},
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
struct AuthState {
    pool: Option<PgPool>,
    admission: Arc<Semaphore>,
    refresh_admission: Arc<Semaphore>,
    throttle: Arc<Throttle>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrationRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalLogoutRequest {
    password: String,
}

#[derive(Serialize)]
struct Accepted {
    status: &'static str,
}

#[derive(Serialize)]
struct LoginResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: u64,
    refresh_expires_in: u64,
}

pub fn app(pool: Option<PgPool>) -> Router {
    app_with_client_policy(pool, ClientIpPolicy::default())
}

/// One shared limiter per app construction. Serving must attach TCP ConnectInfo.
pub fn app_with_client_policy(pool: Option<PgPool>, policy: ClientIpPolicy) -> Router {
    router(AuthState {
        pool,
        admission: Arc::new(Semaphore::new(2)),
        refresh_admission: Arc::new(Semaphore::new(2)),
        throttle: Arc::new(Throttle::new(policy)),
    })
}

fn router(state: AuthState) -> Router {
    let accounts = crate::account_http::routes(state.pool.clone());
    let profile = crate::profile_zone::routes(state.pool.clone());
    let router = Router::new()
        .route("/v1/health/live", get(|| async { "OK" }))
        .route(
            "/v1/auth/register",
            post(register).layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .route(
            "/v1/auth/login",
            post(login).layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .route(
            "/v1/auth/refresh",
            post(refresh).layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .route(
            "/v1/auth/logout",
            post(logout).layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .route(
            "/v1/auth/logout-all",
            post(logout_all)
                .layer(DefaultBodyLimit::max(8 * 1024))
                .layer(middleware::from_fn_with_state(
                    state.pool.clone(),
                    require_identity,
                )),
        )
        .route_layer(middleware::from_fn_with_state(
            state.throttle.clone(),
            protect,
        ))
        .fallback(
            |Extension(id): Extension<RequestId>| async move { ApiError::NotFound.response(&id) },
        )
        .method_not_allowed_fallback(|Extension(id): Extension<RequestId>| async move {
            ApiError::MethodNotAllowed.response(&id)
        })
        .with_state(state);
    with_request_context(router.merge(accounts).merge(profile))
}

async fn logout(
    State(state): State<AuthState>,
    Extension(id): Extension<RequestId>,
    payload: Result<Json<RefreshRequest>, JsonRejection>,
) -> Response {
    let mut response = logout_response(state, &id, payload).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn logout_response(
    state: AuthState,
    id: &RequestId,
    payload: Result<Json<RefreshRequest>, JsonRejection>,
) -> Response {
    let input = match payload {
        Ok(Json(input)) => input,
        Err(error) => return ApiError::invalid_json(error.status()).response(id),
    };
    let Ok(_permit) = state.refresh_admission.try_acquire_owned() else {
        return ApiError::AuthenticationUnavailable.response(id);
    };
    let Some(pool) = state.pool else {
        return ApiError::AuthenticationUnavailable.response(id);
    };
    match revoke(&pool, &input.refresh_token).await {
        // Generic success conceals whether a refresh credential was retained.
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => ApiError::AuthenticationUnavailable.response(id),
    }
}

async fn logout_all(
    State(state): State<AuthState>,
    Extension(id): Extension<RequestId>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    payload: Result<Json<GlobalLogoutRequest>, JsonRejection>,
) -> Response {
    let input = match payload {
        Ok(Json(input)) => input,
        Err(error) => return ApiError::invalid_json(error.status()).response(&id),
    };
    if input.password.len() > 512 || input.password.chars().count() > 128 {
        return ApiError::InvalidRequest.response(&id);
    }
    if let Err(error) = state.throttle.reauthenticate(identity.user_id()) {
        return error.response(&id);
    }
    let Ok(permit) = state.admission.try_acquire_owned() else {
        return ApiError::AuthenticationUnavailable.response(&id);
    };
    let Some(pool) = state.pool else {
        return ApiError::AuthenticationUnavailable.response(&id);
    };
    let credential = match lookup_identity_credentials(&pool, &identity).await {
        Ok(value) => value,
        Err(_) => return ApiError::AuthenticationUnavailable.response(&id),
    };
    let password = input.password;
    let Ok(((verified, credential), _permit)) = blocking_with_permit(permit, move || {
        let verified = verify_password(&password, credential.as_ref().map(|c| c.encoded_hash()));
        (verified, credential)
    })
    .await
    else {
        return ApiError::AuthenticationUnavailable.response(&id);
    };
    match verified {
        Ok(true) => {}
        Ok(false) => return ApiError::InvalidCredentials.response(&id),
        Err(_) => return ApiError::AuthenticationUnavailable.response(&id),
    }
    let Some(credential) = credential else {
        return ApiError::InvalidCredentials.response(&id);
    };
    match revoke_all_sessions(&pool, &credential, &identity).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => ApiError::InvalidCredentials.response(&id),
        Err(_) => ApiError::AuthenticationUnavailable.response(&id),
    }
}

async fn refresh(
    State(state): State<AuthState>,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<RefreshRequest>, JsonRejection>,
) -> Response {
    let mut response = refresh_response(state, &request_id, payload).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn refresh_response(
    state: AuthState,
    request_id: &RequestId,
    payload: Result<Json<RefreshRequest>, JsonRejection>,
) -> Response {
    let input = match payload {
        Ok(Json(input)) => input,
        Err(rejection) => {
            return ApiError::invalid_json(rejection.status()).response(request_id);
        }
    };
    // Refresh work must not take the registration/login password-hashing slots.
    // Hold its own admission through persistence, including replay revocation.
    let Ok(_permit) = state.refresh_admission.try_acquire_owned() else {
        return ApiError::RefreshUnavailable.response(request_id);
    };
    let Some(pool) = state.pool else {
        return ApiError::RefreshUnavailable.response(request_id);
    };
    let tokens = match rotate(&pool, &input.refresh_token).await {
        Ok(Some(tokens)) => tokens,
        Ok(None) => return ApiError::InvalidRefreshToken.response(request_id),
        Err(_) => return ApiError::RefreshUnavailable.response(request_id),
    };
    (
        StatusCode::OK,
        Json(LoginResponse {
            access_token: tokens.access.expose_secret().to_owned(),
            refresh_token: tokens.refresh.expose_secret().to_owned(),
            token_type: "Bearer",
            expires_in: ACCESS_TOKEN_LIFETIME_SECONDS,
            refresh_expires_in: REFRESH_TOKEN_LIFETIME_SECONDS,
        }),
    )
        .into_response()
}

fn registration_unavailable(request_id: &RequestId) -> Response {
    ApiError::RegistrationUnavailable.response(request_id)
}

fn login_unavailable(request_id: &RequestId) -> Response {
    ApiError::LoginUnavailable.response(request_id)
}

async fn register(
    State(state): State<AuthState>,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<RegistrationRequest>, JsonRejection>,
) -> Response {
    let input = match payload {
        Ok(Json(input)) => input,
        Err(rejection) => {
            return ApiError::invalid_json(rejection.status()).response(&request_id);
        }
    };
    let Ok(email) = Email::parse(&input.email) else {
        return ApiError::InvalidRequest.response(&request_id);
    };
    if validate_password(&input.password).is_err() {
        return ApiError::InvalidRequest.response(&request_id);
    }
    let Some(pool) = state.pool else {
        return registration_unavailable(&request_id);
    };
    let Ok(permit) = state.admission.try_acquire_owned() else {
        return registration_unavailable(&request_id);
    };
    // Blocking work owns admission, even if the HTTP future is cancelled. Keep the
    // returned permit through persistence too; do not build an unbounded DB queue.
    let Ok((Ok(hash), _permit)) =
        blocking_with_permit(permit, move || hash_password(&input.password)).await
    else {
        return registration_unavailable(&request_id);
    };
    if persist_registration(&pool, &email, &hash).await.is_err() {
        return registration_unavailable(&request_id);
    }
    (StatusCode::ACCEPTED, Json(Accepted { status: "accepted" })).into_response()
}

async fn login(
    State(state): State<AuthState>,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Response {
    let input = match payload {
        Ok(Json(input)) => input,
        Err(rejection) => {
            return ApiError::invalid_json(rejection.status()).response(&request_id);
        }
    };
    let Ok(email) = Email::parse(&input.email) else {
        return ApiError::InvalidRequest.response(&request_id);
    };
    if input.password.len() > 512 || input.password.chars().count() > 128 {
        return ApiError::InvalidRequest.response(&request_id);
    }
    if let Err(error) = state.throttle.login(&email) {
        return error.response(&request_id);
    }
    let Ok(permit) = state.admission.clone().try_acquire_owned() else {
        return login_unavailable(&request_id);
    };
    let Some(pool) = state.pool else {
        return login_unavailable(&request_id);
    };
    let credential = match lookup_credentials(&pool, &email).await {
        Ok(credential) => credential,
        Err(_) => return login_unavailable(&request_id),
    };
    let password = input.password;
    let Ok(((verified, credential), permit)) = blocking_with_permit(permit, move || {
        let verified = verify_password(
            &password,
            credential.as_ref().map(|snapshot| snapshot.encoded_hash()),
        );
        (verified, credential)
    })
    .await
    else {
        return login_unavailable(&request_id);
    };
    let verified = match verified {
        Ok(verified) => verified,
        Err(_) => return login_unavailable(&request_id),
    };
    if !verified {
        return ApiError::InvalidCredentials.response(&request_id);
    }
    let Some(credential) = credential else {
        return ApiError::InvalidCredentials.response(&request_id);
    };
    let tokens = match generate() {
        Ok(tokens) => tokens,
        Err(_) => return login_unavailable(&request_id),
    };
    match issue_initial_session(&pool, &credential, &tokens).await {
        Ok(true) => {}
        Ok(false) => return ApiError::InvalidCredentials.response(&request_id),
        Err(_) => return login_unavailable(&request_id),
    }
    drop(permit);
    let response = LoginResponse {
        access_token: tokens.access.expose_secret().to_owned(),
        refresh_token: tokens.refresh.expose_secret().to_owned(),
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_LIFETIME_SECONDS,
        refresh_expires_in: REFRESH_TOKEN_LIFETIME_SECONDS,
    };
    let mut response = (StatusCode::OK, Json(response)).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
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
        let state = AuthState {
            pool: Some(pool.clone()),
            admission: Arc::new(Semaphore::new(2)),
            refresh_admission: Arc::new(Semaphore::new(2)),
            throttle: Arc::new(Throttle::new(ClientIpPolicy::default())),
        };
        let first = state.admission.clone().try_acquire_owned().unwrap();
        let second = state.admission.clone().try_acquire_owned().unwrap();
        let router = router(state).layer(Extension(axum::extract::ConnectInfo(
            "192.0.2.1:1".parse::<std::net::SocketAddr>().unwrap(),
        )));
        let response = tokio::time::timeout(std::time::Duration::from_secs(1), router.clone().oneshot(
            Request::post("/v1/auth/register").header(header::CONTENT_TYPE,"application/json")
                .body(Body::from(r#"{"email":"capacity@example.test","password":"violet fields through winter snow"}"#)).unwrap()
        )).await.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"code":"registration_unavailable", "message":"Registration is temporarily unavailable.", "details":null, "request_id":id})
        );
        let response = router
            .oneshot(Request::get("/v1/health/live").body(Body::empty()).unwrap())
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
    async fn registration_and_login_share_capacity_with_route_specific_errors() {
        use axum::{
            body::{Body, to_bytes},
            http::{Request, header},
        };
        use tower::ServiceExt;
        let pool =
            crate::config::pool_from_url(Some("postgres://synthetic:secret@127.0.0.1:1/cuenora"))
                .unwrap()
                .unwrap();
        let state = AuthState {
            pool: Some(pool.clone()),
            admission: Arc::new(Semaphore::new(2)),
            refresh_admission: Arc::new(Semaphore::new(2)),
            throttle: Arc::new(Throttle::new(ClientIpPolicy::default())),
        };
        let first = state.admission.clone().try_acquire_owned().unwrap();
        let second = state.admission.clone().try_acquire_owned().unwrap();
        let router = router(state).layer(Extension(axum::extract::ConnectInfo(
            "192.0.2.1:1".parse::<std::net::SocketAddr>().unwrap(),
        )));
        for (path, message) in [
            ("/v1/auth/register", "registration_unavailable"),
            ("/v1/auth/login", "login_unavailable"),
        ] {
            let response = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                router.clone().oneshot(
                    Request::post(path)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(r#"{"email":"capacity@example.test","password":"violet fields through winter snow"}"#))
                        .unwrap(),
                ),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            let id = response.headers()["x-request-id"]
                .to_str()
                .unwrap()
                .to_owned();
            let body: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap())
                    .unwrap();
            let text = if path == "/v1/auth/register" {
                "Registration is temporarily unavailable."
            } else {
                "Login is temporarily unavailable."
            };
            assert_eq!(
                body,
                serde_json::json!({"code":message, "message":text, "details":null, "request_id":id})
            );
        }
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
