//! Pure calendar slots, independent of rule state, storage and financial posting.

use jiff::{ToSpan, Unit, civil::Date};

use crate::BusinessDate;

/// Supported recurring-expense calendar intervals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecurrenceCadence {
    Weekly,
    Monthly,
    Quarterly,
    Annual,
}

/// Returns the first calendar slot strictly after `after`, or the anchor when
/// `after` is absent or precedes it. Always pass the original anchor, never an
/// adjusted previous date. Returns `None` when no slot remains in years 0001–9999.
///
/// This is calendar arithmetic only: callers apply inclusive rule ends, pause
/// exclusions and generation progress. It neither reads a clock nor resolves a
/// time zone; inputs are civil dates already interpreted in the rule's zone.
pub fn next_occurrence(
    anchor: BusinessDate,
    cadence: RecurrenceCadence,
    after: Option<BusinessDate>,
) -> Option<BusinessDate> {
    let Some(after) = after.filter(|after| *after >= anchor) else {
        return Some(anchor);
    };
    let anchor = anchor.calendar_date();
    let after = after.calendar_date();
    let candidate = match cadence {
        RecurrenceCadence::Weekly => {
            let elapsed = anchor.until((Unit::Day, after)).ok()?.get_days();
            let periods = i64::from(elapsed / 7 + 1);
            anchor.checked_add(periods.weeks()).ok()?
        }
        RecurrenceCadence::Monthly | RecurrenceCadence::Quarterly | RecurrenceCadence::Annual => {
            let step = match cadence {
                RecurrenceCadence::Quarterly => 3,
                RecurrenceCadence::Annual => 12,
                _ => 1,
            };
            let anchor_month = month_index(anchor);
            let periods = (month_index(after) - anchor_month) / step;
            let month = anchor_month + periods * step;
            let candidate = anchored_month(anchor, month)?;
            if candidate > after {
                candidate
            } else {
                anchored_month(anchor, month + step)?
            }
        }
    };
    BusinessDate::from_calendar_date(candidate)
}

fn month_index(date: Date) -> u32 {
    (date.year() as u32 - 1) * 12 + date.month() as u32 - 1
}

fn anchored_month(anchor: Date, month: u32) -> Option<Date> {
    let year = month / 12 + 1;
    if year > 9999 {
        return None;
    }
    // At most four attempts: only days 29–31 can be missing in a month.
    (1..=anchor.day())
        .rev()
        .find_map(|day| Date::new(year as i16, (month % 12 + 1) as i8, day).ok())
}
