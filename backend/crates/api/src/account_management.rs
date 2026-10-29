//! Atomic owned account metadata updates, never financial history mutation.
use crate::{
    account_listing::valid_cursor, api_error::ApiError, auth_identity::AuthenticatedIdentity,
    request_id::RequestId,
};
use axum::{
    Extension, Json,
    extract::{
        Path, RawQuery, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cuenora_domain::AccountName;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    #[serde(default, deserialize_with = "present_string")]
    name: Option<String>,
    #[serde(default, deserialize_with = "present_string")]
    status: Option<String>,
}

fn present_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

#[derive(Serialize)]
struct Output {
    id: String,
    name: String,
    account_type: String,
    currency: String,
    status: &'static str,
}

pub(crate) async fn update(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(request_id): Extension<RequestId>,
    target: Result<Path<String>, PathRejection>,
    RawQuery(query): RawQuery,
    payload: Result<Json<Input>, JsonRejection>,
) -> Response {
    let Path(id) = match target {
        Ok(target) => target,
        Err(_) => return ApiError::InvalidRequest.response(&request_id),
    };
    let Json(input) = match payload {
        Ok(input) => input,
        Err(error) => return ApiError::invalid_json(error.status()).response(&request_id),
    };
    if !valid_cursor(&id) || query.is_some() || (input.name.is_none() && input.status.is_none()) {
        return ApiError::InvalidRequest.response(&request_id);
    }
    let name = match input.name.map(|name| AccountName::parse(&name)).transpose() {
        Ok(name) => name,
        Err(_) => return ApiError::InvalidRequest.response(&request_id),
    };
    let active = match input.status.as_deref() {
        None => None,
        Some("active") => Some(true),
        Some("archived") => Some(false),
        Some(_) => return ApiError::InvalidRequest.response(&request_id),
    };
    let Some(pool) = pool else {
        return ApiError::AccountUpdateUnavailable.response(&request_id);
    };
    match apply(&pool, &identity, &id, name.as_ref(), active).await {
        Ok(account) => (StatusCode::OK, Json(account)).into_response(),
        Err(error) => error.response(&request_id),
    }
}

async fn apply(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    id: &str,
    name: Option<&AccountName>,
    active: Option<bool>,
) -> Result<Output, ApiError> {
    let unavailable = |_| ApiError::AccountUpdateUnavailable;
    let mut tx = pool.begin().await.map_err(unavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    // READ COMMITTED is intentional: after waiting for locks, the next statement
    // sees postings committed by the preceding writer. Keep owner→account order
    // in future financial writes and check active state before inserting money.
    let verified: Option<bool> = sqlx::query_scalar(
        "SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid FOR UPDATE",
    )
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    if verified != Some(true) {
        return Err(ApiError::EmailNotVerified);
    }
    let current: Option<(String, String, String, bool)> = sqlx::query_as(
        "SELECT name,account_type,currency,archived_at IS NULL FROM accounts
         WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE",
    )
    .bind(id)
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    let Some((old_name, account_type, currency, was_active)) = current else {
        return Err(ApiError::NotFound);
    };
    let new_active = active.unwrap_or(was_active);
    if was_active && !new_active {
        // Exact numeric SUM: zero comparison must not narrow overflowing totals.
        // Include compensations and never filter by current event lifecycle.
        let zero: bool = sqlx::query_scalar(
            "SELECT COALESCE(SUM(signed_amount_minor),0)=0 FROM account_movements
             WHERE user_id=$1::uuid AND account_id=$2::uuid AND currency=$3",
        )
        .bind(identity.user_id())
        .bind(id)
        .bind(&currency)
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if !zero {
            return Err(ApiError::AccountBalanceNotZero);
        }
    }
    let new_name = name.map_or(old_name.as_str(), AccountName::as_str);
    if new_name != old_name || new_active != was_active {
        let changed = sqlx::query(
            "UPDATE accounts SET name=$1,
                 archived_at=CASE WHEN $2 THEN NULL
                                  WHEN archived_at IS NULL THEN statement_timestamp()
                                  ELSE archived_at END,
                 updated_at=statement_timestamp()
             WHERE id=$3::uuid AND user_id=$4::uuid",
        )
        .bind(new_name)
        .bind(new_active)
        .bind(id)
        .bind(identity.user_id())
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(ApiError::AccountUpdateUnavailable);
        }
    }
    let result = Output {
        id: id.to_ascii_lowercase(),
        name: new_name.to_owned(),
        account_type,
        currency,
        status: if new_active { "active" } else { "archived" },
    };
    tx.commit().await.map_err(unavailable)?;
    Ok(result)
}
