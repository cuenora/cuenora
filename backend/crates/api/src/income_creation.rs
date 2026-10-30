//! One accountless income event, minimal audit and permanent retry equivalence.
use crate::{
    account_creation::{CreationError, today_at},
    auth_identity::AuthenticatedIdentity,
    profile_zone::parse_zone,
};
use chrono::{DateTime, Utc};
use cuenora_domain::{BusinessDate, Money};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

pub(crate) struct CreateIncome {
    pub amount: Money,
    pub occurred_on: BusinessDate,
    pub category_id: Option<String>,
    pub description: Option<String>,
}

impl CreateIncome {
    // Length framing plus presence markers prevents ambiguous concatenation.
    // Hash the normalized request, not a future mutable transaction snapshot.
    fn fingerprint(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"accountless-income-v1");
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
        digest.finalize().into()
    }

    fn result(&self, id: String) -> CreatedIncome {
        CreatedIncome {
            id,
            kind: "income",
            state: "posted",
            amount: self.amount.to_string(),
            currency: self.amount.currency().to_string(),
            occurred_on: self.occurred_on.to_string(),
            account_id: None,
            category_id: self.category_id.clone(),
            description: self.description.clone(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CreatedIncome {
    id: String,
    kind: &'static str,
    state: &'static str,
    amount: String,
    currency: String,
    occurred_on: String,
    account_id: Option<String>,
    category_id: Option<String>,
    description: Option<String>,
}

pub(crate) enum IncomeError {
    Unverified,
    Conflict,
    InvalidCategory,
    UnconfirmedZone,
    UnsupportedZone,
    DateOutOfRange,
    FutureDate,
    Unavailable,
}
impl From<sqlx::Error> for IncomeError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

pub(crate) async fn create(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    key_hash: &[u8; 32],
    input: &CreateIncome,
) -> Result<CreatedIncome, IncomeError> {
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
    let (true, zone) = owner.ok_or(IncomeError::Unverified)? else {
        return Err(IncomeError::Unverified);
    };
    let fingerprint = input.fingerprint();
    let prior: Option<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT transaction_id::text,request_hash FROM income_creation_keys WHERE user_id=$1::uuid AND key_hash=$2",
    ).bind(identity.user_id()).bind(key_hash.as_slice()).fetch_optional(&mut *tx).await?;
    if let Some((id, previous)) = prior {
        if previous != fingerprint {
            return Err(IncomeError::Conflict);
        }
        tx.commit().await?;
        return Ok(input.result(id));
    }
    let zone = zone.as_deref().ok_or(IncomeError::UnconfirmedZone)?;
    let zone = parse_zone(zone).map_err(|_| IncomeError::UnsupportedZone)?;
    let today = today_at(captured, zone).map_err(|error| match error {
        CreationError::DateOutOfRange => IncomeError::DateOutOfRange,
        _ => IncomeError::Unavailable,
    })?;
    input
        .occurred_on
        .ensure_not_future(today)
        .map_err(|_| IncomeError::FutureDate)?;
    if let Some(category) = &input.category_id {
        let compatible: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE id=$1::uuid AND kind='income')",
        )
        .bind(category)
        .fetch_one(&mut *tx)
        .await?;
        if !compatible {
            return Err(IncomeError::InvalidCategory);
        }
    }
    let id: String = sqlx::query_scalar(
        "INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,category_id,description)
         VALUES($1::uuid,'income',$2::date,$3,$4,$5::uuid,$6) RETURNING id::text",
    ).bind(identity.user_id()).bind(input.occurred_on.to_string()).bind(input.amount.minor_units())
     .bind(input.amount.currency().to_string()).bind(&input.category_id).bind(&input.description)
     .fetch_one(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO transaction_audit(user_id,actor_id,transaction_id,action,changed_fields)
         VALUES($1::uuid,$1::uuid,$2::uuid,'create',ARRAY[]::text[])",
    )
    .bind(identity.user_id())
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO income_creation_keys(user_id,key_hash,request_hash,transaction_id)
         VALUES($1::uuid,$2,$3,$4::uuid)",
    )
    .bind(identity.user_id())
    .bind(key_hash.as_slice())
    .bind(fingerprint.as_slice())
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(input.result(id))
}
