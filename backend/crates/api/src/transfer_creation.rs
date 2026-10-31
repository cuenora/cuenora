//! One transfer, two actual-amount postings, audit and permanent retry equivalence.
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

pub(crate) struct CreateTransfer {
    pub amount: Money,
    pub received: Money,
    pub source: String,
    pub destination: String,
    pub occurred_on: BusinessDate,
    pub description: Option<String>,
}

impl CreateTransfer {
    fn fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"transfer-creation-v1");
        for field in [
            Some(self.amount.to_string()),
            Some(self.amount.currency().to_string()),
            Some(self.received.to_string()),
            Some(self.received.currency().to_string()),
            Some(self.source.clone()),
            Some(self.destination.clone()),
            Some(self.occurred_on.to_string()),
            self.description.clone(),
        ] {
            match field {
                Some(value) => {
                    hash.update([1]);
                    hash.update((value.len() as u64).to_be_bytes());
                    hash.update(value.as_bytes());
                }
                None => hash.update([0]),
            }
        }
        hash.finalize().into()
    }

    fn result(&self, id: String) -> CreatedTransfer {
        CreatedTransfer {
            id,
            kind: "transfer",
            state: "posted",
            amount: self.amount.to_string(),
            currency: self.amount.currency().to_string(),
            account_id: self.source.clone(),
            destination_account_id: self.destination.clone(),
            received_amount: self.received.to_string(),
            received_currency: self.received.currency().to_string(),
            occurred_on: self.occurred_on.to_string(),
            category_id: None,
            description: self.description.clone(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CreatedTransfer {
    id: String,
    kind: &'static str,
    state: &'static str,
    amount: String,
    currency: String,
    account_id: String,
    destination_account_id: String,
    received_amount: String,
    received_currency: String,
    occurred_on: String,
    category_id: Option<String>,
    description: Option<String>,
}

pub(crate) enum TransferError {
    Unverified,
    Conflict,
    NotFound,
    InvalidAccount,
    UnconfirmedZone,
    UnsupportedZone,
    DateOutOfRange,
    FutureDate,
    Unavailable,
}
impl From<sqlx::Error> for TransferError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

pub(crate) async fn create(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    key_hash: &[u8; 32],
    input: &CreateTransfer,
) -> Result<CreatedTransfer, TransferError> {
    let captured: DateTime<Utc> = std::time::SystemTime::now().into();
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    // Follow the same owner -> account ordering as archive, corrections and posting.
    let owner: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT email_verified_at IS NOT NULL,profile_time_zone FROM users WHERE id=$1::uuid FOR UPDATE",
    ).bind(identity.user_id()).fetch_optional(&mut *tx).await?;
    let (true, zone) = owner.ok_or(TransferError::Unverified)? else {
        return Err(TransferError::Unverified);
    };
    let fingerprint = input.fingerprint();
    let prior: Option<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT transaction_id::text,request_hash FROM transfer_creation_keys WHERE user_id=$1::uuid AND key_hash=$2",
    ).bind(identity.user_id()).bind(key_hash.as_slice()).fetch_optional(&mut *tx).await?;
    if let Some((id, previous)) = prior {
        if previous != fingerprint {
            return Err(TransferError::Conflict);
        }
        tx.commit().await?;
        // Reconstruct the original acknowledgement from the original normalized request,
        // not from the possibly edited/voided event or now-archived accounts.
        return Ok(input.result(id));
    }
    let zone = parse_zone(zone.as_deref().ok_or(TransferError::UnconfirmedZone)?)
        .map_err(|_| TransferError::UnsupportedZone)?;
    let today = today_at(captured, zone).map_err(|error| match error {
        CreationError::DateOutOfRange => TransferError::DateOutOfRange,
        _ => TransferError::Unavailable,
    })?;
    input
        .occurred_on
        .ensure_not_future(today)
        .map_err(|_| TransferError::FutureDate)?;
    let accounts: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT id::text,currency,archived_at IS NULL FROM accounts
         WHERE user_id=$1::uuid AND id IN ($2::uuid,$3::uuid) ORDER BY id FOR UPDATE",
    )
    .bind(identity.user_id())
    .bind(&input.source)
    .bind(&input.destination)
    .fetch_all(&mut *tx)
    .await?;
    if accounts.len() != 2 {
        return Err(TransferError::NotFound);
    }
    for (id, currency, active) in accounts {
        let expected = if id == input.source {
            input.amount.currency()
        } else {
            input.received.currency()
        };
        if !active || currency != expected.to_string() {
            return Err(TransferError::InvalidAccount);
        }
    }
    let id: String = sqlx::query_scalar(
        "INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,account_id,
         destination_account_id,received_amount_minor,received_currency,description)
         VALUES($1::uuid,'transfer',$2::date,$3,$4,$5::uuid,$6::uuid,$7,$8,$9) RETURNING id::text",
    )
    .bind(identity.user_id())
    .bind(input.occurred_on.to_string())
    .bind(input.amount.minor_units())
    .bind(input.amount.currency().to_string())
    .bind(&input.source)
    .bind(&input.destination)
    .bind(input.received.minor_units())
    .bind(input.received.currency().to_string())
    .bind(&input.description)
    .fetch_one(&mut *tx)
    .await?;
    for (account, signed_amount, currency) in [
        (
            &input.source,
            -input.amount.minor_units(),
            input.amount.currency(),
        ),
        (
            &input.destination,
            input.received.minor_units(),
            input.received.currency(),
        ),
    ] {
        sqlx::query(
            "INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on)
             VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5,$6::date)",
        ).bind(identity.user_id()).bind(&id).bind(account).bind(signed_amount)
         .bind(currency.to_string()).bind(input.occurred_on.to_string()).execute(&mut *tx).await?;
    }
    sqlx::query(
        "INSERT INTO transaction_audit(user_id,actor_id,transaction_id,action,changed_fields)
        VALUES($1::uuid,$1::uuid,$2::uuid,'create',ARRAY[]::text[])",
    )
    .bind(identity.user_id())
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO transfer_creation_keys(user_id,key_hash,request_hash,transaction_id)
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
