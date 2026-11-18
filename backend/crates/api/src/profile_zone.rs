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
    routing::get,
};
use jiff::tz;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    time_zone: String,
}

#[derive(Serialize)]
struct Output {
    time_zone: String,
}

#[derive(Serialize)]
struct Profile {
    id: String,
    email_verified: bool,
    time_zone: Option<String>,
}

/// Parse exact supported IANA identifiers and recognized aliases.
pub(crate) fn parse_zone(value: &str) -> Result<tz::TimeZone, ApiError> {
    if value.is_empty()
        || value.len() > 255
        // chrono-tz did not expose these database pseudo-zones as parseable
        // `Tz` values; retain that narrower accepted-name set.
        || matches!(value, "Factory" | "posixrules")
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(ApiError::UnsupportedTimeZone);
    }
    // Jiff's default database can prefer host zoneinfo; the application
    // contract uses a stable, embedded IANA catalog instead.
    let database = tz::TimeZoneDatabase::bundled();
    // Jiff's database lookup is case-insensitive. The previous contract was
    // case-sensitive and accepted IANA IDs/aliases only, so check its exact
    // identifier inventory before resolving a zone.
    if !database.available().any(|name| name.as_str() == value) {
        return Err(ApiError::UnsupportedTimeZone);
    }
    let zone = database
        .get(value)
        .map_err(|_| ApiError::UnsupportedTimeZone)?;
    if zone.iana_name().is_none() {
        return Err(ApiError::UnsupportedTimeZone);
    }
    Ok(zone)
}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route(
            "/v1/me",
            get(read)
                .patch(update)
                .layer(DefaultBodyLimit::max(8 * 1024))
                .layer(middleware::from_fn_with_state(
                    pool.clone(),
                    require_identity,
                )),
        )
        .with_state(pool)
}

async fn read(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
) -> Response {
    let Some(pool) = pool else {
        return ApiError::ProfileReadUnavailable.response(&id);
    };
    match sqlx::query_as::<_, (String, bool, Option<String>)>(
        "SELECT id::text, email_verified_at IS NOT NULL AS email_verified,
         profile_time_zone AS time_zone FROM users WHERE id = $1::uuid",
    )
    .bind(identity.user_id())
    .fetch_optional(&pool)
    .await
    {
        Ok(Some((id, email_verified, time_zone))) => Json(Profile {
            id,
            email_verified,
            time_zone,
        })
        .into_response(),
        _ => ApiError::ProfileReadUnavailable.response(&id),
    }
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
        for value in ["Europe/Stockholm", "US/Eastern", "Etc/UTC", "UTC"] {
            assert!(parse_zone(value).unwrap().iana_name().is_some(), "{value}");
        }
    }

    #[test]
    fn rejects_unsupported_or_noncanonical_input() {
        for value in [
            "",
            "Not/AZone",
            "Factory",
            "posixrules",
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
