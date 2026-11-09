//! Owner-scoped, read-only pages for recurring rules and planned occurrences.
use crate::{
    api_error::ApiError, auth_identity::AuthenticatedIdentity, income_http::normalize_uuid,
    recurring_rules, request_id::RequestId,
};
use axum::{
    Extension, Json,
    body::Bytes,
    extract::{
        Query, State,
        rejection::{BytesRejection, QueryRejection},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use cuenora_domain::{BusinessDate, Currency, Money};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};

const DEFAULT_LIMIT: u16 = 50;
const MAX_LIMIT: u16 = 100;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuleFilters {
    limit: Option<u16>,
    cursor: Option<String>,
    state: Option<String>,
}

impl RuleFilters {
    fn validate(&self) -> Result<(), ApiError> {
        if !(1..=MAX_LIMIT).contains(&self.limit.unwrap_or(DEFAULT_LIMIT))
            || self
                .state
                .as_deref()
                .is_some_and(|state| !matches!(state, "active" | "paused"))
        {
            return Err(ApiError::InvalidRequest);
        }
        if self
            .cursor
            .as_deref()
            .is_some_and(|cursor| normalize_uuid(cursor.to_owned()).is_err())
        {
            return Err(ApiError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OccurrenceFilters {
    limit: Option<u16>,
    cursor: Option<String>,
    state: Option<String>,
    rule_id: Option<String>,
}

impl OccurrenceFilters {
    fn validate(&self) -> Result<(), ApiError> {
        if !(1..=MAX_LIMIT).contains(&self.limit.unwrap_or(DEFAULT_LIMIT))
            || self
                .state
                .as_deref()
                .is_some_and(|state| !matches!(state, "planned" | "confirmed" | "skipped"))
        {
            return Err(ApiError::InvalidRequest);
        }
        if self
            .rule_id
            .as_deref()
            .is_some_and(|id| normalize_uuid(id.to_owned()).is_err())
        {
            return Err(ApiError::InvalidRequest);
        }
        if let Some(cursor) = &self.cursor {
            let (due_on, id) = cursor.split_once('~').ok_or(ApiError::InvalidRequest)?;
            BusinessDate::parse(due_on).map_err(|_| ApiError::InvalidRequest)?;
            normalize_uuid(id.to_owned()).map_err(|_| ApiError::InvalidRequest)?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct RulePage {
    rules: Vec<recurring_rules::Output>,
    next_cursor: Option<String>,
}

#[derive(Serialize)]
struct Occurrence {
    id: String,
    rule_id: String,
    scheduled_on: String,
    due_on: String,
    state: String,
    amount: String,
    currency: String,
    account_id: Option<String>,
    category_id: Option<String>,
    description: Option<String>,
    transaction_id: Option<String>,
}

#[derive(Serialize)]
struct OccurrencePage {
    occurrences: Vec<Occurrence>,
    next_cursor: Option<String>,
}

fn unavailable(_: sqlx::Error) -> ApiError {
    ApiError::RecurrenceUnavailable
}

fn rule_from_row(row: &PgRow) -> Result<recurring_rules::Output, ApiError> {
    recurring_rules::Output::from_row(row)
}

fn occurrence_from_row(row: &PgRow) -> Result<Occurrence, ApiError> {
    let currency: String = row.try_get("currency").map_err(unavailable)?;
    let amount = Money::from_minor_units(
        row.try_get("amount_minor").map_err(unavailable)?,
        currency
            .parse::<Currency>()
            .map_err(|_| ApiError::RecurrenceUnavailable)?,
    )
    .map_err(|_| ApiError::RecurrenceUnavailable)?;
    Ok(Occurrence {
        id: row.try_get("id").map_err(unavailable)?,
        rule_id: row.try_get("rule_id").map_err(unavailable)?,
        scheduled_on: row.try_get("scheduled_on").map_err(unavailable)?,
        due_on: row.try_get("due_on").map_err(unavailable)?,
        state: row.try_get("state").map_err(unavailable)?,
        amount: amount.to_string(),
        currency,
        account_id: row.try_get("account_id").map_err(unavailable)?,
        category_id: row.try_get("category_id").map_err(unavailable)?,
        description: row.try_get("description").map_err(unavailable)?,
        transaction_id: row.try_get("transaction_id").map_err(unavailable)?,
    })
}

async fn read_rules(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    filters: RuleFilters,
) -> Result<RulePage, ApiError> {
    let limit = filters.limit.unwrap_or(DEFAULT_LIMIT);
    let cursor = filters
        .cursor
        .as_deref()
        .map(|value| normalize_uuid(value.to_owned()))
        .transpose()?;
    let mut tx = recurring_rules::begin(pool, identity).await?;
    if let Some(cursor) = &cursor {
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM recurring_rules WHERE id=$1::uuid AND user_id=$2::uuid)",
        )
        .bind(cursor)
        .bind(identity.user_id())
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if !owned {
            return Err(ApiError::InvalidRequest);
        }
    }
    let rows = sqlx::query(
        "SELECT id::text,amount_minor,currency,account_id::text,category_id::text,description,
                cadence,to_char(starts_on,'YYYY-MM-DD') AS starts_on,
                to_char(ends_on,'YYYY-MM-DD') AS ends_on,time_zone,state,
                to_char(paused_on,'YYYY-MM-DD') AS paused_on,
                to_char(generated_through,'YYYY-MM-DD') AS generated_through
         FROM recurring_rules
         WHERE user_id=$1::uuid AND ($2::text IS NULL OR state=$2)
           AND ($3::uuid IS NULL OR id>$3::uuid)
         ORDER BY id ASC LIMIT $4",
    )
    .bind(identity.user_id())
    .bind(filters.state.as_deref())
    .bind(cursor.as_deref())
    .bind(i64::from(limit) + 1)
    .fetch_all(&mut *tx)
    .await
    .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    let mut rules: Vec<_> = rows.iter().map(rule_from_row).collect::<Result<_, _>>()?;
    let has_more = rules.len() > usize::from(limit);
    if has_more {
        rules.pop();
    }
    let next_cursor = has_more
        .then(|| rules.last().map(|rule| rule.id().to_owned()))
        .flatten();
    Ok(RulePage { rules, next_cursor })
}

async fn read_occurrences(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    filters: OccurrenceFilters,
) -> Result<OccurrencePage, ApiError> {
    let limit = filters.limit.unwrap_or(DEFAULT_LIMIT);
    let (cursor_due_on, cursor_id) = filters
        .cursor
        .as_deref()
        .map(|value| {
            let (due_on, id) = value.split_once('~').ok_or(ApiError::InvalidRequest)?;
            BusinessDate::parse(due_on).map_err(|_| ApiError::InvalidRequest)?;
            let id = normalize_uuid(id.to_owned())?;
            Ok::<_, ApiError>((due_on.to_owned(), id))
        })
        .transpose()?
        .map_or((None, None), |(due_on, id)| (Some(due_on), Some(id)));
    let rule_id = filters
        .rule_id
        .as_deref()
        .map(|value| normalize_uuid(value.to_owned()))
        .transpose()?;
    let mut tx = recurring_rules::begin(pool, identity).await?;
    if let (Some(due_on), Some(id)) = (cursor_due_on.as_deref(), cursor_id.as_deref()) {
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM recurring_occurrences
             WHERE id=$1::uuid AND due_on=$2::date AND user_id=$3::uuid)",
        )
        .bind(id)
        .bind(due_on)
        .bind(identity.user_id())
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if !owned {
            return Err(ApiError::InvalidRequest);
        }
    }
    let rows = sqlx::query(
        "SELECT o.id::text,o.rule_id::text,to_char(o.scheduled_on,'YYYY-MM-DD') AS scheduled_on,
                to_char(o.due_on,'YYYY-MM-DD') AS due_on,o.state,o.amount_minor,o.currency,
                o.account_id::text,o.category_id::text,o.description,o.transaction_id::text
         FROM recurring_occurrences o
         WHERE o.user_id=$1::uuid AND ($2::text IS NULL OR o.state=$2)
           AND ($3::uuid IS NULL OR o.rule_id=$3::uuid)
           AND ($4::date IS NULL OR (o.due_on,o.id)>($4::date,$5::uuid))
         ORDER BY o.due_on ASC,o.id ASC LIMIT $6",
    )
    .bind(identity.user_id())
    .bind(filters.state.as_deref())
    .bind(rule_id.as_deref())
    .bind(cursor_due_on.as_deref())
    .bind(cursor_id.as_deref())
    .bind(i64::from(limit) + 1)
    .fetch_all(&mut *tx)
    .await
    .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    let mut occurrences: Vec<_> = rows
        .iter()
        .map(occurrence_from_row)
        .collect::<Result<_, _>>()?;
    let has_more = occurrences.len() > usize::from(limit);
    if has_more {
        occurrences.pop();
    }
    let next_cursor = has_more
        .then(|| {
            occurrences
                .last()
                .map(|occurrence| format!("{}~{}", occurrence.due_on, occurrence.id))
        })
        .flatten();
    Ok(OccurrencePage {
        occurrences,
        next_cursor,
    })
}

pub(crate) async fn list_rules(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(request_id): Extension<RequestId>,
    query: Result<Query<RuleFilters>, QueryRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let Query(filters) = match query {
        Ok(query) => query,
        Err(_) => return ApiError::InvalidRequest.response(&request_id),
    };
    let body = match body {
        Ok(body) => body,
        Err(error) => return ApiError::invalid_json(error.status()).response(&request_id),
    };
    if !body.is_empty() {
        return ApiError::InvalidRequest.response(&request_id);
    }
    if let Err(error) = filters.validate() {
        return error.response(&request_id);
    }
    let Some(pool) = pool else {
        return ApiError::RecurrenceUnavailable.response(&request_id);
    };
    match read_rules(&pool, &identity, filters).await {
        Ok(listing) => Json(listing).into_response(),
        Err(error) => error.response(&request_id),
    }
}

pub(crate) async fn list_occurrences(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(request_id): Extension<RequestId>,
    query: Result<Query<OccurrenceFilters>, QueryRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let Query(filters) = match query {
        Ok(query) => query,
        Err(_) => return ApiError::InvalidRequest.response(&request_id),
    };
    let body = match body {
        Ok(body) => body,
        Err(error) => return ApiError::invalid_json(error.status()).response(&request_id),
    };
    if !body.is_empty() {
        return ApiError::InvalidRequest.response(&request_id);
    }
    if let Err(error) = filters.validate() {
        return error.response(&request_id);
    }
    let Some(pool) = pool else {
        return ApiError::RecurrenceUnavailable.response(&request_id);
    };
    match read_occurrences(&pool, &identity, filters).await {
        Ok(listing) => Json(listing).into_response(),
        Err(error) => error.response(&request_id),
    }
}

pub(crate) async fn bound_query(request: axum::extract::Request, next: Next) -> Response {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_contracts_reject_invalid_ranges_states_duplicates_and_cursor_shapes() {
        for limit in [0, 101] {
            assert!(
                RuleFilters {
                    limit: Some(limit),
                    cursor: None,
                    state: None,
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            RuleFilters {
                limit: None,
                cursor: None,
                state: Some("ended".into()),
            }
            .validate()
            .is_err()
        );
        assert!(
            OccurrenceFilters {
                limit: None,
                cursor: Some("not-a-cursor".into()),
                state: None,
                rule_id: None,
            }
            .validate()
            .is_err()
        );
        assert!(
            OccurrenceFilters {
                limit: None,
                cursor: None,
                state: Some("active".into()),
                rule_id: None,
            }
            .validate()
            .is_err()
        );
    }
}
