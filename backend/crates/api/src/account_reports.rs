//! Corrected-ledger history and current-snapshot planning transport.
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
        DefaultBodyLimit, Path, Query, State,
        rejection::{BytesRejection, PathRejection, QueryRejection},
    },
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use cuenora_domain::BusinessDate;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};

pub(crate) type Tx<'a> = Transaction<'a, Postgres>;
pub(crate) fn unavailable(_: sqlx::Error) -> ApiError {
    ApiError::AccountReportUnavailable
}
pub(crate) fn money(value: &str) -> Result<String, ApiError> {
    crate::flow_reports::decimal(value).map_err(|_| ApiError::AccountReportUnavailable)
}
pub(crate) async fn begin<'a>(
    pool: &'a PgPool,
    identity: &AuthenticatedIdentity,
) -> Result<(Tx<'a>, Option<String>), ApiError> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    for statement in [
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY",
        "SET LOCAL lock_timeout='2s'",
        "SET LOCAL statement_timeout='5s'",
    ] {
        sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
    }
    let owner: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT email_verified_at IS NOT NULL,profile_time_zone FROM users WHERE id=$1::uuid",
    )
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    match owner {
        Some((true, zone)) => Ok((tx, zone)),
        _ => Err(ApiError::EmailNotVerified),
    }
}
pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route("/v1/accounts/{id}/balance-history", get(history))
        .route("/v1/reports/account-planning", get(planning))
        .route_layer(middleware::from_fn(crate::transaction_listing::bound_query))
        .route_layer(DefaultBodyLimit::max(8 * 1024))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ))
        .with_state(pool)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Period {
    date_from: String,
    date_to: String,
}
impl Period {
    fn valid(&self) -> bool {
        if BusinessDate::parse(&self.date_from).is_err()
            || BusinessDate::parse(&self.date_to).is_err()
        {
            return false;
        }
        match (
            NaiveDate::parse_from_str(&self.date_from, "%Y-%m-%d"),
            NaiveDate::parse_from_str(&self.date_to, "%Y-%m-%d"),
        ) {
            (Ok(from), Ok(to)) => (0..366).contains(&to.signed_duration_since(from).num_days()),
            _ => false,
        }
    }
}
#[derive(Serialize)]
struct History {
    as_of: String,
    account_id: String,
    currency: String,
    status: &'static str,
    date_from: String,
    date_to: String,
    history_basis: &'static str,
    opening_balance: String,
    days: Vec<Day>,
}
#[derive(Serialize)]
struct Day {
    date: String,
    balance: String,
}
async fn read_history(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    account: String,
    period: Period,
) -> Result<History, ApiError> {
    let (mut tx, _) = begin(pool, identity).await?;
    let as_of: DateTime<Utc> = std::time::SystemTime::now().into();
    let (currency, active): (String, bool) = sqlx::query_as(
        "SELECT currency,archived_at IS NULL FROM accounts WHERE id=$1::uuid AND user_id=$2::uuid",
    )
    .bind(&account)
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?
    .ok_or(ApiError::NotFound)?;
    let rows: Vec<(String,String,String)> = sqlx::query_as(
        "WITH opening AS (
            SELECT coalesce(sum(signed_amount_minor),0) AS amount FROM account_movements
            WHERE user_id=$1::uuid AND account_id=$2::uuid AND currency=$5 AND occurred_on<$3::date
        ), daily AS (
            SELECT occurred_on,sum(signed_amount_minor) AS amount FROM account_movements
            WHERE user_id=$1::uuid AND account_id=$2::uuid AND currency=$5
                AND occurred_on >= $3::date AND occurred_on <= $4::date GROUP BY occurred_on
        )
        SELECT to_char(s.day,'YYYY-MM-DD'),o.amount::text,
            (o.amount+sum(coalesce(d.amount,0)) OVER (ORDER BY s.day ROWS UNBOUNDED PRECEDING))::text
        FROM generate_series($3::date::timestamp,$4::date::timestamp,interval '1 day') s(day)
        CROSS JOIN opening o LEFT JOIN daily d ON d.occurred_on=s.day::date ORDER BY s.day",
    ).bind(identity.user_id()).bind(&account).bind(&period.date_from).bind(&period.date_to).bind(&currency)
        .fetch_all(&mut *tx).await.map_err(unavailable)?;
    let opening_balance = money(&rows.first().ok_or(ApiError::AccountReportUnavailable)?.1)?;
    let days = rows
        .into_iter()
        .map(|(date, _, amount)| {
            Ok(Day {
                date,
                balance: money(&amount)?,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    tx.commit().await.map_err(unavailable)?;
    Ok(History {
        as_of: as_of.to_rfc3339_opts(SecondsFormat::Micros, true),
        account_id: account,
        currency,
        status: if active { "active" } else { "archived" },
        date_from: period.date_from,
        date_to: period.date_to,
        history_basis: "current_corrected_ledger",
        opening_balance,
        days,
    })
}
fn empty_body(body: Result<Bytes, BytesRejection>) -> Result<(), ApiError> {
    match body {
        Ok(b) if b.is_empty() => Ok(()),
        Ok(_) => Err(ApiError::InvalidRequest),
        Err(e) => Err(ApiError::invalid_json(e.status())),
    }
}
async fn history(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Period>, QueryRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let account = match path
        .map_err(|_| ApiError::InvalidRequest)
        .and_then(|Path(s)| normalize_uuid(s))
    {
        Ok(s) => s,
        Err(e) => return e.response(&id),
    };
    let period = match query {
        Ok(Query(p)) if p.valid() => p,
        _ => return ApiError::InvalidRequest.response(&id),
    };
    if let Err(e) = empty_body(body) {
        return e.response(&id);
    }
    let Some(pool) = pool else {
        return ApiError::AccountReportUnavailable.response(&id);
    };
    match read_history(&pool, &identity, account, period).await {
        Ok(output) => Json(output).into_response(),
        Err(e) => e.response(&id),
    }
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Cutoff {
    through: Option<String>,
}
async fn planning(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    query: Result<Query<Cutoff>, QueryRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let cutoff = match query {
        Ok(Query(c))
            if c.through
                .as_deref()
                .is_none_or(|d| BusinessDate::parse(d).is_ok()) =>
        {
            c
        }
        _ => return ApiError::InvalidRequest.response(&id),
    };
    if let Err(e) = empty_body(body) {
        return e.response(&id);
    }
    let Some(pool) = pool else {
        return ApiError::AccountReportUnavailable.response(&id);
    };
    match crate::account_planning::read(&pool, &identity, cutoff.through).await {
        Ok(output) => Json(output).into_response(),
        Err(e) => e.response(&id),
    }
}
