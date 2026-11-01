//! Atomic expense with optional posting, minimal audit and independent permanent retries.
use crate::{
    account_creation::{CreationError, today_at},
    auth_identity::AuthenticatedIdentity,
    profile_zone::parse_zone,
};
use chrono::{DateTime, Utc};
use cuenora_domain::{BusinessDate, Money};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

pub(crate) struct CreateExpense {
    pub amount: Money,
    pub occurred_on: BusinessDate,
    pub category_id: Option<String>,
    pub account_id: Option<String>,
    pub description: Option<String>,
}

impl CreateExpense {
    // Length framing plus presence markers prevents ambiguous concatenation.
    // Hash the normalized request, not a future mutable transaction snapshot.
    pub(crate) fn fingerprint(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        // Keep all existing accountless hashes compatible with permanent keys.
        digest.update(if self.account_id.is_some() {
            b"linked-expense-v1".as_slice()
        } else {
            b"accountless-expense-v1".as_slice()
        });
        for field in [
            Some(self.amount.to_string()),
            Some(self.amount.currency().to_string()),
            Some(self.occurred_on.to_string()),
            self.category_id.clone(),
            self.description.clone(),
        ] {
            match field {
                Some(field) => {
                    digest.update([1]);
                    digest.update((field.len() as u64).to_be_bytes());
                    digest.update(field.as_bytes());
                }
                None => digest.update([0]),
            }
        }
        if let Some(account) = &self.account_id {
            digest.update((account.len() as u64).to_be_bytes());
            digest.update(account.as_bytes());
        }
        digest.finalize().into()
    }

    pub(crate) fn result(&self, id: String) -> CreatedExpense {
        CreatedExpense {
            id,
            kind: "expense",
            state: "posted",
            amount: self.amount.to_string(),
            currency: self.amount.currency().to_string(),
            occurred_on: self.occurred_on.to_string(),
            account_id: self.account_id.clone(),
            category_id: self.category_id.clone(),
            description: self.description.clone(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CreatedExpense {
    pub(crate) id: String,
    kind: &'static str,
    state: &'static str,
    amount: String,
    currency: String,
    occurred_on: String,
    account_id: Option<String>,
    category_id: Option<String>,
    description: Option<String>,
}

pub(crate) enum ExpenseError {
    Unverified,
    Conflict,
    InvalidCategory,
    InvalidAccount,
    NotFound,
    UnconfirmedZone,
    UnsupportedZone,
    DateOutOfRange,
    FutureDate,
    Unavailable,
}
impl From<sqlx::Error> for ExpenseError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

pub(crate) async fn create(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    key_hash: &[u8; 32],
    input: &CreateExpense,
) -> Result<CreatedExpense, ExpenseError> {
    let (mut tx, zone, captured) = begin(pool, identity).await?;
    let fingerprint = input.fingerprint();
    let prior: Option<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT transaction_id::text,request_hash FROM expense_creation_keys WHERE user_id=$1::uuid AND key_hash=$2",
    ).bind(identity.user_id()).bind(key_hash.as_slice()).fetch_optional(&mut *tx).await?;
    if let Some((id, previous)) = prior {
        if previous != fingerprint {
            return Err(ExpenseError::Conflict);
        }
        tx.commit().await?;
        return Ok(input.result(id));
    }
    let expense = post(&mut tx, identity, input, zone.as_deref(), captured).await?;
    let id = &expense.id;
    sqlx::query(
        "INSERT INTO expense_creation_keys(user_id,key_hash,request_hash,transaction_id)
         VALUES($1::uuid,$2,$3,$4::uuid)",
    )
    .bind(identity.user_id())
    .bind(key_hash.as_slice())
    .bind(fingerprint.as_slice())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(expense)
}

/// Shared owner serialization; callers keep every related write in this transaction.
pub(crate) async fn begin<'a>(
    pool: &'a PgPool,
    identity: &AuthenticatedIdentity,
) -> Result<(Transaction<'a, Postgres>, Option<String>, DateTime<Utc>), ExpenseError> {
    let captured: DateTime<Utc> = std::time::SystemTime::now().into();
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    // Same serialization boundary as account creation and profile changes.
    let owner: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT email_verified_at IS NOT NULL,profile_time_zone FROM users WHERE id=$1::uuid FOR UPDATE",
    ).bind(identity.user_id()).fetch_optional(&mut *tx).await?;
    let (true, zone) = owner.ok_or(ExpenseError::Unverified)? else {
        return Err(ExpenseError::Unverified);
    };
    Ok((tx, zone, captured))
}

/// New posting only. The caller owns retry admission and commit/rollback.
pub(crate) async fn post(
    connection: &mut PgConnection,
    identity: &AuthenticatedIdentity,
    input: &CreateExpense,
    zone: Option<&str>,
    captured: DateTime<Utc>,
) -> Result<CreatedExpense, ExpenseError> {
    let zone = zone.ok_or(ExpenseError::UnconfirmedZone)?;
    let zone = parse_zone(zone).map_err(|_| ExpenseError::UnsupportedZone)?;
    let today = today_at(captured, zone).map_err(|error| match error {
        CreationError::DateOutOfRange => ExpenseError::DateOutOfRange,
        _ => ExpenseError::Unavailable,
    })?;
    input
        .occurred_on
        .ensure_not_future(today)
        .map_err(|_| ExpenseError::FutureDate)?;
    if let Some(category) = &input.category_id {
        let compatible: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE id=$1::uuid AND kind='expense')",
        )
        .bind(category)
        .fetch_one(&mut *connection)
        .await?;
        if !compatible {
            return Err(ExpenseError::InvalidCategory);
        }
    }
    if let Some(account) = &input.account_id {
        // Follow owner→account ordering shared with archive and income posting.
        // Callers admit matching retries before entering this new-posting path.
        let source: Option<(String, bool)> = sqlx::query_as(
            "SELECT currency,archived_at IS NULL FROM accounts
             WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE",
        )
        .bind(account)
        .bind(identity.user_id())
        .fetch_optional(&mut *connection)
        .await?;
        let (currency, active) = source.ok_or(ExpenseError::NotFound)?;
        if currency != input.amount.currency().to_string() || !active {
            return Err(ExpenseError::InvalidAccount);
        }
    }
    let id: String = sqlx::query_scalar(
        "INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,category_id,description,account_id)
         VALUES($1::uuid,'expense',$2::date,$3,$4,$5::uuid,$6,$7::uuid) RETURNING id::text",
    ).bind(identity.user_id()).bind(input.occurred_on.to_string()).bind(input.amount.minor_units())
     .bind(input.amount.currency().to_string()).bind(&input.category_id).bind(&input.description).bind(&input.account_id)
     .fetch_one(&mut *connection).await?;
    if let Some(account) = &input.account_id {
        sqlx::query(
            "INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on)
             VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5,$6::date)",
        ).bind(identity.user_id()).bind(&id).bind(account).bind(-input.amount.minor_units())
         .bind(input.amount.currency().to_string()).bind(input.occurred_on.to_string())
         .execute(&mut *connection).await?;
    }
    sqlx::query(
        "INSERT INTO transaction_audit(user_id,actor_id,transaction_id,action,changed_fields)
         VALUES($1::uuid,$1::uuid,$2::uuid,'create',ARRAY[]::text[])",
    )
    .bind(identity.user_id())
    .bind(&id)
    .execute(&mut *connection)
    .await?;
    Ok(input.result(id))
}
