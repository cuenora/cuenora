//! Bounded, read-only account listing scoped to one verified owner.
use crate::auth_identity::AuthenticatedIdentity;
use serde::Serialize;
use sqlx::PgPool;

const DEFAULT_LIMIT: u16 = 50;
const MAX_LIMIT: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListingError {
    EmailNotVerified,
    InvalidRequest,
    Unavailable,
}

#[derive(Serialize)]
pub(crate) struct AccountSummary {
    id: String,
    name: String,
    account_type: String,
    currency: String,
    status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct AccountListing {
    accounts: Vec<AccountSummary>,
    next_cursor: Option<String>,
}

struct AccountRow {
    id: String,
    name: String,
    account_type: String,
    currency: String,
    active: bool,
}

impl From<(String, String, String, String, bool)> for AccountRow {
    fn from(row: (String, String, String, String, bool)) -> Self {
        Self {
            id: row.0,
            name: row.1,
            account_type: row.2,
            currency: row.3,
            active: row.4,
        }
    }
}

pub(crate) fn valid_cursor(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

pub(crate) async fn list(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    requested_limit: Option<u16>,
    cursor: Option<&str>,
) -> Result<AccountListing, ListingError> {
    let limit = requested_limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(ListingError::InvalidRequest);
    }

    let mut tx = pool.begin().await.map_err(|_| ListingError::Unavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| ListingError::Unavailable)?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| ListingError::Unavailable)?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| ListingError::Unavailable)?;

    let verified: Option<bool> =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id = $1::uuid")
            .bind(identity.user_id())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| ListingError::Unavailable)?;
    if verified != Some(true) {
        return Err(ListingError::EmailNotVerified);
    }

    if let Some(cursor) = cursor {
        if !valid_cursor(cursor) {
            return Err(ListingError::InvalidRequest);
        }
        let owned_cursor: Option<String> = sqlx::query_scalar(
            "SELECT id::text FROM accounts WHERE user_id = $1::uuid AND id = $2::uuid",
        )
        .bind(identity.user_id())
        .bind(cursor)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| ListingError::Unavailable)?;
        if owned_cursor.is_none() {
            return Err(ListingError::InvalidRequest);
        }
    }

    let rows: Vec<(String, String, String, String, bool)> = sqlx::query_as(
        "SELECT a.id::text, a.name, a.account_type, a.currency, a.archived_at IS NULL
         FROM accounts a
         WHERE a.user_id = $1::uuid
           AND ($2::uuid IS NULL OR (a.created_at, a.id) > (
                SELECT c.created_at, c.id FROM accounts c
                WHERE c.user_id = $1::uuid AND c.id = $2::uuid
           ))
         ORDER BY a.created_at ASC, a.id ASC
         LIMIT $3",
    )
    .bind(identity.user_id())
    .bind(cursor)
    .bind(i64::from(limit) + 1)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| ListingError::Unavailable)?;

    let mut rows: Vec<AccountRow> = rows.into_iter().map(AccountRow::from).collect();
    let has_more = rows.len() > usize::from(limit);
    if has_more {
        rows.truncate(usize::from(limit));
    }
    let next_cursor = if has_more {
        rows.last().map(|row| row.id.clone())
    } else {
        None
    };
    let accounts = rows
        .into_iter()
        .map(|row| AccountSummary {
            id: row.id,
            name: row.name,
            account_type: row.account_type,
            currency: row.currency,
            status: if row.active { "active" } else { "archived" },
        })
        .collect();

    tx.commit().await.map_err(|_| ListingError::Unavailable)?;
    Ok(AccountListing {
        accounts,
        next_cursor,
    })
}
