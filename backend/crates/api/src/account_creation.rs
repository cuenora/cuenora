//! Account creation and special opening posting as one owned business operation.
use crate::{auth_identity::AuthenticatedIdentity, profile_zone::parse_zone};
use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Tz;
use cuenora_domain::{AccountName, AccountType, BusinessDate, Currency, Money};
use serde::Serialize;
use sqlx::PgPool;

pub(crate) struct OpeningBalance {
    pub amount: Money,
    pub occurred_on: BusinessDate,
}

pub(crate) struct CreateAccount {
    pub name: AccountName,
    pub account_type: AccountType,
    pub currency: Currency,
    pub opening: Option<OpeningBalance>,
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
    UnconfirmedZone,
    UnsupportedZone,
    DateOutOfRange,
    FutureDate,
}
impl From<sqlx::Error> for CreationError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

// No clock access here: one caller-captured instant defines user today. Do not
// replace the named IANA zone with a database/OS/device zone or numeric offset.
pub(crate) fn today_at(captured: DateTime<Utc>, zone: Tz) -> Result<BusinessDate, CreationError> {
    let date = captured.with_timezone(&zone).date_naive();
    if !(1..=9999).contains(&date.year()) {
        return Err(CreationError::DateOutOfRange);
    }
    BusinessDate::parse(&date.to_string()).map_err(|_| CreationError::DateOutOfRange)
}

type PriorCreation = (String, String, String, String, i64, Option<String>);

/// Bearer identity is mandatory. Lock the owner once for verification, profile
/// snapshot and retry serialization. A matching replay never reposts or rejects
/// an accepted historical date solely because the profile zone later changed.
pub(crate) async fn create(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    key_hash: &[u8; 32],
    input: &CreateAccount,
) -> Result<CreatedAccount, CreationError> {
    let captured: DateTime<Utc> = std::time::SystemTime::now().into();
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    let owner: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT email_verified_at IS NOT NULL,profile_time_zone FROM users WHERE id=$1::uuid FOR UPDATE",
    ).bind(identity.user_id()).fetch_optional(&mut *tx).await?;
    let (true, zone) = owner.ok_or(CreationError::Unverified)? else {
        return Err(CreationError::Unverified);
    };
    let currency = input.currency.to_string();
    let amount = input
        .opening
        .as_ref()
        .map_or(0, |opening| opening.amount.minor_units());
    let date = input
        .opening
        .as_ref()
        .map(|opening| opening.occurred_on.to_string());
    let prior: Option<PriorCreation> = sqlx::query_as(
        "SELECT account_id::text,name,account_type,currency,opening_amount_minor,
                to_char(opening_occurred_on,'YYYY-MM-DD') FROM account_creation_keys
         WHERE user_id=$1::uuid AND key_hash=$2",
    )
    .bind(identity.user_id())
    .bind(key_hash.as_slice())
    .fetch_optional(&mut *tx)
    .await?;
    let id = if let Some((id, name, kind, code, prior_amount, prior_date)) = prior {
        if name != input.name.as_str()
            || kind != input.kind()
            || code != currency
            || amount != prior_amount
            || date != prior_date
        {
            return Err(CreationError::Conflict);
        }
        id
    } else {
        if let Some(opening) = &input.opening {
            let zone = zone.as_deref().ok_or(CreationError::UnconfirmedZone)?;
            let zone = parse_zone(zone).map_err(|_| CreationError::UnsupportedZone)?;
            opening
                .occurred_on
                .ensure_not_future(today_at(captured, zone)?)
                .map_err(|_| CreationError::FutureDate)?;
        }
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
        if amount != 0 {
            let event: String = sqlx::query_scalar(
                "INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,account_id)
                 VALUES($1::uuid,'opening_balance',$2::date,$3,$4,$5::uuid) RETURNING id::text",
            ).bind(identity.user_id()).bind(&date).bind(amount).bind(&currency).bind(&id)
             .fetch_one(&mut *tx).await?;
            sqlx::query(
                "INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on)
                 VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5,$6::date)",
            ).bind(identity.user_id()).bind(&event).bind(&id).bind(amount).bind(&currency).bind(&date)
             .execute(&mut *tx).await?;
            sqlx::query(
                "INSERT INTO transaction_audit(user_id,actor_id,transaction_id,action,changed_fields)
                 VALUES($1::uuid,$1::uuid,$2::uuid,'create',ARRAY[]::text[])",
            ).bind(identity.user_id()).bind(&event).execute(&mut *tx).await?;
        }
        sqlx::query(
            "INSERT INTO account_creation_keys(user_id,key_hash,account_id,name,account_type,currency,opening_amount_minor,opening_occurred_on)
             VALUES($1::uuid,$2,$3::uuid,$4,$5,$6,$7,$8::date)",
        ).bind(identity.user_id()).bind(key_hash.as_slice()).bind(&id)
         .bind(input.name.as_str()).bind(input.kind()).bind(&currency).bind(amount).bind(&date)
         .execute(&mut *tx).await?;
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

#[cfg(test)]
mod tests {
    use super::{CreationError, today_at};
    use chrono::{TimeZone, Utc};

    #[test]
    fn captured_instant_uses_confirmed_zone_not_utc_day() {
        let captured = Utc.with_ymd_and_hms(2026, 10, 24, 22, 30, 0).unwrap();
        let stockholm = today_at(captured, chrono_tz::Europe::Stockholm)
            .ok()
            .unwrap();
        let new_york = today_at(captured, chrono_tz::America::New_York)
            .ok()
            .unwrap();
        assert_eq!(stockholm.to_string(), "2026-10-25");
        assert_eq!(new_york.to_string(), "2026-10-24");
        assert!(stockholm.ensure_not_future(new_york).is_err());
        assert!(new_york.ensure_not_future(stockholm).is_ok());
    }

    #[test]
    fn zone_conversion_cannot_escape_supported_years() {
        let first = Utc.with_ymd_and_hms(1, 1, 1, 0, 0, 0).unwrap();
        let last = Utc.with_ymd_and_hms(9999, 12, 31, 23, 59, 59).unwrap();
        assert!(matches!(
            today_at(first, "Etc/GMT+12".parse().unwrap()),
            Err(CreationError::DateOutOfRange)
        ));
        assert!(matches!(
            today_at(last, chrono_tz::Pacific::Kiritimati),
            Err(CreationError::DateOutOfRange)
        ));
    }
}
