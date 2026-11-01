//! Pure payout forecasts; no clock, persistence, rate selection or actual receipts.

use std::fmt;

use chrono::Datelike;
use num_bigint::BigInt;
use num_rational::BigRational;

use crate::{BusinessDate, Currency, Money, NominalAnnualRate, daily_interest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayoutFrequency {
    Daily,
    Monthly,
    Quarterly,
    Annual,
}

/// Required explicitly: there is no implicit reinvestment policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capitalization {
    Exclude,
    Reinvest,
}

/// Caller-selected posted closing balance and effective nominal rate for one day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastDay {
    pub date: BusinessDate,
    pub closing_balance: Money,
    pub rate: NominalAnnualRate,
}

/// Exact accrual for an inclusive interval, in currency minor units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastPeriod {
    pub start: BusinessDate,
    pub end: BusinessDate,
    pub exact_minor_units: BigRational,
    /// Half-even rounded estimate at natural period close; None for an open tail.
    /// Arbitrary precision, not recorded Money and never an actual payment.
    pub payout_minor_units: Option<BigInt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavingsForecast {
    pub currency: Currency,
    pub periods: Vec<ForecastPeriod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavingsForecastError {
    EmptyInput,
    NonConsecutiveDates,
    CurrencyMismatch,
}

impl fmt::Display for SavingsForecastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyInput => "Forecast requires at least one day",
            Self::NonConsecutiveDates => "Forecast dates must be consecutive and increasing",
            Self::CurrencyMismatch => "Forecast currencies must match",
        })
    }
}

impl std::error::Error for SavingsForecastError {}

/// Forecast one contiguous window with explicitly fixed frequency/capitalization.
///
/// Callers supply every day from the terms start (or a natural period start),
/// including days without movements, and select effective rates inclusively.
/// The first period may be partial. A horizon before natural close retains exact
/// accrual with no payout. Each invocation starts with zero projected capital;
/// it is not a continuation API and must not be split at rate changes.
///
/// Rounded payouts enter only the following day's projected base when Reinvest
/// is selected. Supplied balances must not also include those projected receipts.
/// Existing/actual receipts belong to posted inputs, not to projected payouts;
/// callers must choose a window without overlapping actual/projected receipts.
/// Effective-dated frequency/mode changes require separate orchestration, not
/// silent resets inside this fixed-terms primitive.
pub fn forecast_interest(
    days: &[ForecastDay],
    frequency: PayoutFrequency,
    capitalization: Capitalization,
) -> Result<SavingsForecast, SavingsForecastError> {
    let first = days.first().ok_or(SavingsForecastError::EmptyInput)?;
    let currency = first.closing_balance.currency();
    // Validate the whole window before calculating any result.
    for (i, day) in days.iter().enumerate() {
        if day.closing_balance.currency() != currency {
            return Err(SavingsForecastError::CurrencyMismatch);
        }
        if i > 0 && days[i - 1].date.calendar_date().succ_opt() != Some(day.date.calendar_date()) {
            return Err(SavingsForecastError::NonConsecutiveDates);
        }
    }

    let mut periods = Vec::new();
    let mut start = first.date;
    let mut exact = BigRational::from_integer(0.into());
    let mut capital = BigInt::from(0);
    for (i, day) in days.iter().enumerate() {
        let base = (BigInt::from(day.closing_balance.minor_units()) + &capital).max(0.into());
        // Reuse the daily primitive's rate/day-count policy, scaling its exact
        // one-minor-unit result to avoid narrowing arbitrary projected capital.
        let unit = daily_interest(
            Money::from_minor_units(1, currency).expect("one minor unit is valid"),
            &day.rate,
            day.date,
        );
        exact += BigRational::new(unit.numerator().clone(), unit.denominator().clone()) * base;
        if closes_period(day.date, frequency) {
            let payout = round_half_even(&exact);
            if capitalization == Capitalization::Reinvest {
                capital += &payout;
            }
            periods.push(ForecastPeriod {
                start,
                end: day.date,
                exact_minor_units: exact,
                payout_minor_units: Some(payout),
            });
            exact = BigRational::from_integer(0.into());
            if let Some(next) = days.get(i + 1) {
                start = next.date;
            }
        }
    }
    let last = days.last().expect("nonempty window validated");
    if !closes_period(last.date, frequency) {
        periods.push(ForecastPeriod {
            start,
            end: last.date,
            exact_minor_units: exact,
            payout_minor_units: None,
        });
    }
    Ok(SavingsForecast { currency, periods })
}

fn closes_period(date: BusinessDate, frequency: PayoutFrequency) -> bool {
    let date = date.calendar_date();
    let month_end = date
        .succ_opt()
        .is_none_or(|next| next.month() != date.month());
    match frequency {
        PayoutFrequency::Daily => true,
        PayoutFrequency::Monthly => month_end,
        PayoutFrequency::Quarterly => month_end && date.month().is_multiple_of(3),
        PayoutFrequency::Annual => month_end && date.month() == 12,
    }
}

fn round_half_even(value: &BigRational) -> BigInt {
    // Accrual is nonnegative; no signed remainder ambiguity occurs here.
    let quotient = value.numer() / value.denom();
    let twice_remainder = (value.numer() % value.denom()) * 2;
    if twice_remainder > *value.denom()
        || (twice_remainder == *value.denom() && &quotient % 2 != BigInt::from(0))
    {
        quotient + 1
    } else {
        quotient
    }
}
