//! Strict no-quote transfer request adapter; ordinary entry inputs stay unchanged.
use crate::{
    api_error::ApiError,
    auth_identity::AuthenticatedIdentity,
    income_http::normalize_uuid,
    request_id::RequestId,
    transfer_creation::{self, CreateTransfer, TransferError},
};
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cuenora_domain::{BusinessDate, BusinessDateError, Currency, Money};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    kind: String,
    amount: String,
    currency: String,
    account_id: String,
    destination_account_id: String,
    received_amount: String,
    received_currency: String,
    occurred_on: String,
    description: Option<String>,
}

fn validate(headers: &HeaderMap, input: Input) -> Result<(CreateTransfer, [u8; 32]), ApiError> {
    let mut keys = headers.get_all("idempotency-key").iter();
    let key = keys
        .next()
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::InvalidRequest)?;
    if keys.next().is_some()
        || key.is_empty()
        || key.len() > 128
        || !key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        || input.kind != "transfer"
    {
        return Err(ApiError::InvalidRequest);
    }
    let currency: Currency = input
        .currency
        .parse()
        .map_err(|_| ApiError::InvalidRequest)?;
    let received_currency: Currency = input
        .received_currency
        .parse()
        .map_err(|_| ApiError::InvalidRequest)?;
    let amount = Money::parse(&input.amount, currency).map_err(|_| ApiError::InvalidRequest)?;
    let received = Money::parse(&input.received_amount, received_currency)
        .map_err(|_| ApiError::InvalidRequest)?;
    if amount.minor_units() <= 0
        || received.minor_units() <= 0
        || (currency == received_currency && amount != received)
    {
        return Err(ApiError::InvalidRequest);
    }
    let source = normalize_uuid(input.account_id)?;
    let destination = normalize_uuid(input.destination_account_id)?;
    if source == destination {
        return Err(ApiError::InvalidRequest);
    }
    let occurred_on = BusinessDate::parse(&input.occurred_on).map_err(|error| match error {
        BusinessDateError::InvalidDate => ApiError::InvalidBusinessDate,
        BusinessDateError::OutOfRange => ApiError::DateOutOfRange,
        BusinessDateError::FutureDate => ApiError::FutureBusinessDate,
    })?;
    if input
        .description
        .as_ref()
        .is_some_and(|text| text.chars().count() > 1000 || text.chars().any(char::is_control))
    {
        return Err(ApiError::InvalidRequest);
    }
    Ok((
        CreateTransfer {
            amount,
            received,
            source,
            destination,
            occurred_on,
            description: input.description,
        },
        Sha256::digest(key.as_bytes()).into(),
    ))
}

pub(crate) async fn create(
    pool: Option<PgPool>,
    identity: AuthenticatedIdentity,
    id: RequestId,
    headers: HeaderMap,
    input: Input,
) -> Response {
    let (input, key) = match validate(&headers, input) {
        Ok(value) => value,
        Err(error) => return error.response(&id),
    };
    let Some(pool) = pool else {
        return ApiError::TransferCreationUnavailable.response(&id);
    };
    match transfer_creation::create(&pool, &identity, &key, &input).await {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(error) => match error {
            TransferError::Unverified => ApiError::EmailNotVerified,
            TransferError::Conflict => ApiError::IdempotencyConflict,
            TransferError::NotFound => ApiError::NotFound,
            TransferError::InvalidAccount => ApiError::InvalidRequest,
            TransferError::UnconfirmedZone => ApiError::ProfileZoneUnconfirmed,
            TransferError::UnsupportedZone => ApiError::UnsupportedTimeZone,
            TransferError::DateOutOfRange => ApiError::DateOutOfRange,
            TransferError::FutureDate => ApiError::FutureBusinessDate,
            TransferError::Unavailable => ApiError::TransferCreationUnavailable,
        }
        .response(&id),
    }
}
