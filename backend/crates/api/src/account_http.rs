//! Strict account transport. No client-supplied authority or opening balance.
use crate::{
    account_creation::{self, CreateAccount, CreationError},
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    request_id::RequestId,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use cuenora_domain::{AccountName, AccountType, Currency};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    name: String,
    account_type: String,
    currency: String,
}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route(
            "/v1/accounts",
            post(create).layer(DefaultBodyLimit::max(8 * 1024)).layer(
                middleware::from_fn_with_state(pool.clone(), require_identity),
            ),
        )
        .with_state(pool)
}

fn validate(headers: &HeaderMap, input: Input) -> Result<(CreateAccount, [u8; 32]), ApiError> {
    let mut keys = headers.get_all("idempotency-key").iter();
    let key = keys
        .next()
        .and_then(|key| key.to_str().ok())
        .ok_or(ApiError::InvalidRequest)?;
    if keys.next().is_some()
        || key.is_empty()
        || key.len() > 128
        || !key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(ApiError::InvalidRequest);
    }
    let name = AccountName::parse(&input.name).map_err(|_| ApiError::InvalidRequest)?;
    let account_type = match input.account_type.as_str() {
        "normal" => AccountType::Normal,
        "savings" => AccountType::Savings,
        _ => return Err(ApiError::InvalidRequest),
    };
    let currency: Currency = input
        .currency
        .parse()
        .map_err(|_| ApiError::InvalidRequest)?;
    Ok((
        CreateAccount {
            name,
            account_type,
            currency,
        },
        Sha256::digest(key.as_bytes()).into(),
    ))
}

async fn create(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    headers: HeaderMap,
    payload: Result<Json<Input>, JsonRejection>,
) -> Response {
    let input = match payload {
        Ok(Json(input)) => input,
        Err(error) => return ApiError::invalid_json(error.status()).response(&id),
    };
    let (input, key) = match validate(&headers, input) {
        Ok(input) => input,
        Err(error) => return error.response(&id),
    };
    let Some(pool) = pool else {
        return ApiError::AccountCreationUnavailable.response(&id);
    };
    match account_creation::create(&pool, &identity, &key, &input).await {
        Ok(account) => (StatusCode::CREATED, Json(account)).into_response(),
        Err(error) => match error {
            CreationError::Unverified => ApiError::EmailNotVerified,
            CreationError::Conflict => ApiError::IdempotencyConflict,
            CreationError::Unavailable => ApiError::AccountCreationUnavailable,
        }
        .response(&id),
    }
}
