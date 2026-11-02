//! Bounded, verified-owner forecasts over a consistent, read-only posted snapshot.
use crate::{
    account_creation::today_at, api_error::ApiError, auth_identity::AuthenticatedIdentity,
    profile_zone::parse_zone, savings_http::Terms,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use cuenora_domain::{
    BusinessDate, Capitalization, Currency, ForecastDay, Money, NominalAnnualRate, PayoutFrequency,
    forecast_interest,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Window {
    start_on: String,
    end_on: String,
}
impl Window {
    pub(crate) fn dates(&self) -> Result<(NaiveDate, NaiveDate), ApiError> {
        BusinessDate::parse(&self.start_on).map_err(|_| ApiError::InvalidBusinessDate)?;
        BusinessDate::parse(&self.end_on).map_err(|_| ApiError::InvalidBusinessDate)?;
        let start = NaiveDate::parse_from_str(&self.start_on, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidBusinessDate)?;
        let end = NaiveDate::parse_from_str(&self.end_on, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidBusinessDate)?;
        if !(0..366).contains(&(end - start).num_days()) {
            return Err(ApiError::InvalidForecastWindow);
        }
        Ok((start, end))
    }
}
#[derive(Serialize)]
pub(crate) struct Forecast {
    account_id: String,
    currency: String,
    start_on: String,
    end_on: String,
    balance_as_of: String,
    posted_balance: String,
    balance_assumption: &'static str,
    includes_prior_accrual: bool,
    partial_first_period: bool,
    terms: Terms,
    rate_segments: Vec<RateSegment>,
    periods: Vec<Period>,
}
#[derive(Serialize)]
struct Ratio {
    numerator: String,
    denominator: String,
}
#[derive(Serialize)]
struct Period {
    start_on: String,
    end_on: String,
    exact_accrual_minor_units: Ratio,
    projected_payout: Option<String>,
}
#[derive(Serialize)]
struct RateSegment {
    start_on: String,
    end_on: String,
    effective_on: String,
    nominal_annual_percent: String,
    days_in_year: u16,
}
fn unavailable(_: sqlx::Error) -> ApiError {
    ApiError::SavingsForecastUnavailable
}

// Forecast payouts are arbitrary precision and nonnegative. Format their minor
// units directly rather than narrowing them into recorded Money or floating point.
fn decimal_minor_units(digits: &str) -> String {
    let padded = format!("{digits:0>3}");
    let (whole, fraction) = padded.split_at(padded.len() - 2);
    format!("{whole}.{fraction}")
}

pub(crate) async fn read(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    account: &str,
    window: Window,
) -> Result<Forecast, ApiError> {
    let (start, end) = window.dates()?;
    let captured: DateTime<Utc> = std::time::SystemTime::now().into();
    let mut tx = pool.begin().await.map_err(unavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    let owner: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT email_verified_at IS NOT NULL,profile_time_zone FROM users WHERE id=$1::uuid",
    )
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    let Some((true, zone)) = owner else {
        return Err(ApiError::EmailNotVerified);
    };
    // Resolve ownership before exposing zone or configuration state.
    let currency: Option<String> = sqlx::query_scalar("SELECT currency FROM accounts WHERE id=$1::uuid AND user_id=$2::uuid AND account_type='savings'").bind(account).bind(identity.user_id()).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    let currency = currency.ok_or(ApiError::NotFound)?;
    let zone = parse_zone(zone.as_deref().ok_or(ApiError::ProfileZoneUnconfirmed)?)?;
    let today = today_at(captured, zone)
        .map_err(|_| ApiError::DateOutOfRange)?
        .to_string();
    if window.start_on < today {
        return Err(ApiError::InvalidForecastWindow);
    }
    let terms_row: Option<(String,String,bool,String,String,String,String)> = sqlx::query_as("SELECT effective_on::text,payout_frequency,capitalize,rate_basis,day_count,balance_timing,rounding FROM savings_terms WHERE account_id=$1::uuid AND user_id=$2::uuid AND effective_on<=$3::date ORDER BY effective_on DESC LIMIT 1").bind(account).bind(identity.user_id()).bind(&window.start_on).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    let (
        effective_on,
        payout_frequency,
        capitalize,
        rate_basis,
        day_count,
        balance_timing,
        rounding,
    ) = terms_row.ok_or(ApiError::ForecastConfigurationMissing)?;
    let changed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM savings_terms WHERE account_id=$1::uuid AND user_id=$2::uuid AND effective_on>$3::date AND effective_on<=$4::date AND (payout_frequency<>$5 OR capitalize<>$6))").bind(account).bind(identity.user_id()).bind(&window.start_on).bind(&window.end_on).bind(&payout_frequency).bind(capitalize).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if changed {
        return Err(ApiError::ForecastTermsChangeUnsupported);
    }
    let initial: Option<(String,String)> = sqlx::query_as("SELECT effective_on::text,nominal_annual_percent::text FROM savings_rate_periods WHERE account_id=$1::uuid AND user_id=$2::uuid AND effective_on<=$3::date ORDER BY effective_on DESC LIMIT 1").bind(account).bind(identity.user_id()).bind(&window.start_on).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    let mut rates = vec![initial.ok_or(ApiError::ForecastConfigurationMissing)?];
    let changes: Vec<(String,String)> = sqlx::query_as("SELECT effective_on::text,nominal_annual_percent::text FROM savings_rate_periods WHERE account_id=$1::uuid AND user_id=$2::uuid AND effective_on>$3::date AND effective_on<=$4::date ORDER BY effective_on LIMIT 366").bind(account).bind(identity.user_id()).bind(&window.start_on).bind(&window.end_on).fetch_all(&mut *tx).await.map_err(unavailable)?;
    rates.extend(changes);
    let balance: String = sqlx::query_scalar("SELECT coalesce(sum(signed_amount_minor),0)::text FROM account_movements WHERE account_id=$1::uuid AND user_id=$2::uuid AND currency=$3 AND occurred_on<=$4::date").bind(account).bind(identity.user_id()).bind(&currency).bind(&today).fetch_one(&mut *tx).await.map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    let currency_value: Currency = currency
        .parse()
        .map_err(|_| ApiError::SavingsForecastUnavailable)?;
    let balance = Money::from_minor_units(
        balance
            .parse()
            .map_err(|_| ApiError::SavingsForecastUnavailable)?,
        currency_value,
    )
    .map_err(|_| ApiError::SavingsForecastUnavailable)?;
    let frequency = match payout_frequency.as_str() {
        "daily" => PayoutFrequency::Daily,
        "monthly" => PayoutFrequency::Monthly,
        "quarterly" => PayoutFrequency::Quarterly,
        "annual" => PayoutFrequency::Annual,
        _ => return Err(ApiError::SavingsForecastUnavailable),
    };
    let partial_first_period = match frequency {
        PayoutFrequency::Daily => false,
        PayoutFrequency::Monthly => start.day() != 1,
        PayoutFrequency::Quarterly => start.day() != 1 || !matches!(start.month(), 1 | 4 | 7 | 10),
        PayoutFrequency::Annual => start.day() != 1 || start.month() != 1,
    };
    // Bound arbitrary-precision compounding before CPU work. This conservative
    // digit budget limits this endpoint, not the accepted stored rate grammar.
    let count = (end - start).num_days() as usize + 1;
    let max_periods = match frequency {
        PayoutFrequency::Daily => count,
        PayoutFrequency::Monthly => count / 28 + 2,
        PayoutFrequency::Quarterly => count / 89 + 2,
        PayoutFrequency::Annual => count / 365 + 2,
    };
    let rate_digits = rates
        .iter()
        .map(|(_, r)| {
            r.split('.')
                .next()
                .unwrap_or("0")
                .trim_start_matches('0')
                .len()
        })
        .max()
        .unwrap_or(0);
    if capitalize && 24 + (rate_digits + 1) * max_periods > 4096 {
        return Err(ApiError::ForecastTooComplex);
    }
    let parsed_rates: Vec<_> = rates
        .iter()
        .map(|(_, rate)| {
            NominalAnnualRate::parse(rate).map_err(|_| ApiError::SavingsForecastUnavailable)
        })
        .collect::<Result<_, _>>()?;
    let mut input = Vec::new();
    let mut segments: Vec<RateSegment> = Vec::new();
    let mut index = 0;
    for date in start
        .iter_days()
        .take(((end - start).num_days() + 1) as usize)
    {
        let text = date.to_string();
        while rates
            .get(index + 1)
            .is_some_and(|(effective, _)| effective <= &text)
        {
            index += 1;
        }
        let year_days = if date.leap_year() { 366 } else { 365 };
        if let Some(segment) = segments
            .last_mut()
            .filter(|s| s.effective_on == rates[index].0 && s.days_in_year == year_days)
        {
            segment.end_on = text.clone();
        } else {
            segments.push(RateSegment {
                start_on: text.clone(),
                end_on: text.clone(),
                effective_on: rates[index].0.clone(),
                nominal_annual_percent: rates[index].1.clone(),
                days_in_year: year_days,
            });
        }
        input.push(ForecastDay {
            date: BusinessDate::parse(&text).map_err(|_| ApiError::InvalidBusinessDate)?,
            closing_balance: balance,
            rate: parsed_rates[index].clone(),
        });
    }
    let result = forecast_interest(
        &input,
        frequency,
        if capitalize {
            Capitalization::Reinvest
        } else {
            Capitalization::Exclude
        },
    )
    .map_err(|_| ApiError::SavingsForecastUnavailable)?;
    let periods = result
        .periods
        .into_iter()
        .map(|p| Period {
            start_on: p.start.to_string(),
            end_on: p.end.to_string(),
            exact_accrual_minor_units: Ratio {
                numerator: p.exact_minor_units.numer().to_string(),
                denominator: p.exact_minor_units.denom().to_string(),
            },
            projected_payout: p
                .payout_minor_units
                .map(|n| decimal_minor_units(&n.to_string())),
        })
        .collect();
    Ok(Forecast {
        account_id: account.to_owned(),
        currency,
        start_on: window.start_on,
        end_on: window.end_on,
        balance_as_of: today,
        posted_balance: balance.to_string(),
        balance_assumption: "constant_posted_snapshot",
        includes_prior_accrual: false,
        partial_first_period,
        terms: Terms {
            effective_on,
            payout_frequency,
            capitalize,
            rate_basis,
            day_count,
            balance_timing,
            rounding,
        },
        rate_segments: segments,
        periods,
    })
}
