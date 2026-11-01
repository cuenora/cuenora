//! Strict recurrence transport. Rule metadata never posts a financial event.
use crate::{
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    income_http::normalize_uuid,
    profile_zone::parse_zone,
    recurring_rules,
    request_id::RequestId,
};
use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Path, RawQuery, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{patch, post},
};
use cuenora_domain::{BusinessDate, Currency, Money};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Terms {
    pub amount: String,
    pub currency: String,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub description: Option<String>,
    pub cadence: String,
    pub starts_on: String,
    pub ends_on: Option<String>,
    pub time_zone: String,
}
impl Terms {
    pub fn validate(mut self) -> Result<Self, ApiError> {
        let currency: Currency = self
            .currency
            .parse()
            .map_err(|_| ApiError::InvalidRequest)?;
        let money = Money::parse(&self.amount, currency).map_err(|_| ApiError::InvalidRequest)?;
        if money.minor_units() <= 0
            || !matches!(
                self.cadence.as_str(),
                "weekly" | "monthly" | "quarterly" | "annual"
            )
        {
            return Err(ApiError::InvalidRequest);
        }
        self.amount = money.to_string();
        let start =
            BusinessDate::parse(&self.starts_on).map_err(|_| ApiError::InvalidBusinessDate)?;
        if let Some(end) = &self.ends_on {
            let end = BusinessDate::parse(end).map_err(|_| ApiError::InvalidBusinessDate)?;
            if end < start {
                return Err(ApiError::InvalidRequest);
            }
        }
        parse_zone(&self.time_zone)?;
        self.account_id = self.account_id.map(normalize_uuid).transpose()?;
        self.category_id = self.category_id.map(normalize_uuid).transpose()?;
        if self
            .description
            .as_ref()
            .is_some_and(|s| s.chars().count() > 1000 || s.chars().any(char::is_control))
        {
            return Err(ApiError::InvalidRequest);
        }
        Ok(self)
    }
    pub fn minor_units(&self) -> i64 {
        // Only validated inputs and constrained database rows reach this method.
        Money::parse(
            &self.amount,
            self.currency.parse().expect("validated currency"),
        )
        .expect("validated amount")
        .minor_units()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"recurring-rule-v1");
        for field in [
            Some(&self.amount),
            Some(&self.currency),
            self.account_id.as_ref(),
            self.category_id.as_ref(),
            self.description.as_ref(),
            Some(&self.cadence),
            Some(&self.starts_on),
            self.ends_on.as_ref(),
            Some(&self.time_zone),
        ] {
            match field {
                Some(value) => {
                    hash.update([1]);
                    hash.update((value.len() as u64).to_be_bytes());
                    hash.update(value.as_bytes());
                }
                None => hash.update([0]),
            }
        }
        hash.finalize().into()
    }
}

#[derive(Default)]
enum Field<T> {
    #[default]
    Absent,
    Present(T),
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Field<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::Present)
    }
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Edit {
    #[serde(default)]
    amount: Field<String>,
    #[serde(default)]
    currency: Field<String>,
    #[serde(default)]
    account_id: Field<Option<String>>,
    #[serde(default)]
    category_id: Field<Option<String>>,
    #[serde(default)]
    description: Field<Option<String>>,
    #[serde(default)]
    cadence: Field<String>,
    #[serde(default)]
    starts_on: Field<String>,
    #[serde(default)]
    ends_on: Field<Option<String>>,
    #[serde(default)]
    time_zone: Field<String>,
}
impl Edit {
    pub fn merge(self, mut old: Terms) -> Result<Terms, ApiError> {
        let mut present = false;
        macro_rules! apply { ($($name:ident),*) => { $(if let Field::Present(value) = self.$name { old.$name = value; present = true; })* }; }
        apply!(
            amount,
            currency,
            account_id,
            category_id,
            description,
            cadence,
            starts_on,
            ends_on,
            time_zone
        );
        if !present {
            return Err(ApiError::InvalidRequest);
        }
        old.validate()
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route("/v1/recurring-rules", post(create))
        .route(
            "/v1/recurring-occurrences/{id}/confirm",
            post(crate::recurring_confirmation::confirm),
        )
        .route(
            "/v1/recurring-occurrences/{id}/skip",
            post(crate::recurring_occurrence_actions::skip),
        )
        .route(
            "/v1/recurring-occurrences/{id}/postpone",
            post(crate::recurring_occurrence_actions::postpone),
        )
        .route("/v1/recurring-rules/{id}", patch(edit))
        .route("/v1/recurring-rules/{id}/pause", post(pause))
        .route("/v1/recurring-rules/{id}/resume", post(resume))
        .route_layer(DefaultBodyLimit::max(8 * 1024))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ))
        .with_state(pool)
}
pub(crate) fn key(headers: &HeaderMap) -> Result<[u8; 32], ApiError> {
    let mut values = headers.get_all("idempotency-key").iter();
    let value = values
        .next()
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::InvalidRequest)?;
    if values.next().is_some()
        || value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(ApiError::InvalidRequest);
    }
    Ok(Sha256::digest(value.as_bytes()).into())
}
fn target(
    target: Result<Path<String>, PathRejection>,
    query: Option<String>,
) -> Result<String, ApiError> {
    if query.is_some() {
        return Err(ApiError::InvalidRequest);
    }
    normalize_uuid(target.map_err(|_| ApiError::InvalidRequest)?.0)
}
async fn create(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    payload: Result<Json<Terms>, JsonRejection>,
) -> Response {
    let result = async {
        if query.is_some() {
            return Err(ApiError::InvalidRequest);
        }
        let input = payload
            .map_err(|e| ApiError::invalid_json(e.status()))?
            .0
            .validate()?;
        let hash = key(&headers)?;
        let pool = pool.ok_or(ApiError::RecurrenceUnavailable)?;
        recurring_rules::create(&pool, &identity, &hash, input).await
    }
    .await;
    match result {
        Ok(output) => (StatusCode::CREATED, Json(output)).into_response(),
        Err(error) => error.response(&id),
    }
}
async fn edit(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    path: Result<Path<String>, PathRejection>,
    RawQuery(query): RawQuery,
    payload: Result<Json<Edit>, JsonRejection>,
) -> Response {
    let result = async {
        let target = target(path, query)?;
        let input = payload.map_err(|e| ApiError::invalid_json(e.status()))?.0;
        recurring_rules::change(
            &pool.ok_or(ApiError::RecurrenceUnavailable)?,
            &identity,
            &target,
            recurring_rules::Change::Edit(Box::new(input)),
        )
        .await
    }
    .await;
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error.response(&id),
    }
}
macro_rules! transition_handler {
    ($name:ident,$change:ident) => {
        async fn $name(
            State(pool): State<Option<PgPool>>,
            Extension(identity): Extension<AuthenticatedIdentity>,
            Extension(id): Extension<RequestId>,
            path: Result<Path<String>, PathRejection>,
            RawQuery(query): RawQuery,
            payload: Result<Json<Transition>, JsonRejection>,
        ) -> Response {
            let result = async {
                let target = target(path, query)?;
                let Json(_) = payload.map_err(|e| ApiError::invalid_json(e.status()))?;
                recurring_rules::change(
                    &pool.ok_or(ApiError::RecurrenceUnavailable)?,
                    &identity,
                    &target,
                    recurring_rules::Change::$change,
                )
                .await
            }
            .await;
            match result {
                Ok(output) => Json(output).into_response(),
                Err(error) => error.response(&id),
            }
        }
    };
}
transition_handler!(pause, Pause);
transition_handler!(resume, Resume);
