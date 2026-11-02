//! Read-only realized event aggregates; postings are not income/spending.
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
            "/v1/reports/flows",
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

#[derive(Serialize)]
struct Flows {
    date_from: String,
    date_to: String,
    currencies: Vec<CurrencyFlow>,
}

#[derive(Serialize)]
struct CurrencyFlow {
    currency: String,
    income: String,
    expenses: String,
    net_flow: String,
}

// SUM(bigint) is an arbitrary-precision numeric integer. Format its decimal text
// without narrowing to transaction-sized i64, floating point or locale formatting.
pub(crate) fn decimal(minor: &str) -> Result<String, ApiError> {
    let (sign, digits) = minor.strip_prefix('-').map_or(("", minor), |s| ("-", s));
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::FlowReportUnavailable);
    }
    let padded = format!("{digits:0>3}");
    let (whole, fraction) = padded.split_at(padded.len() - 2);
    Ok(format!("{sign}{whole}.{fraction}"))
}

async fn read(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    period: Period,
) -> Result<Flows, ApiError> {
    let unavailable = |_| ApiError::FlowReportUnavailable;
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
    let verified: Option<bool> =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid")
            .bind(identity.user_id())
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    if verified != Some(true) {
        return Err(ApiError::EmailNotVerified);
    }
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "WITH totals AS (
            SELECT currency,
                coalesce(sum(amount_minor) FILTER (WHERE kind IN ('income','interest_income')),0) AS income,
                coalesce(sum(amount_minor) FILTER (WHERE kind='expense'),0) AS expenses
            FROM transactions
            WHERE user_id=$1::uuid AND state='posted'
                AND kind IN ('income','interest_income','expense')
                AND occurred_on >= $2::date AND occurred_on <= $3::date
            GROUP BY currency
        )
        SELECT c.currency,coalesce(t.income,0)::text,coalesce(t.expenses,0)::text,
            (coalesce(t.income,0)-coalesce(t.expenses,0))::text
        FROM (VALUES ('SEK',1),('EUR',2),('USD',3)) AS c(currency,position)
        LEFT JOIN totals t ON t.currency=c.currency ORDER BY c.position",
    ).bind(identity.user_id()).bind(&period.date_from).bind(&period.date_to)
        .fetch_all(&mut *tx).await.map_err(unavailable)?;
    let currencies = rows
        .into_iter()
        .map(|(currency, income, expenses, net)| {
            Ok(CurrencyFlow {
                currency,
                income: decimal(&income)?,
                expenses: decimal(&expenses)?,
                net_flow: decimal(&net)?,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    tx.commit().await.map_err(unavailable)?;
    Ok(Flows {
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
        Ok(Query(p))
            if BusinessDate::parse(&p.date_from).is_ok()
                && BusinessDate::parse(&p.date_to).is_ok()
                && p.date_from <= p.date_to =>
        {
            p
        }
        _ => return ApiError::InvalidRequest.response(&id),
    };
    match body {
        Ok(b) if b.is_empty() => {}
        Ok(_) => return ApiError::InvalidRequest.response(&id),
        Err(e) => return ApiError::invalid_json(e.status()).response(&id),
    }
    let Some(pool) = pool else {
        return ApiError::FlowReportUnavailable.response(&id);
    };
    match read(&pool, &identity, period).await {
        Ok(flows) => Json(flows).into_response(),
        Err(error) => error.response(&id),
    }
}

#[cfg(test)]
mod tests {
    use super::decimal;
    #[test]
    fn numeric_integer_totals_are_exact_signed_decimal_strings() {
        for (minor, expected) in [
            ("0", "0.00"),
            ("1", "0.01"),
            ("-1", "-0.01"),
            ("100", "1.00"),
            ("18446744073709551618", "184467440737095516.18"),
        ] {
            assert_eq!(decimal(minor).unwrap(), expected);
        }
        for invalid in ["", "-", "1.0", "1e2", " 1"] {
            assert!(decimal(invalid).is_err());
        }
    }
}
