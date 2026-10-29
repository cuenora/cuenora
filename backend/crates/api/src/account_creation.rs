//! Account creation use case: validated domain input and atomic owned persistence.
use crate::auth_identity::AuthenticatedIdentity;
use cuenora_domain::{AccountName, AccountType, Currency};
use serde::Serialize;
use sqlx::PgPool;

pub(crate) struct CreateAccount {
    pub name: AccountName,
    pub account_type: AccountType,
    pub currency: Currency,
}
impl CreateAccount {
    fn kind(&self) -> &'static str {
        match self.account_type {
            AccountType::Normal => "normal",
            AccountType::Savings => "savings",
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CreatedAccount {
    id: String,
    name: String,
    account_type: String,
    currency: String,
    status: &'static str,
}

pub(crate) enum CreationError {
    Unverified,
    Conflict,
    Unavailable,
}
impl From<sqlx::Error> for CreationError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

/// The identity must come from bearer middleware. Serialize creations per owner
/// under the user row lock, checking verification before either replay or write.
/// This avoids advisory-lock hash collisions and speculative account inserts.
pub(crate) async fn create(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    key_hash: &[u8; 32],
    input: &CreateAccount,
) -> Result<CreatedAccount, CreationError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    let verified: Option<bool> = sqlx::query_scalar(
        "SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid FOR UPDATE",
    )
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await?;
    if verified != Some(true) {
        return Err(CreationError::Unverified);
    }
    let currency = input.currency.to_string();
    let prior: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT account_id::text,name,account_type,currency FROM account_creation_keys
         WHERE user_id=$1::uuid AND key_hash=$2",
    )
    .bind(identity.user_id())
    .bind(key_hash.as_slice())
    .fetch_optional(&mut *tx)
    .await?;
    let id = if let Some((id, name, kind, code)) = prior {
        if name != input.name.as_str() || kind != input.kind() || code != currency {
            return Err(CreationError::Conflict);
        }
        id
    } else {
        let id: String = sqlx::query_scalar(
            "INSERT INTO accounts (user_id,name,account_type,currency)
             VALUES ($1::uuid,$2,$3,$4) RETURNING id::text",
        )
        .bind(identity.user_id())
        .bind(input.name.as_str())
        .bind(input.kind())
        .bind(&currency)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO account_creation_keys (user_id,key_hash,account_id,name,account_type,currency)
             VALUES ($1::uuid,$2,$3::uuid,$4,$5,$6)",
        ).bind(identity.user_id()).bind(key_hash.as_slice()).bind(&id)
         .bind(input.name.as_str()).bind(input.kind()).bind(&currency).execute(&mut *tx).await?;
        id
    };
    tx.commit().await?;
    Ok(CreatedAccount {
        id,
        name: input.name.as_str().to_owned(),
        account_type: input.kind().into(),
        currency,
        status: "active",
    })
}
