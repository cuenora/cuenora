//! Owner-scoped realized spending and exact half-even category shares.
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
            "/v1/reports/expense-distribution",
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
struct Distribution {
    date_from: String,
    date_to: String,
    currencies: Vec<CurrencyDistribution>,
}

#[derive(Serialize)]
struct CurrencyDistribution {
    currency: String,
    expenses: String,
    categories: Vec<CategoryShare>,
}

#[derive(Serialize)]
struct CategoryShare {
    category_id: Option<String>,
    code: Option<String>,
    name: Option<String>,
    expenses: String,
    percentage: Option<String>,
}

type ShareRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
);

fn decimal(minor: &str) -> Result<String, ApiError> {
    crate::flow_reports::decimal(minor).map_err(|_| ApiError::ExpenseDistributionUnavailable)
}

async fn read(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    period: Period,
) -> Result<Distribution, ApiError> {
    let unavailable = |_| ApiError::ExpenseDistributionUnavailable;
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
    // div/mod operate on exact integer numerics: no finite-precision quotient
    // rounding before tie detection, even for aggregates exceeding i64.
    let rows: Vec<ShareRow> = sqlx::query_as(
        "WITH buckets AS (
            SELECT id,code,name FROM categories WHERE kind='expense'
            UNION ALL SELECT NULL::uuid,NULL::text,NULL::text
        ), totals AS (
            SELECT currency,category_id,sum(amount_minor) AS amount
            FROM transactions WHERE user_id=$1::uuid AND state='posted' AND kind='expense'
                AND occurred_on >= $2::date AND occurred_on <= $3::date
            GROUP BY currency,category_id
        ), amounts AS (
            SELECT c.currency,c.position,b.id,b.code,b.name,coalesce(t.amount,0) AS amount
            FROM (VALUES ('SEK',1),('EUR',2),('USD',3)) AS c(currency,position)
            CROSS JOIN buckets b LEFT JOIN totals t ON t.currency=c.currency
                AND t.category_id IS NOT DISTINCT FROM b.id
        ), denominators AS (
            SELECT *,sum(amount) OVER (PARTITION BY currency) AS total FROM amounts
        ), ratios AS (
            SELECT *,div(amount*10000,NULLIF(total,0)) AS q,
                mod(amount*10000,NULLIF(total,0)) AS r FROM denominators
        )
        SELECT currency,id::text,code,name,amount::text,total::text,
            CASE WHEN total=0 THEN NULL ELSE
                (q+CASE WHEN r*2>total OR (r*2=total AND mod(q,2)=1) THEN 1 ELSE 0 END)::text
            END
        FROM ratios ORDER BY position,id NULLS LAST",
    )
    .bind(identity.user_id())
    .bind(&period.date_from)
    .bind(&period.date_to)
    .fetch_all(&mut *tx)
    .await
    .map_err(unavailable)?;
    let mut currencies = Vec::new();
    for currency in ["SEK", "EUR", "USD"] {
        let mut expenses = "0.00".to_owned();
        let mut categories = Vec::new();
        for (code_currency, category_id, code, name, amount, total, share) in &rows {
            if code_currency != currency {
                continue;
            }
            expenses = decimal(total)?;
            categories.push(CategoryShare {
                category_id: category_id.clone(),
                code: code.clone(),
                name: name.clone(),
                expenses: decimal(amount)?,
                percentage: share.as_deref().map(decimal).transpose()?,
            });
        }
        currencies.push(CurrencyDistribution {
            currency: currency.to_owned(),
            expenses,
            categories,
        });
    }
    tx.commit().await.map_err(unavailable)?;
    Ok(Distribution {
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
        return ApiError::ExpenseDistributionUnavailable.response(&id);
    };
    match read(&pool, &identity, period).await {
        Ok(flows) => Json(flows).into_response(),
        Err(error) => error.response(&id),
    }
}
