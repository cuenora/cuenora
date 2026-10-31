use crate::{Currency, Money, MoneyError};
use std::fmt;

/// Exact positive destination units per source unit, with at most twelve decimals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotedRate {
    canonical: String,
    coefficient: i128,
    denominator: i128,
}

impl QuotedRate {
    pub fn parse(input: &str) -> Result<Self, MoneyError> {
        let (integer, fraction) = match input.split_once('.') {
            Some((_, "")) => return Err(MoneyError::InvalidDecimal),
            Some(parts) => parts,
            None => (input, ""),
        };
        if integer.is_empty()
            || !integer.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(MoneyError::InvalidDecimal);
        }
        if fraction.len() > 12 {
            return Err(MoneyError::ExcessiveScale);
        }
        let integer = integer.trim_start_matches('0');
        let integer = if integer.is_empty() { "0" } else { integer };
        let fraction = fraction.trim_end_matches('0');
        let coefficient = format!("{integer}{fraction}")
            .parse::<i128>()
            .map_err(|_| MoneyError::OutOfRange)?;
        if coefficient == 0 {
            return Err(MoneyError::InvalidDecimal);
        }
        let canonical = if fraction.is_empty() {
            integer.to_owned()
        } else {
            format!("{integer}.{fraction}")
        };
        Ok(Self {
            canonical,
            coefficient,
            denominator: 10_i128.pow(fraction.len() as u32),
        })
    }

    /// Quantize only once in destination minor units, never changing actual money.
    pub fn indicative(&self, sent: Money, destination: Currency) -> Result<Money, MoneyError> {
        let numerator = i128::from(sent.minor_units())
            .checked_mul(self.coefficient)
            .ok_or(MoneyError::OutOfRange)?;
        Money::from_minor_ratio(numerator, self.denominator, destination)
    }
}

impl fmt::Display for QuotedRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical)
    }
}
