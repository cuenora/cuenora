use std::{fmt, str::FromStr};

/// A currency supported by Cuenora's recorded-money model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Currency {
    Sek,
    Eur,
    Usd,
}

impl FromStr for Currency {
    type Err = MoneyError;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        match code {
            "SEK" => Ok(Self::Sek),
            "EUR" => Ok(Self::Eur),
            "USD" => Ok(Self::Usd),
            _ => Err(MoneyError::UnsupportedCurrency),
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sek => "SEK",
            Self::Eur => "EUR",
            Self::Usd => "USD",
        })
    }
}

/// An invalid monetary input or operation; no input value is stored in the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyError {
    UnsupportedCurrency,
    InvalidDecimal,
    ExcessiveScale,
    InvalidRatio,
    CurrencyMismatch,
    OutOfRange,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedCurrency => "Unsupported currency",
            Self::InvalidDecimal => "Invalid decimal amount",
            Self::ExcessiveScale => "Recorded amounts support at most two decimal places",
            Self::InvalidRatio => "The denominator must be positive",
            Self::CurrencyMismatch => "Currencies must match",
            Self::OutOfRange => "Amount exceeds the supported range",
        })
    }
}

impl std::error::Error for MoneyError {}

/// Exact recorded money in signed hundredths with an explicit currency.
///
/// The range is symmetric: `-i64::MAX..=i64::MAX`. Currency is deliberately not
/// part of an ordering; values in different currencies cannot be ordered here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Money {
    minor_units: i64,
    currency: Currency,
}

impl Money {
    /// Construct an exact value, rejecting the asymmetric machine minimum.
    pub fn from_minor_units(minor_units: i64, currency: Currency) -> Result<Self, MoneyError> {
        if minor_units == i64::MIN {
            return Err(MoneyError::OutOfRange);
        }
        Ok(Self {
            minor_units,
            currency,
        })
    }

    /// Parse `-?[0-9]+(\.[0-9]{1,2})?` without rounding or locale conversion.
    ///
    /// Leading zeros are accepted. Even a trailing zero beyond two fractional
    /// digits is rejected. A negative sign on zero does not survive parsing.
    pub fn parse(amount: &str, currency: Currency) -> Result<Self, MoneyError> {
        let (negative, unsigned) = match amount.strip_prefix('-') {
            Some(unsigned) => (true, unsigned),
            None => (false, amount),
        };
        let (integer, fraction) = match unsigned.split_once('.') {
            Some((_, "")) => return Err(MoneyError::InvalidDecimal),
            Some(parts) => parts,
            None => (unsigned, ""),
        };
        if integer.is_empty()
            || !integer.bytes().all(|digit| digit.is_ascii_digit())
            || !fraction.bytes().all(|digit| digit.is_ascii_digit())
        {
            return Err(MoneyError::InvalidDecimal);
        }
        if fraction.len() > 2 {
            return Err(MoneyError::ExcessiveScale);
        }

        let integer = integer.parse::<i64>().map_err(|_| MoneyError::OutOfRange)?;
        let mut cents = fraction
            .bytes()
            .fold(0_i64, |value, digit| value * 10 + i64::from(digit - b'0'));
        if fraction.len() == 1 {
            cents *= 10;
        }
        let magnitude = integer
            .checked_mul(100)
            .and_then(|value| value.checked_add(cents))
            .ok_or(MoneyError::OutOfRange)?;
        Self::from_minor_units(if negative { -magnitude } else { magnitude }, currency)
    }

    /// Round an exact ratio of minor units to nearest, breaking ties to even.
    ///
    /// The denominator must be positive. The rounded result must fit the same
    /// symmetric range as recorded input; this is not an interest or FX engine.
    pub fn from_minor_ratio(
        numerator: i128,
        denominator: i128,
        currency: Currency,
    ) -> Result<Self, MoneyError> {
        if denominator <= 0 {
            return Err(MoneyError::InvalidRatio);
        }
        let denominator = denominator as u128;
        let magnitude = numerator.unsigned_abs();
        let quotient = magnitude / denominator;
        let remainder = magnitude % denominator;
        // Comparing complementary remainders avoids overflowing `2 * remainder`.
        let distance_to_next = denominator - remainder;
        let round_up = remainder > distance_to_next
            || (remainder == distance_to_next && !quotient.is_multiple_of(2));
        let rounded = quotient
            .checked_add(u128::from(round_up))
            .ok_or(MoneyError::OutOfRange)?;
        let units = i64::try_from(rounded).map_err(|_| MoneyError::OutOfRange)?;
        Self::from_minor_units(if numerator < 0 { -units } else { units }, currency)
    }

    pub fn minor_units(self) -> i64 {
        self.minor_units
    }

    pub fn currency(self) -> Currency {
        self.currency
    }

    /// Add only matching currencies; reject machine and domain overflow.
    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch);
        }
        let units = self
            .minor_units
            .checked_add(other.minor_units)
            .ok_or(MoneyError::OutOfRange)?;
        Self::from_minor_units(units, self.currency)
    }

    /// Subtract only matching currencies; negative results are valid.
    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch);
        }
        let units = self
            .minor_units
            .checked_sub(other.minor_units)
            .ok_or(MoneyError::OutOfRange)?;
        Self::from_minor_units(units, self.currency)
    }

    /// Negate exactly; every valid amount has a valid opposite.
    pub fn checked_neg(self) -> Result<Self, MoneyError> {
        let units = self
            .minor_units
            .checked_neg()
            .ok_or(MoneyError::OutOfRange)?;
        Self::from_minor_units(units, self.currency)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let magnitude = self.minor_units.unsigned_abs();
        let sign = if self.minor_units < 0 { "-" } else { "" };
        write!(f, "{sign}{}.{:02}", magnitude / 100, magnitude % 100)
    }
}
