use crate::{BusinessDate, Currency, Money};
use num_bigint::BigInt;
use num_rational::BigRational;
use std::fmt;

/// Explicit nominal annual percentage, not an effective yield or daily rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NominalAnnualRate(BigRational);

/// Invalid nominal percentage; never retains the supplied input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NominalAnnualRateError;

impl fmt::Display for NominalAnnualRateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Expected a nonnegative nominal annual percentage with at most six fractional digits and 1024 characters")
    }
}

impl std::error::Error for NominalAnnualRateError {}

impl NominalAnnualRate {
    /// Parse the same bounded decimal grammar as savings configuration, exactly.
    /// Zero and leading zeros are allowed; signs, whitespace and exponents are not.
    pub fn parse(input: &str) -> Result<Self, NominalAnnualRateError> {
        if input.is_empty() || input.len() > 1024 {
            return Err(NominalAnnualRateError);
        }
        let mut parts = input.split('.');
        let integer = parts.next().ok_or(NominalAnnualRateError)?;
        let fraction = parts.next();
        if integer.is_empty()
            || !integer.bytes().all(|byte| byte.is_ascii_digit())
            || fraction.is_some_and(|part| {
                part.is_empty() || part.len() > 6 || !part.bytes().all(|byte| byte.is_ascii_digit())
            })
            || parts.next().is_some()
        {
            return Err(NominalAnnualRateError);
        }
        let fraction = fraction.unwrap_or_default();
        let digits = format!("{integer}{fraction}");
        let coefficient =
            BigInt::parse_bytes(digits.as_bytes(), 10).ok_or(NominalAnnualRateError)?;
        // The validated scale is at most six, so conversion cannot truncate.
        let denominator = BigInt::from(10u32).pow(fraction.len() as u32);
        Ok(Self(BigRational::new(coefficient, denominator)))
    }
}

/// An unrounded estimate in currency minor units, never a posted monetary value.
/// The canonical numerator is nonnegative and denominator strictly positive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyInterest {
    minor_units: BigRational,
    currency: Currency,
}

impl DailyInterest {
    pub fn numerator(&self) -> &BigInt {
        self.minor_units.numer()
    }

    pub fn denominator(&self) -> &BigInt {
        self.minor_units.denom()
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }
}

/// Estimate one civil day's interest from its caller-supplied closing base.
///
/// Uses max(base, 0) * nominal annual percentage / 100 / calendar-year days.
/// No daily rounding, clock, database, payout scheduling or capitalization occurs.
/// The caller selects the applicable effective-dated rate and closing base.
pub fn daily_interest(
    closing_base: Money,
    rate: &NominalAnnualRate,
    date: BusinessDate,
) -> DailyInterest {
    let days = if date.calendar_date().leap_year() {
        366
    } else {
        365
    };
    let base = BigInt::from(closing_base.minor_units().max(0));
    DailyInterest {
        minor_units: &rate.0 * BigRational::new(base, BigInt::from(100 * days)),
        currency: closing_base.currency(),
    }
}
