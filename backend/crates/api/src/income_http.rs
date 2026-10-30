//! Strict realized-income transport with optional normal-account assignment.
use crate::{
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    income_creation::{self, CreateIncome, IncomeError},
    request_id::RequestId,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, Uri},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use cuenora_domain::{BusinessDate, BusinessDateError, Currency, Money};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    kind: String,
    amount: String,
    currency: String,
    occurred_on: String,
    account_id: Option<String>,
    category_id: Option<String>,
    description: Option<String>,
}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route("/v1/transactions", post(create))
        .route_layer(DefaultBodyLimit::max(8 * 1024))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ))
        .with_state(pool)
}

fn validate(headers: &HeaderMap, input: Input) -> Result<(CreateIncome, [u8; 32]), ApiError> {
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
        || input.kind != "income"
    {
        return Err(ApiError::InvalidRequest);
    }
    let currency: Currency = input
        .currency
        .parse()
        .map_err(|_| ApiError::InvalidRequest)?;
    let amount = Money::parse(&input.amount, currency).map_err(|_| ApiError::InvalidRequest)?;
    if amount.minor_units() <= 0 {
        return Err(ApiError::InvalidRequest);
    }
    let occurred_on = BusinessDate::parse(&input.occurred_on).map_err(|error| match error {
        BusinessDateError::InvalidDate => ApiError::InvalidBusinessDate,
        BusinessDateError::OutOfRange => ApiError::DateOutOfRange,
        BusinessDateError::FutureDate => ApiError::FutureBusinessDate,
    })?;
    let category_id = input.category_id.map(normalize_uuid).transpose()?;
    let account_id = input.account_id.map(normalize_uuid).transpose()?;
    if input
        .description
        .as_ref()
        .is_some_and(|text| text.chars().count() > 1000 || text.chars().any(char::is_control))
    {
        return Err(ApiError::InvalidRequest);
    }
    Ok((
        CreateIncome {
            amount,
            occurred_on,
            category_id,
            account_id,
            description: input.description,
        },
        Sha256::digest(key.as_bytes()).into(),
    ))
}

fn normalize_uuid(id: String) -> Result<String, ApiError> {
    let valid = id.len() == 36
        && id.bytes().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        });
    if valid {
        Ok(id.to_ascii_lowercase())
    } else {
        Err(ApiError::InvalidRequest)
    }
}

async fn create(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    uri: Uri,
    headers: HeaderMap,
    payload: Result<Json<Input>, JsonRejection>,
) -> Response {
    if uri.query().is_some_and(|query| !query.is_empty()) {
        return ApiError::InvalidRequest.response(&id);
    }
    let input = match payload {
        Ok(Json(input)) => input,
        Err(error) => return ApiError::invalid_json(error.status()).response(&id),
    };
    let (input, key) = match validate(&headers, input) {
        Ok(input) => input,
        Err(error) => return error.response(&id),
    };
    let Some(pool) = pool else {
        return ApiError::IncomeCreationUnavailable.response(&id);
    };
    match income_creation::create(&pool, &identity, &key, &input).await {
        Ok(income) => (StatusCode::CREATED, Json(income)).into_response(),
        Err(error) => match error {
            IncomeError::Unverified => ApiError::EmailNotVerified,
            IncomeError::Conflict => ApiError::IdempotencyConflict,
            IncomeError::InvalidCategory | IncomeError::InvalidAccount => ApiError::InvalidRequest,
            IncomeError::NotFound => ApiError::NotFound,
            IncomeError::UnconfirmedZone => ApiError::ProfileZoneUnconfirmed,
            IncomeError::UnsupportedZone => ApiError::UnsupportedTimeZone,
            IncomeError::DateOutOfRange => ApiError::DateOutOfRange,
            IncomeError::FutureDate => ApiError::FutureBusinessDate,
            IncomeError::Unavailable => ApiError::IncomeCreationUnavailable,
        }
        .response(&id),
    }
}
