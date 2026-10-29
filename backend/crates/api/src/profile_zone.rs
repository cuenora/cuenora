//! Explicit profile time-zone confirmation for authenticated users.
use crate::{
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    request_id::RequestId,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::patch,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    time_zone: String,
}

#[derive(Serialize)]
struct Output {
    time_zone: String,
}

/// Parse exact supported IANA identifiers and recognized chrono-tz aliases.
pub(crate) fn parse_zone(value: &str) -> Result<Tz, ApiError> {
    if value.is_empty()
        || value.len() > 255
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(ApiError::UnsupportedTimeZone);
    }
    Tz::from_str(value).map_err(|_| ApiError::UnsupportedTimeZone)
}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route(
            "/v1/me",
            patch(update).layer(DefaultBodyLimit::max(8 * 1024)).layer(
                middleware::from_fn_with_state(pool.clone(), require_identity),
            ),
        )
        .with_state(pool)
}

async fn update(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    payload: Result<Json<Input>, JsonRejection>,
) -> Response {
    let Json(input) = match payload {
        Ok(input) => input,
        Err(error) => return ApiError::invalid_json(error.status()).response(&id),
    };
    if let Err(error) = parse_zone(&input.time_zone) {
        return error.response(&id);
    }
    let Some(pool) = pool else {
        return ApiError::ProfileUpdateUnavailable.response(&id);
    };

    match update_profile_zone(&pool, &identity, &input.time_zone).await {
        Ok(()) => (
            StatusCode::OK,
            Json(Output {
                time_zone: input.time_zone,
            }),
        )
            .into_response(),
        Err(()) => ApiError::ProfileUpdateUnavailable.response(&id),
    }
}

async fn update_profile_zone(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    time_zone: &str,
) -> Result<(), ()> {
    let mut tx = pool.begin().await.map_err(|_| ())?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| ())?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| ())?;
    let current: Option<Option<String>> =
        sqlx::query_scalar("SELECT profile_time_zone FROM users WHERE id = $1::uuid FOR UPDATE")
            .bind(identity.user_id())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| ())?;
    let Some(current) = current else {
        return Err(());
    };
    if current.as_deref() != Some(time_zone) {
        let updated = sqlx::query(
            "UPDATE users SET profile_time_zone = $1, updated_at = statement_timestamp()
             WHERE id = $2::uuid",
        )
        .bind(time_zone)
        .bind(identity.user_id())
        .execute(&mut *tx)
        .await
        .map_err(|_| ())?;
        if updated.rows_affected() != 1 {
            return Err(());
        }
    }
    tx.commit().await.map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::parse_zone;

    #[test]
    fn parses_supported_zones_and_recognized_aliases() {
        assert_eq!(
            parse_zone("Europe/Stockholm").unwrap().name(),
            "Europe/Stockholm"
        );
        assert_eq!(parse_zone("US/Eastern").unwrap().name(), "US/Eastern");
        assert_eq!(parse_zone("Etc/UTC").unwrap().name(), "Etc/UTC");
    }

    #[test]
    fn rejects_unsupported_or_noncanonical_input() {
        for value in [
            "",
            "Not/AZone",
            "europe/Stockholm",
            "+01:00",
            "UTC+1",
            " Europe/Stockholm",
            "Europe/Stockholm ",
            "Europe//Stockholm",
            "Europe/Stockholm\n",
        ] {
            assert!(parse_zone(value).is_err(), "accepted {value:?}");
        }
        assert!(parse_zone(&"a".repeat(256)).is_err());
    }
}
