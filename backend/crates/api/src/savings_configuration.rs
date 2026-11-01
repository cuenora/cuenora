//! Verified-owner savings history use cases; no financial writes.
use crate::{
    api_error::ApiError,
    auth_identity::AuthenticatedIdentity,
    savings_http::{Configuration, History, Page, Rate, Terms, Version},
};
use sqlx::PgPool;
use sqlx::{Postgres, Transaction};
type Tx<'a> = Transaction<'a, Postgres>;
type TermsRow = (String, String, String, bool, String, String, String, String);
fn unavailable(_: sqlx::Error) -> ApiError {
    ApiError::SavingsUnavailable
}
async fn begin<'a>(
    pool: &'a PgPool,
    identity: &AuthenticatedIdentity,
    account: &str,
) -> Result<Tx<'a>, ApiError> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    // Owner first, like existing posting/archive operations. Also serializes date-slot retries.
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
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT account_type FROM accounts WHERE id=$1::uuid AND user_id=$2::uuid FOR
             UPDATE",
    )
    .bind(account)
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    if kind.as_deref() != Some("savings") {
        return Err(ApiError::NotFound);
    }
    Ok(tx)
}
async fn read(
    tx: &mut Tx<'_>,
    owner: &str,
    account: &str,
    rates: bool,
    cursor: Option<&str>,
    limit: i64,
    inclusive: bool,
) -> Result<Vec<Version>, ApiError> {
    if rates {
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id::text, effective_on::text, nominal_annual_percent::text FROM
             savings_rate_periods WHERE user_id=$1::uuid AND account_id=$2::uuid AND
             ($3::date IS NULL OR effective_on>$3::date OR ($5 AND effective_on=$3::date))
             ORDER BY effective_on LIMIT $4",
        )
        .bind(owner)
        .bind(account)
        .bind(cursor)
        .bind(limit)
        .bind(inclusive)
        .fetch_all(&mut **tx)
        .await
        .map_err(unavailable)?;
        Ok(rows
            .into_iter()
            .map(|(id, effective_on, nominal_annual_percent)| Version {
                id,
                configuration: Configuration::Rate(Rate {
                    effective_on,
                    nominal_annual_percent,
                }),
            })
            .collect())
    } else {
        let rows: Vec<TermsRow> = sqlx::query_as(
            "SELECT id::text, effective_on::text, payout_frequency, capitalize, rate_basis,
             day_count, balance_timing, rounding FROM savings_terms WHERE user_id=$1::uuid
             AND account_id=$2::uuid AND ($3::date IS NULL OR effective_on>$3::date OR ($5
             AND effective_on=$3::date)) ORDER BY effective_on LIMIT $4",
        )
        .bind(owner)
        .bind(account)
        .bind(cursor)
        .bind(limit)
        .bind(inclusive)
        .fetch_all(&mut **tx)
        .await
        .map_err(unavailable)?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    effective_on,
                    payout_frequency,
                    capitalize,
                    rate_basis,
                    day_count,
                    balance_timing,
                    rounding,
                )| Version {
                    id,
                    configuration: Configuration::Terms(Terms {
                        effective_on,
                        payout_frequency,
                        capitalize,
                        rate_basis,
                        day_count,
                        balance_timing,
                        rounding,
                    }),
                },
            )
            .collect())
    }
}
pub(crate) async fn history(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    account: &str,
    rates: bool,
    page: Page,
) -> Result<History, ApiError> {
    let mut tx = begin(pool, identity, account).await?;
    let limit = usize::from(page.limit.unwrap_or(50));
    let mut versions = read(
        &mut tx,
        identity.user_id(),
        account,
        rates,
        page.cursor.as_deref(),
        (limit + 1) as i64,
        false,
    )
    .await?;
    let next_cursor = if versions.len() > limit {
        versions.truncate(limit);
        versions.last().map(|v| v.configuration.date().to_owned())
    } else {
        None
    };
    tx.commit().await.map_err(unavailable)?;
    Ok(History {
        versions,
        next_cursor,
    })
}
pub(crate) async fn append(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    account: &str,
    input: Configuration,
) -> Result<Version, ApiError> {
    let mut tx = begin(pool, identity, account).await?;
    let rates = matches!(input, Configuration::Rate(_));
    // Natural account/date identity makes equal retries safe without a new key table.
    let existing: Option<String> = if rates {
        sqlx::query_scalar(
            "SELECT id::text FROM savings_rate_periods WHERE account_id=$1::uuid AND
             user_id=$2::uuid AND effective_on=$3::date",
        )
    } else {
        sqlx::query_scalar(
            "SELECT id::text FROM savings_terms WHERE account_id=$1::uuid AND
             user_id=$2::uuid AND effective_on=$3::date",
        )
    }
    .bind(account)
    .bind(identity.user_id())
    .bind(input.date())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    if let Some(id) = existing {
        // Compare in SQL for rate numeric equivalence (2 and 2.0 are the same rate).
        let equal: bool = match &input {
            Configuration::Rate(r) => sqlx::query_scalar(
                "SELECT nominal_annual_percent=$2::numeric FROM savings_rate_periods WHERE
             id=$1::uuid",
            )
            .bind(&id)
            .bind(&r.nominal_annual_percent)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?,
            Configuration::Terms(t) => sqlx::query_scalar(
                "SELECT payout_frequency=$2 AND capitalize=$3 AND rate_basis=$4 AND
             day_count=$5 AND balance_timing=$6 AND rounding=$7 FROM savings_terms WHERE
             id=$1::uuid",
            )
            .bind(&id)
            .bind(&t.payout_frequency)
            .bind(t.capitalize)
            .bind(&t.rate_basis)
            .bind(&t.day_count)
            .bind(&t.balance_timing)
            .bind(&t.rounding)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?,
        };
        if !equal {
            return Err(ApiError::SavingsConfigurationConflict);
        }
    } else {
        match &input {
            Configuration::Rate(r) => {
                sqlx::query(
                    "INSERT INTO savings_rate_periods(user_id, account_id, effective_on,
             nominal_annual_percent) VALUES($1::uuid, $2::uuid, $3::date, $4::numeric)",
                )
                .bind(identity.user_id())
                .bind(account)
                .bind(&r.effective_on)
                .bind(&r.nominal_annual_percent)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            }
            Configuration::Terms(t) => {
                sqlx::query(
                    "INSERT INTO savings_terms(user_id, account_id, effective_on, payout_frequency,
             capitalize, rate_basis, day_count, balance_timing, rounding) VALUES($1::uuid,
             $2::uuid, $3::date, $4, $5, $6, $7, $8, $9)",
                )
                .bind(identity.user_id())
                .bind(account)
                .bind(&t.effective_on)
                .bind(&t.payout_frequency)
                .bind(t.capitalize)
                .bind(&t.rate_basis)
                .bind(&t.day_count)
                .bind(&t.balance_timing)
                .bind(&t.rounding)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            }
        }
    }
    let version = read(
        &mut tx,
        identity.user_id(),
        account,
        rates,
        Some(input.date()),
        1,
        true,
    )
    .await?
    .into_iter()
    .next()
    .ok_or(ApiError::SavingsUnavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(version)
}
