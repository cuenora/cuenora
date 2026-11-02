//! Bounded calendar-month realized flows; not account balance history.
use crate::{
    api_error::ApiError,
    auth_identity::{AuthenticatedIdentity, require_identity},
    request_id::RequestId,
};
use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, Query, State,
        rejection::{BytesRejection, QueryRejection},
    },
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use cuenora_domain::BusinessDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route(
            "/v1/reports/monthly-flows",
            get(report).layer(middleware::from_fn(crate::transaction_listing::bound_query)),
        )
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
            || self.date_from > self.date_to
        {
            return false;
        }
        let index = |date: &str| -> Option<u32> {
            date.get(..4)?
                .parse::<u32>()
                .ok()?
                .checked_mul(12)?
                .checked_add(date.get(5..7)?.parse::<u32>().ok()?)
        };
        match (index(&self.date_from), index(&self.date_to)) {
            (Some(from), Some(to)) => to
                .checked_sub(from)
                .is_some_and(|difference| difference < 120),
            _ => false,
        }
    }
}

#[derive(Serialize)]
struct MonthlyFlows {
    date_from: String,
    date_to: String,
    currencies: Vec<CurrencyHistory>,
}

#[derive(Serialize)]
struct CurrencyHistory {
    currency: String,
    months: Vec<MonthFlow>,
}

#[derive(Serialize)]
struct MonthFlow {
    month: String,
    date_from: String,
    date_to: String,
    income: String,
    expenses: String,
    net_flow: String,
}

type MonthRow = (String, String, String, String, String, String, String);

fn decimal(minor: &str) -> Result<String, ApiError> {
    crate::flow_reports::decimal(minor).map_err(|_| ApiError::MonthlyFlowsUnavailable)
}

async fn read(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    period: Period,
) -> Result<MonthlyFlows, ApiError> {
    let unavailable = |_| ApiError::MonthlyFlowsUnavailable;
    let mut tx = pool.begin().await.map_err(unavailable)?;
    for statement in ["SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY"] {
        sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
    }
    let verified: Option<bool> =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid")
            .bind(identity.user_id())
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    if verified != Some(true) {
        return Err(ApiError::EmailNotVerified);
    }
    // Explicit timestamp WITHOUT time zone keeps civil month boundaries independent
    // of session time zone/DST. Validation bounds series length before any query.
    let rows: Vec<MonthRow> = sqlx::query_as(
        "WITH months AS (
            SELECT generate_series(date_trunc('month',$2::date::timestamp),
                date_trunc('month',$3::date::timestamp),interval '1 month') AS month
        ), totals AS (
            SELECT currency,date_trunc('month',occurred_on::timestamp) AS month,
                coalesce(sum(amount_minor) FILTER (WHERE kind IN ('income','interest_income')),0) AS income,
                coalesce(sum(amount_minor) FILTER (WHERE kind='expense'),0) AS expenses
            FROM transactions WHERE user_id=$1::uuid AND state='posted'
                AND kind IN ('income','interest_income','expense')
                AND occurred_on >= $2::date AND occurred_on <= $3::date
            GROUP BY currency,date_trunc('month',occurred_on::timestamp)
        )
        SELECT c.currency,to_char(m.month,'YYYY-MM'),
            to_char(greatest(m.month::date,$2::date),'YYYY-MM-DD'),
            to_char(least((m.month+interval '1 month')::date-1,$3::date),'YYYY-MM-DD'),
            coalesce(t.income,0)::text,coalesce(t.expenses,0)::text,
            (coalesce(t.income,0)-coalesce(t.expenses,0))::text
        FROM months m CROSS JOIN (VALUES ('SEK',1),('EUR',2),('USD',3)) AS c(currency,position)
        LEFT JOIN totals t ON t.currency=c.currency AND t.month=m.month
        ORDER BY c.position,m.month",
    ).bind(identity.user_id()).bind(&period.date_from).bind(&period.date_to)
        .fetch_all(&mut *tx).await.map_err(unavailable)?;
    let mut currencies = Vec::new();
    for currency in ["SEK", "EUR", "USD"] {
        let mut months = Vec::new();
        for (row_currency, month, from, to, income, expenses, net) in &rows {
            if row_currency != currency {
                continue;
            }
            months.push(MonthFlow {
                month: month.clone(),
                date_from: from.clone(),
                date_to: to.clone(),
                income: decimal(income)?,
                expenses: decimal(expenses)?,
                net_flow: decimal(net)?,
            });
        }
        currencies.push(CurrencyHistory {
            currency: currency.to_owned(),
            months,
        });
    }
    tx.commit().await.map_err(unavailable)?;
    Ok(MonthlyFlows {
        date_from: period.date_from,
        date_to: period.date_to,
        currencies,
    })
}

async fn report(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    query: Result<Query<Period>, QueryRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let period = match query {
        Ok(Query(period)) if period.valid() => period,
        _ => return ApiError::InvalidRequest.response(&id),
    };
    match body {
        Ok(b) if b.is_empty() => {}
        Ok(_) => return ApiError::InvalidRequest.response(&id),
        Err(e) => return ApiError::invalid_json(e.status()).response(&id),
    }
    let Some(pool) = pool else {
        return ApiError::MonthlyFlowsUnavailable.response(&id);
    };
    match read(&pool, &identity, period).await {
        Ok(flows) => Json(flows).into_response(),
        Err(error) => error.response(&id),
    }
}
