use std::fmt;

use chrono::{Datelike, NaiveDate};

/// A canonical Gregorian business date in the supported year range 0001–9999.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BusinessDate(NaiveDate);

/// Errors returned while parsing or validating a business date.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BusinessDateError {
    /// The input is not a valid canonical Gregorian date.
    InvalidDate,
    /// The year is outside the supported range 0001–9999.
    OutOfRange,
    /// The date is later than the supplied current business date.
    FutureDate,
}

impl BusinessDate {
    /// Parses an exact ASCII `YYYY-MM-DD` date without normalizing the input.
    pub fn parse(input: &str) -> Result<Self, BusinessDateError> {
        let bytes = input.as_bytes();

        if bytes.len() == 11
            && bytes[5] == b'-'
            && bytes[8] == b'-'
            && bytes[..5].iter().all(u8::is_ascii_digit)
            && bytes[6..8].iter().all(u8::is_ascii_digit)
            && bytes[9..11].iter().all(u8::is_ascii_digit)
        {
            let year = parse_digits(&bytes[..5]);
            if !(1..=9999).contains(&year) {
                return Err(BusinessDateError::OutOfRange);
            }
            return Err(BusinessDateError::InvalidDate);
        }

        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes[..4].iter().all(u8::is_ascii_digit)
            || !bytes[5..7].iter().all(u8::is_ascii_digit)
            || !bytes[8..10].iter().all(u8::is_ascii_digit)
        {
            return Err(BusinessDateError::InvalidDate);
        }

        let year = parse_digits(&bytes[..4]);
        if !(1..=9999).contains(&year) {
            return Err(BusinessDateError::OutOfRange);
        }
        let month = parse_digits(&bytes[5..7]) as u32;
        let day = parse_digits(&bytes[8..10]) as u32;
        NaiveDate::from_ymd_opt(year, month, day)
            .map(Self)
            .ok_or(BusinessDateError::InvalidDate)
    }

    /// Returns this date when it is no later than `today`.
    pub fn ensure_not_future(self, today: Self) -> Result<Self, BusinessDateError> {
        if self > today {
            Err(BusinessDateError::FutureDate)
        } else {
            Ok(self)
        }
    }
}

impl fmt::Display for BusinessDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.0.year(),
            self.0.month(),
            self.0.day()
        )
    }
}

impl fmt::Display for BusinessDateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidDate => "invalid business date",
            Self::OutOfRange => "business date year is out of range",
            Self::FutureDate => "business date cannot be in the future",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for BusinessDateError {}

fn parse_digits(bytes: &[u8]) -> i32 {
    bytes
        .iter()
        .fold(0, |value, digit| value * 10 + i32::from(digit - b'0'))
}
