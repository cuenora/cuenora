//! Strict account transport with optional exact, explicitly dated opening input.
use crate::{
    account_creation::{self, CreateAccount, CreationError},
    account_listing::{self, ListingError},
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    request_id::RequestId,
};
use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, Query, Request, State,
        rejection::{BytesRejection, JsonRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use cuenora_domain::{AccountName, AccountType, BusinessDate, BusinessDateError, Currency, Money};
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    name: String,
    account_type: String,
    currency: String,
    #[serde(default, deserialize_with = "opening_input")]
    opening_balance: Option<OpeningInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpeningInput {
    amount: String,
    occurred_on: String,
}

// Absence preserves P05.02; explicit null is not an opening instruction.
fn opening_input<'de, D: Deserializer<'de>>(d: D) -> Result<Option<OpeningInput>, D::Error> {
    OpeningInput::deserialize(d).map(Some)
}

fn date_error(error: BusinessDateError) -> ApiError {
    match error {
        BusinessDateError::InvalidDate => ApiError::InvalidBusinessDate,
        BusinessDateError::OutOfRange => ApiError::DateOutOfRange,
        BusinessDateError::FutureDate => ApiError::FutureBusinessDate,
    }
}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    let methods = get(list)
        .layer(middleware::from_fn(bound_listing_query))
        .post(create);
    Router::new()
        .route("/v1/accounts", methods)
        .route_layer(DefaultBodyLimit::max(8 * 1024))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ))
        .with_state(pool)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListingQuery {
    limit: Option<u16>,
    cursor: Option<String>,
}

async fn bound_listing_query(request: Request, next: Next) -> Response {
    if request
        .uri()
        .query()
        .is_some_and(|query| query.len() > 1024)
    {
        return match request.extensions().get::<RequestId>() {
            Some(id) => ApiError::InvalidRequest.response(id),
            None => ApiError::without_request_id(),
        };
    }
    next.run(request).await
}

async fn list(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    query: Result<Query<ListingQuery>, QueryRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(_) => return ApiError::InvalidRequest.response(&id),
    };
    let body = match body {
        Ok(body) => body,
        Err(error) => return ApiError::invalid_json(error.status()).response(&id),
    };
    if !body.is_empty() {
        return ApiError::InvalidRequest.response(&id);
    }
    if query.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
        return ApiError::InvalidRequest.response(&id);
    }
    let Some(pool) = pool else {
        return ApiError::AccountListingUnavailable.response(&id);
    };
    match account_listing::list(&pool, &identity, query.limit, query.cursor.as_deref()).await {
        Ok(listing) => (StatusCode::OK, Json(listing)).into_response(),
        Err(error) => match error {
            ListingError::EmailNotVerified => ApiError::EmailNotVerified,
            ListingError::InvalidRequest => ApiError::InvalidRequest,
            ListingError::Unavailable => ApiError::AccountListingUnavailable,
        }
        .response(&id),
    }
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
    let opening = input
        .opening_balance
        .map(|opening| {
            Ok(account_creation::OpeningBalance {
                amount: Money::parse(&opening.amount, currency)
                    .map_err(|_| ApiError::InvalidRequest)?,
                occurred_on: BusinessDate::parse(&opening.occurred_on).map_err(date_error)?,
            })
        })
        .transpose()?;
    Ok((
        CreateAccount {
            name,
            account_type,
            currency,
            opening,
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
            CreationError::UnconfirmedZone => ApiError::ProfileZoneUnconfirmed,
            CreationError::UnsupportedZone => ApiError::UnsupportedTimeZone,
            CreationError::DateOutOfRange => ApiError::DateOutOfRange,
            CreationError::FutureDate => ApiError::FutureBusinessDate,
        }
        .response(&id),
    }
}
