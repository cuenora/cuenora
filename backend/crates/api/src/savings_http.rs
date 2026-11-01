//! Append-only savings configuration. Explicit dates never infer a local today.
use crate::{
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    income_http::normalize_uuid,
    request_id::RequestId,
};
use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, Path, Query, RawQuery, State,
        rejection::{BytesRejection, JsonRejection, PathRejection, QueryRejection},
    },
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use cuenora_domain::BusinessDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Terms {
    pub(crate) effective_on: String,
    pub(crate) payout_frequency: String,
    pub(crate) capitalize: bool,
    pub(crate) rate_basis: String,
    pub(crate) day_count: String,
    pub(crate) balance_timing: String,
    pub(crate) rounding: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rate {
    pub(crate) effective_on: String,
    pub(crate) nominal_annual_percent: String,
}
#[derive(Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum Configuration {
    Terms(Terms),
    Rate(Rate),
}
impl Configuration {
    pub(crate) fn date(&self) -> &str {
        match self {
            Self::Terms(t) => &t.effective_on,
            Self::Rate(r) => &r.effective_on,
        }
    }
    fn validate(&self) -> Result<(), ApiError> {
        BusinessDate::parse(self.date()).map_err(|_| ApiError::InvalidBusinessDate)?;
        let valid = match self {
            Self::Terms(t) => {
                matches!(
                    t.payout_frequency.as_str(),
                    "daily" | "monthly" | "quarterly" | "annual"
                ) && t.rate_basis == "nominal_annual"
                    && t.day_count == "actual_calendar_year"
                    && t.balance_timing == "daily_close"
                    && t.rounding == "period_end_half_even"
            }
            Self::Rate(r) => valid_rate(&r.nominal_annual_percent),
        };
        if valid {
            Ok(())
        } else {
            Err(ApiError::InvalidRequest)
        }
    }
}
fn valid_rate(rate: &str) -> bool {
    // Bound parsing and PostgreSQL NUMERIC conversion, without floating point or rounding.
    if rate.len() > 1024 {
        return false;
    }
    let mut parts = rate.split('.');
    let integer = parts.next().unwrap_or_default();
    !integer.is_empty()
        && integer.bytes().all(|c| c.is_ascii_digit())
        && parts.next().is_none_or(|fraction| {
            !fraction.is_empty()
                && fraction.len() <= 6
                && fraction.bytes().all(|c| c.is_ascii_digit())
        })
        && parts.next().is_none()
}
#[derive(Serialize)]
pub(crate) struct Version {
    pub(crate) id: String,
    #[serde(flatten)]
    pub(crate) configuration: Configuration,
}
#[derive(Serialize)]
pub(crate) struct History {
    pub(crate) versions: Vec<Version>,
    pub(crate) next_cursor: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Page {
    pub(crate) limit: Option<u16>,
    pub(crate) cursor: Option<String>,
}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route(
            "/v1/accounts/{id}/savings-terms",
            get(list_terms).post(append_terms),
        )
        .route(
            "/v1/accounts/{id}/savings-rates",
            get(list_rates).post(append_rates),
        )
        .method_not_allowed_fallback(|Extension(id): Extension<RequestId>| async move {
            ApiError::MethodNotAllowed.response(&id)
        })
        .route_layer(DefaultBodyLimit::max(8 * 1024))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ))
        .with_state(pool)
}
fn reply<T: Serialize>(
    result: Result<T, ApiError>,
    status: StatusCode,
    id: &RequestId,
) -> Response {
    match result {
        Ok(body) => (status, Json(body)).into_response(),
        Err(error) => error.response(id),
    }
}
fn account(path: Result<Path<String>, PathRejection>) -> Result<String, ApiError> {
    normalize_uuid(path.map_err(|_| ApiError::InvalidRequest)?.0)
}
macro_rules! append_handler {
    ($name:ident,$input:ty,$kind:ident) => {
        async fn $name(
            State(pool): State<Option<PgPool>>,
            Extension(identity): Extension<AuthenticatedIdentity>,
            Extension(id): Extension<RequestId>,
            path: Result<Path<String>, PathRejection>,
            RawQuery(query): RawQuery,
            payload: Result<Json<$input>, JsonRejection>,
        ) -> Response {
            let result = async {
                if query.is_some() {
                    return Err(ApiError::InvalidRequest);
                }
                let account = account(path)?;
                let input = Configuration::$kind(
                    payload.map_err(|e| ApiError::invalid_json(e.status()))?.0,
                );
                input.validate()?;
                crate::savings_configuration::append(
                    &pool.ok_or(ApiError::SavingsUnavailable)?,
                    &identity,
                    &account,
                    input,
                )
                .await
            }
            .await;
            reply(result, StatusCode::CREATED, &id)
        }
    };
}
append_handler!(append_terms, Terms, Terms);
append_handler!(append_rates, Rate, Rate);
macro_rules! list_handler {
    ($name:ident,$rates:expr) => {
        async fn $name(
            State(pool): State<Option<PgPool>>,
            Extension(identity): Extension<AuthenticatedIdentity>,
            Extension(id): Extension<RequestId>,
            path: Result<Path<String>, PathRejection>,
            RawQuery(raw): RawQuery,
            query: Result<Query<Page>, QueryRejection>,
            body: Result<Bytes, BytesRejection>,
        ) -> Response {
            let result = async {
                if raw.is_some_and(|s| s.len() > 1024) {
                    return Err(ApiError::InvalidRequest);
                }
                let account = account(path)?;
                let page = query.map_err(|_| ApiError::InvalidRequest)?.0;
                if !body
                    .map_err(|e| ApiError::invalid_json(e.status()))?
                    .is_empty()
                    || page.limit.is_some_and(|n| !(1..=100).contains(&n))
                {
                    return Err(ApiError::InvalidRequest);
                }
                if let Some(cursor) = &page.cursor {
                    BusinessDate::parse(cursor).map_err(|_| ApiError::InvalidRequest)?;
                }
                crate::savings_configuration::history(
                    &pool.ok_or(ApiError::SavingsUnavailable)?,
                    &identity,
                    &account,
                    $rates,
                    page,
                )
                .await
            }
            .await;
            reply(result, StatusCode::OK, &id)
        }
    };
}
list_handler!(list_terms, false);
list_handler!(list_rates, true);
