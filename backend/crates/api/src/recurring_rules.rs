//! Serialized recurrence lifecycle; never generates occurrences or posts money.
use crate::{
    account_creation::today_at,
    api_error::ApiError,
    auth_identity::AuthenticatedIdentity,
    profile_zone::parse_zone,
    recurring_rule_http::{Edit, Terms},
};
use chrono::{DateTime, Utc};
use cuenora_domain::{BusinessDate, Money};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgRow};

type Tx<'a> = Transaction<'a, Postgres>;
fn unavailable(_: sqlx::Error) -> ApiError {
    ApiError::RecurrenceUnavailable
}

#[derive(Serialize)]
pub(crate) struct Output {
    id: String,
    #[serde(flatten)]
    terms: Terms,
    state: String,
    paused_on: Option<String>,
    generated_through: Option<String>,
}
impl Output {
    fn created(id: String, terms: Terms) -> Self {
        Self {
            id,
            terms,
            state: "active".into(),
            paused_on: None,
            generated_through: None,
        }
    }
    fn from_row(row: &PgRow) -> Result<Self, ApiError> {
        let currency: String = row.try_get("currency").map_err(unavailable)?;
        let money = Money::from_minor_units(
            row.try_get("amount_minor").map_err(unavailable)?,
            currency
                .parse()
                .map_err(|_| ApiError::RecurrenceUnavailable)?,
        )
        .map_err(|_| ApiError::RecurrenceUnavailable)?;
        Ok(Self {
            id: row.try_get("id").map_err(unavailable)?,
            terms: Terms {
                amount: money.to_string(),
                currency,
                account_id: row.try_get("account_id").map_err(unavailable)?,
                category_id: row.try_get("category_id").map_err(unavailable)?,
                description: row.try_get("description").map_err(unavailable)?,
                cadence: row.try_get("cadence").map_err(unavailable)?,
                starts_on: row.try_get("starts_on").map_err(unavailable)?,
                ends_on: row.try_get("ends_on").map_err(unavailable)?,
                time_zone: row.try_get("time_zone").map_err(unavailable)?,
            },
            state: row.try_get("state").map_err(unavailable)?,
            paused_on: row.try_get("paused_on").map_err(unavailable)?,
            generated_through: row.try_get("generated_through").map_err(unavailable)?,
        })
    }
}
async fn begin<'a>(pool: &'a PgPool, identity: &AuthenticatedIdentity) -> Result<Tx<'a>, ApiError> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    // Same owner-first serialization boundary as account archival and posting.
    // Future generation must acquire this owner lock before the rule lock too.
    let verified: Option<bool> = sqlx::query_scalar(
        "SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid FOR UPDATE",
    )
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    if verified != Some(true) {
        return Err(ApiError::EmailNotVerified);
    }
    Ok(tx)
}
async fn references(
    tx: &mut Tx<'_>,
    identity: &AuthenticatedIdentity,
    terms: &Terms,
    selecting_account: bool,
) -> Result<(), ApiError> {
    if let Some(category) = &terms.category_id {
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE id=$1::uuid AND kind='expense')",
        )
        .bind(category)
        .fetch_one(&mut **tx)
        .await
        .map_err(unavailable)?;
        if !valid {
            return Err(ApiError::InvalidRequest);
        }
    }
    if let Some(account) = &terms.account_id {
        let row:Option<(String,bool)>=sqlx::query_as("SELECT currency,archived_at IS NULL FROM accounts WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE").bind(account).bind(identity.user_id()).fetch_optional(&mut **tx).await.map_err(unavailable)?;
        let (currency, active) = row.ok_or(ApiError::NotFound)?;
        if currency != terms.currency {
            return Err(ApiError::InvalidRequest);
        }
        if selecting_account && !active {
            return Err(ApiError::RecurrenceAccountArchived);
        }
    }
    Ok(())
}
pub(crate) async fn create(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    key: &[u8; 32],
    terms: Terms,
) -> Result<Output, ApiError> {
    let mut tx = begin(pool, identity).await?;
    let fingerprint = terms.fingerprint();
    let previous:Option<(String,Vec<u8>)>=sqlx::query_as("SELECT rule_id::text,request_hash FROM recurring_rule_creation_keys WHERE user_id=$1::uuid AND key_hash=$2").bind(identity.user_id()).bind(key.as_slice()).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    if let Some((id, hash)) = previous {
        if hash != fingerprint {
            return Err(ApiError::IdempotencyConflict);
        }
        tx.commit().await.map_err(unavailable)?;
        return Ok(Output::created(id, terms));
    }
    references(&mut tx, identity, &terms, true).await?;
    let id:String=sqlx::query_scalar("INSERT INTO recurring_rules(user_id,amount_minor,currency,account_id,category_id,description,cadence,starts_on,ends_on,time_zone) VALUES($1::uuid,$2,$3,$4::uuid,$5::uuid,$6,$7,$8::date,$9::date,$10) RETURNING id::text")
        .bind(identity.user_id()).bind(terms.minor_units()).bind(&terms.currency).bind(&terms.account_id).bind(&terms.category_id).bind(&terms.description).bind(&terms.cadence).bind(&terms.starts_on).bind(&terms.ends_on).bind(&terms.time_zone).fetch_one(&mut *tx).await.map_err(unavailable)?;
    sqlx::query("INSERT INTO recurring_rule_creation_keys(user_id,key_hash,request_hash,rule_id) VALUES($1::uuid,$2,$3,$4::uuid)").bind(identity.user_id()).bind(key.as_slice()).bind(fingerprint.as_slice()).bind(&id).execute(&mut *tx).await.map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(Output::created(id, terms))
}
pub(crate) enum Change {
    Edit(Box<Edit>),
    Pause,
    Resume,
}
async fn load(
    tx: &mut Tx<'_>,
    identity: &AuthenticatedIdentity,
    id: &str,
) -> Result<Output, ApiError> {
    let row=sqlx::query("SELECT id::text,amount_minor,currency,account_id::text,category_id::text,description,cadence,to_char(starts_on,'YYYY-MM-DD') AS starts_on,to_char(ends_on,'YYYY-MM-DD') AS ends_on,time_zone,state,to_char(paused_on,'YYYY-MM-DD') AS paused_on,to_char(generated_through,'YYYY-MM-DD') AS generated_through FROM recurring_rules WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE")
        .bind(id).bind(identity.user_id()).fetch_optional(&mut **tx).await.map_err(unavailable)?.ok_or(ApiError::NotFound)?;
    Output::from_row(&row)
}
fn local_day(instant: DateTime<Utc>, zone: &str) -> Result<BusinessDate, ApiError> {
    today_at(instant, parse_zone(zone)?).map_err(|_| ApiError::DateOutOfRange)
}
pub(crate) async fn change(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    id: &str,
    change: Change,
) -> Result<Output, ApiError> {
    let mut tx = begin(pool, identity).await?;
    let mut current = load(&mut tx, identity, id).await?;
    // Capture after lock acquisition so a midnight lock wait cannot use yesterday.
    let instant: DateTime<Utc> = std::time::SystemTime::now().into();
    match change {
        Change::Edit(edit) => {
            let terms = edit.merge(current.terms.clone())?;
            if terms != current.terms {
                references(
                    &mut tx,
                    identity,
                    &terms,
                    terms.account_id != current.terms.account_id,
                )
                .await?;
                let calendar_changed = terms.starts_on != current.terms.starts_on
                    || terms.cadence != current.terms.cadence
                    || terms.time_zone != current.terms.time_zone;
                let boundary = if calendar_changed {
                    Some(
                        local_day(instant, &terms.time_zone)?
                            .max(local_day(instant, &current.terms.time_zone)?)
                            .to_string(),
                    )
                } else {
                    None
                };
                sqlx::query("UPDATE recurring_rules SET amount_minor=$1,currency=$2,account_id=$3::uuid,category_id=$4::uuid,description=$5,cadence=$6,starts_on=$7::date,ends_on=$8::date,time_zone=$9,generated_through=CASE WHEN $10::date IS NULL THEN generated_through ELSE GREATEST(generated_through,$10::date) END,updated_at=statement_timestamp() WHERE id=$11::uuid AND user_id=$12::uuid")
                    .bind(terms.minor_units()).bind(&terms.currency).bind(&terms.account_id).bind(&terms.category_id).bind(&terms.description).bind(&terms.cadence).bind(&terms.starts_on).bind(&terms.ends_on).bind(&terms.time_zone).bind(boundary).bind(id).bind(identity.user_id()).execute(&mut *tx).await.map_err(unavailable)?;
                current = load(&mut tx, identity, id).await?;
            }
        }
        Change::Pause if current.state == "active" => {
            let today = local_day(instant, &current.terms.time_zone)?.to_string();
            sqlx::query("UPDATE recurring_rules SET state='paused',paused_on=$1::date,updated_at=statement_timestamp() WHERE id=$2::uuid AND user_id=$3::uuid").bind(today).bind(id).bind(identity.user_id()).execute(&mut *tx).await.map_err(unavailable)?;
            current = load(&mut tx, identity, id).await?;
        }
        Change::Resume if current.state == "paused" => {
            let today = local_day(instant, &current.terms.time_zone)?.to_string();
            if current
                .paused_on
                .as_ref()
                .is_none_or(|paused| paused > &today)
            {
                return Err(ApiError::RecurrenceStateConflict);
            }
            sqlx::query("UPDATE recurring_rules SET excluded_dates=excluded_dates + datemultirange(daterange(paused_on,$1::date,'[)')),state='active',paused_on=NULL,updated_at=statement_timestamp() WHERE id=$2::uuid AND user_id=$3::uuid").bind(today).bind(id).bind(identity.user_id()).execute(&mut *tx).await.map_err(unavailable)?;
            current = load(&mut tx, identity, id).await?;
        }
        Change::Pause | Change::Resume => {}
    }
    tx.commit().await.map_err(unavailable)?;
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::local_day;
    use chrono::{DateTime, Utc};

    #[test]
    fn transition_days_use_rule_zone_across_date_lines_and_dst() {
        let instant: DateTime<Utc> = "2024-03-31T00:30:00Z".parse().unwrap();
        assert_eq!(
            local_day(instant, "Europe/Stockholm").unwrap().to_string(),
            "2024-03-31"
        );
        assert_eq!(
            local_day(instant, "America/Los_Angeles")
                .unwrap()
                .to_string(),
            "2024-03-30"
        );
        assert_eq!(
            local_day(instant, "Pacific/Kiritimati")
                .unwrap()
                .to_string(),
            "2024-03-31"
        );
        let instant: DateTime<Utc> = "9999-12-31T23:30:00Z".parse().unwrap();
        assert!(local_day(instant, "Pacific/Kiritimati").is_err());
    }
}
