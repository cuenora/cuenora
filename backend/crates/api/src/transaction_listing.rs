//! Bounded recorded history. Never infer balances or authority from a cursor.
use crate::{
    account_listing::valid_cursor, api_error::ApiError, auth_identity::AuthenticatedIdentity,
    request_id::RequestId,
};
use axum::{
    Extension, Json,
    body::Bytes,
    extract::Request,
    extract::{
        Query, State,
        rejection::{BytesRejection, QueryRejection},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use cuenora_domain::{BusinessDate, Currency, Money};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Filters {
    limit: Option<u16>,
    cursor: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    category_id: Option<String>,
    account_id: Option<String>,
}

impl Filters {
    fn validate(&self) -> Result<(), ApiError> {
        if !(1..=100).contains(&self.limit.unwrap_or(50)) {
            return Err(ApiError::InvalidRequest);
        }
        for id in [
            self.cursor.as_deref(),
            self.category_id.as_deref(),
            self.account_id.as_deref().filter(|id| *id != "none"),
        ]
        .into_iter()
        .flatten()
        {
            if !valid_cursor(id) {
                return Err(ApiError::InvalidRequest);
            }
        }
        for date in [self.date_from.as_deref(), self.date_to.as_deref()]
            .into_iter()
            .flatten()
        {
            BusinessDate::parse(date).map_err(|_| ApiError::InvalidRequest)?;
        }
        if matches!((&self.date_from, &self.date_to), (Some(from), Some(to)) if from > to) {
            return Err(ApiError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct Listing {
    transactions: Vec<RecordedTransaction>,
    next_cursor: Option<String>,
}

#[derive(Serialize)]
struct RecordedTransaction {
    id: String,
    kind: String,
    state: String,
    amount: String,
    currency: String,
    occurred_on: String,
    account_id: Option<String>,
    category_id: Option<String>,
    description: Option<String>,
    destination_account_id: Option<String>,
    received_amount: Option<String>,
    received_currency: Option<String>,
}

type TransactionRow = (
    String,
    String,
    String,
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<String>,
);

fn money(minor: i64, currency: &str) -> Result<String, ApiError> {
    let currency = currency
        .parse::<Currency>()
        .map_err(|_| ApiError::TransactionListingUnavailable)?;
    Money::from_minor_units(minor, currency)
        .map(|money| money.to_string())
        .map_err(|_| ApiError::TransactionListingUnavailable)
}

impl TryFrom<TransactionRow> for RecordedTransaction {
    type Error = ApiError;
    fn try_from(row: TransactionRow) -> Result<Self, Self::Error> {
        let (
            id,
            kind,
            state,
            minor,
            currency,
            occurred_on,
            account_id,
            category_id,
            description,
            destination_account_id,
            received_minor,
            received_currency,
        ) = row;
        let amount = money(minor, &currency)?;
        let received_amount = match (received_minor, received_currency.as_deref()) {
            (Some(minor), Some(currency)) => Some(money(minor, currency)?),
            (None, None) => None,
            _ => return Err(ApiError::TransactionListingUnavailable),
        };
        Ok(Self {
            id,
            kind,
            state,
            amount,
            currency,
            occurred_on,
            account_id,
            category_id,
            description,
            destination_account_id,
            received_amount,
            received_currency,
        })
    }
}

// Share the exact owner/filter predicate between cursor admission and page reads.
// Every interpolation below is static SQL, never client-supplied text.
const FILTER: &str = "t.user_id=$1::uuid
    AND ($2::text IS NULL OR t.occurred_on >= $2::date)
    AND ($3::text IS NULL OR t.occurred_on <= $3::date)
    AND ($4::uuid IS NULL OR t.category_id=$4::uuid)
    AND (NOT $5::bool OR t.account_id IS NULL)
    AND ($6::uuid IS NULL OR t.account_id=$6::uuid OR t.destination_account_id=$6::uuid)";

async fn read(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    filters: Filters,
) -> Result<Listing, ApiError> {
    let unavailable = |_| ApiError::TransactionListingUnavailable;
    let mut tx = pool.begin().await.map_err(unavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("SET LOCAL lock_timeout='2s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("SET LOCAL statement_timeout='5s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    let verified: Option<bool> =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid")
            .bind(identity.user_id())
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    if verified != Some(true) {
        return Err(ApiError::EmailNotVerified);
    }
    let accountless = filters.account_id.as_deref() == Some("none");
    let account = filters.account_id.as_deref().filter(|_| !accountless);
    if let Some(account) = account {
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE user_id=$1::uuid AND id=$2::uuid)",
        )
        .bind(identity.user_id())
        .bind(account)
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if !owned {
            return Err(ApiError::InvalidRequest);
        }
    }
    if let Some(category) = &filters.category_id {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM categories WHERE id=$1::uuid)")
                .bind(category)
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?;
        if !exists {
            return Err(ApiError::InvalidRequest);
        }
    }
    if let Some(cursor) = &filters.cursor {
        let sql =
            format!("SELECT EXISTS(SELECT 1 FROM transactions t WHERE {FILTER} AND t.id=$7::uuid)");
        let admitted: bool = sqlx::query_scalar(&sql)
            .bind(identity.user_id())
            .bind(&filters.date_from)
            .bind(&filters.date_to)
            .bind(&filters.category_id)
            .bind(accountless)
            .bind(account)
            .bind(cursor)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
        if !admitted {
            return Err(ApiError::InvalidRequest);
        }
    }
    let limit = filters.limit.unwrap_or(50);
    let sql = format!("SELECT t.id::text,t.kind,t.state,t.amount_minor,t.currency,
            to_char(t.occurred_on,'YYYY-MM-DD'),t.account_id::text,t.category_id::text,
            t.description,t.destination_account_id::text,t.received_amount_minor,t.received_currency
        FROM transactions t WHERE {FILTER}
        AND ($7::uuid IS NULL OR (t.occurred_on,t.id) < (
            SELECT c.occurred_on,c.id FROM transactions c WHERE c.user_id=$1::uuid AND c.id=$7::uuid))
        ORDER BY t.occurred_on DESC,t.id DESC LIMIT $8");
    let mut rows: Vec<TransactionRow> = sqlx::query_as(&sql)
        .bind(identity.user_id())
        .bind(&filters.date_from)
        .bind(&filters.date_to)
        .bind(&filters.category_id)
        .bind(accountless)
        .bind(account)
        .bind(&filters.cursor)
        .bind(i64::from(limit) + 1)
        .fetch_all(&mut *tx)
        .await
        .map_err(unavailable)?;
    let has_more = rows.len() > usize::from(limit);
    rows.truncate(usize::from(limit));
    let transactions = rows
        .into_iter()
        .map(RecordedTransaction::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    let next_cursor = if has_more {
        transactions.last().map(|row| row.id.clone())
    } else {
        None
    };
    tx.commit().await.map_err(unavailable)?;
    Ok(Listing {
        transactions,
        next_cursor,
    })
}

pub(crate) async fn bound_query(request: Request, next: Next) -> Response {
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

pub(crate) async fn list(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    query: Result<Query<Filters>, QueryRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let Query(filters) = match query {
        Ok(query) => query,
        Err(_) => return ApiError::InvalidRequest.response(&id),
    };
    let body = match body {
        Ok(body) => body,
        Err(error) => return ApiError::invalid_json(error.status()).response(&id),
    };
    if !body.is_empty() {
        return ApiError::InvalidRequest.response(&id);
    }
    if let Err(error) = filters.validate() {
        return error.response(&id);
    }
    let Some(pool) = pool else {
        return ApiError::TransactionListingUnavailable.response(&id);
    };
    match read(&pool, &identity, filters).await {
        Ok(listing) => Json(listing).into_response(),
        Err(error) => error.response(&id),
    }
}
