//! Actual-payment review composed with existing expense posting in one transaction.
use crate::{
    api_error::ApiError,
    auth_identity::AuthenticatedIdentity,
    expense_creation::{self, CreateExpense, CreatedExpense, ExpenseError},
    income_http::{self, OrdinaryInput, normalize_uuid},
    request_id::RequestId,
};
use axum::{
    Extension, Json,
    extract::{
        Path, RawQuery, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    amount: String,
    currency: String,
    occurred_on: String,
    account_id: Option<String>,
    category_id: Option<String>,
    description: Option<String>,
}

pub(crate) async fn confirm(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    path: Result<Path<String>, PathRejection>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    payload: Result<Json<Input>, JsonRejection>,
) -> Response {
    let result = async {
        if query.is_some() {
            return Err(ApiError::InvalidRequest);
        }
        let target = normalize_uuid(path.map_err(|_| ApiError::InvalidRequest)?.0)?;
        let input = payload.map_err(|e| ApiError::invalid_json(e.status()))?.0;
        // One validator for actual ordinary expenses; planned terms never fill omissions.
        let (input, key) = income_http::validate(
            &headers,
            OrdinaryInput {
                kind: "expense".into(),
                amount: input.amount,
                currency: input.currency,
                occurred_on: input.occurred_on,
                account_id: input.account_id,
                category_id: input.category_id,
                description: input.description,
            },
        )?;
        let input = CreateExpense {
            amount: input.amount,
            occurred_on: input.occurred_on,
            account_id: input.account_id,
            category_id: input.category_id,
            description: input.description,
        };
        post(
            &pool.ok_or(ApiError::RecurrenceUnavailable)?,
            &identity,
            &target,
            &key,
            &input,
        )
        .await
    }
    .await;
    match result {
        Ok(expense) => (StatusCode::CREATED, Json(expense)).into_response(),
        Err(error) => error.response(&id),
    }
}

async fn post(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    occurrence: &str,
    key: &[u8; 32],
    input: &CreateExpense,
) -> Result<CreatedExpense, ApiError> {
    let (mut tx, zone, _) = expense_creation::begin(pool, identity)
        .await
        .map_err(expense_error)?;
    // Owner first serializes confirmation with posting, archive, profile and rule changes.
    // No rule-state check: a paused/ended rule's existing planned items stay actionable.
    let state: Option<String> = sqlx::query_scalar(
        "SELECT state FROM recurring_occurrences WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE",
    )
    .bind(occurrence)
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;
    let state = state.ok_or(ApiError::NotFound)?;
    let fingerprint = input.fingerprint();
    let prior: Option<(String, String, Vec<u8>)> = sqlx::query_as(
        "SELECT occurrence_id::text,transaction_id::text,request_hash FROM recurring_confirmation_keys
         WHERE user_id=$1::uuid AND key_hash=$2",
    ).bind(identity.user_id()).bind(key.as_slice()).fetch_optional(&mut *tx).await.map_err(storage)?;
    if let Some((target, transaction, previous)) = prior {
        if target != occurrence || previous != fingerprint {
            return Err(ApiError::IdempotencyConflict);
        }
        // Reconstruct the original normalized response, not the mutable/voided expense.
        tx.commit().await.map_err(storage)?;
        return Ok(input.result(transaction));
    }
    if state != "planned" {
        return Err(ApiError::OccurrenceStateConflict);
    }
    let captured = std::time::SystemTime::now().into();
    let expense = expense_creation::post(&mut tx, identity, input, zone.as_deref(), captured)
        .await
        .map_err(expense_error)?;
    sqlx::query(
        "UPDATE recurring_occurrences SET state='confirmed',transaction_id=$3::uuid,
                 updated_at=statement_timestamp() WHERE id=$1::uuid AND user_id=$2::uuid",
    )
    .bind(occurrence)
    .bind(identity.user_id())
    .bind(&expense.id)
    .execute(&mut *tx)
    .await
    .map_err(storage)?;
    sqlx::query("INSERT INTO recurring_confirmation_keys(user_id,key_hash,request_hash,occurrence_id,transaction_id)
                 VALUES($1::uuid,$2,$3,$4::uuid,$5::uuid)")
        .bind(identity.user_id()).bind(key.as_slice()).bind(fingerprint.as_slice())
        .bind(occurrence).bind(&expense.id).execute(&mut *tx).await.map_err(storage)?;
    tx.commit().await.map_err(storage)?;
    Ok(expense)
}
fn storage(_: sqlx::Error) -> ApiError {
    ApiError::RecurrenceUnavailable
}
fn expense_error(error: ExpenseError) -> ApiError {
    match error {
        ExpenseError::Unverified => ApiError::EmailNotVerified,
        ExpenseError::Conflict => ApiError::IdempotencyConflict,
        ExpenseError::InvalidCategory | ExpenseError::InvalidAccount => ApiError::InvalidRequest,
        ExpenseError::NotFound => ApiError::NotFound,
        ExpenseError::UnconfirmedZone => ApiError::ProfileZoneUnconfirmed,
        ExpenseError::UnsupportedZone => ApiError::UnsupportedTimeZone,
        ExpenseError::DateOutOfRange => ApiError::DateOutOfRange,
        ExpenseError::FutureDate => ApiError::FutureBusinessDate,
        ExpenseError::Unavailable => ApiError::RecurrenceUnavailable,
    }
}
