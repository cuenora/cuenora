//! Optional informational metadata; actual transfer amounts always remain authoritative.
use cuenora_domain::{Money, MoneyError, QuotedRate};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TransferQuote {
    pub quoted_rate: String,
    rate_comparison: RateComparison,
}

#[derive(Serialize)]
struct RateComparison {
    indicative_received_amount: String,
    difference: String,
    mismatch: bool,
}

impl TransferQuote {
    pub(crate) fn new(rate: QuotedRate, sent: Money, received: Money) -> Result<Self, MoneyError> {
        let indicative = rate.indicative(sent, received.currency())?;
        let difference = received.checked_sub(indicative)?;
        Ok(Self {
            quoted_rate: rate.to_string(),
            rate_comparison: RateComparison {
                indicative_received_amount: indicative.to_string(),
                difference: difference.to_string(),
                mismatch: difference.minor_units() != 0,
            },
        })
    }
}
