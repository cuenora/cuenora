use cuenora_domain::{BusinessDate, RecurrenceCadence, next_occurrence};

fn date(value: &str) -> BusinessDate {
    BusinessDate::parse(value).unwrap()
}

#[test]
fn starts_at_the_anchor_and_advances_strictly_after_the_supplied_date() {
    let anchor = date("2025-01-31");
    assert_eq!(
        next_occurrence(anchor, RecurrenceCadence::Monthly, None),
        Some(anchor)
    );
    assert_eq!(
        next_occurrence(anchor, RecurrenceCadence::Monthly, Some(date("2025-01-01"))),
        Some(anchor)
    );
    for after in ["2025-01-31", "2025-02-12", "2025-02-27"] {
        assert_eq!(
            next_occurrence(anchor, RecurrenceCadence::Monthly, Some(date(after))),
            Some(date("2025-02-28"))
        );
    }
    assert_eq!(
        next_occurrence(anchor, RecurrenceCadence::Monthly, Some(date("2025-02-28"))),
        Some(date("2025-03-31"))
    );
}

#[test]
fn advances_each_cadence_from_the_original_anchor() {
    for (cadence, anchor, after, expected) in [
        (
            RecurrenceCadence::Weekly,
            "2025-01-31",
            "2025-02-01",
            "2025-02-07",
        ),
        (
            RecurrenceCadence::Monthly,
            "2025-01-31",
            "2025-02-28",
            "2025-03-31",
        ),
        (
            RecurrenceCadence::Quarterly,
            "2025-01-31",
            "2025-04-30",
            "2025-07-31",
        ),
        (
            RecurrenceCadence::Annual,
            "2024-02-29",
            "2027-02-28",
            "2028-02-29",
        ),
    ] {
        assert_eq!(
            next_occurrence(date(anchor), cadence, Some(date(after))),
            Some(date(expected))
        );
    }
}

#[test]
fn returns_none_when_no_supported_date_remains() {
    for cadence in [
        RecurrenceCadence::Weekly,
        RecurrenceCadence::Monthly,
        RecurrenceCadence::Quarterly,
        RecurrenceCadence::Annual,
    ] {
        assert_eq!(
            next_occurrence(date("0001-01-01"), cadence, Some(date("9999-12-31"))),
            None
        );
        assert_eq!(
            next_occurrence(date("9999-12-31"), cadence, None),
            Some(date("9999-12-31"))
        );
    }
}

#[test]
fn preserves_ordinary_days_and_searches_without_replaying_old_slots() {
    for (cadence, anchor, after, expected) in [
        (
            RecurrenceCadence::Weekly,
            "2025-01-01",
            "2025-01-08",
            "2025-01-15",
        ),
        (
            RecurrenceCadence::Monthly,
            "0001-01-15",
            "9999-11-15",
            "9999-12-15",
        ),
        (
            RecurrenceCadence::Monthly,
            "2025-01-15",
            "2025-02-14",
            "2025-02-15",
        ),
        (
            RecurrenceCadence::Quarterly,
            "2025-01-15",
            "2025-03-31",
            "2025-04-15",
        ),
        (
            RecurrenceCadence::Annual,
            "2025-04-15",
            "2026-01-01",
            "2026-04-15",
        ),
        (
            RecurrenceCadence::Weekly,
            "9999-12-24",
            "9999-12-24",
            "9999-12-31",
        ),
    ] {
        assert_eq!(
            next_occurrence(date(anchor), cadence, Some(date(after))),
            Some(date(expected))
        );
    }
}

fn assert_sequence(case: &str, cadence: RecurrenceCadence, start: &str, expected: &[&str]) {
    let anchor = date(start);
    let mut after = anchor;
    for expected in expected {
        let next = date(expected);
        assert_eq!(
            next_occurrence(anchor, cadence, Some(after)),
            Some(next),
            "{case}: anchor {anchor}, after {after}"
        );
        after = next;
    }
}

#[test]
fn monthly_calendar_examples_rc01_through_rc06_preserve_nominal_days() {
    for (case, anchor, expected) in [
        ("RC01", "2025-01-29", &["2025-02-28", "2025-03-29"][..]),
        ("RC02", "2024-01-29", &["2024-02-29", "2024-03-29"]),
        (
            "RC03",
            "2025-01-30",
            &["2025-02-28", "2025-03-30", "2025-04-30", "2025-05-30"],
        ),
        ("RC04", "2024-01-30", &["2024-02-29", "2024-03-30"]),
        (
            "RC05",
            "2025-01-31",
            &["2025-02-28", "2025-03-31", "2025-04-30", "2025-05-31"],
        ),
        ("RC06", "2024-01-31", &["2024-02-29", "2024-03-31"]),
    ] {
        assert_sequence(case, RecurrenceCadence::Monthly, anchor, expected);
    }
}

#[test]
fn quarterly_calendar_examples_rc07_through_rc09_preserve_the_original_phase() {
    for (case, anchor, expected) in [
        (
            "RC07",
            "2023-11-30",
            &["2024-02-29", "2024-05-30", "2024-08-30", "2024-11-30"][..],
        ),
        ("RC08", "2024-11-30", &["2025-02-28", "2025-05-30"]),
        (
            "RC09",
            "2025-01-31",
            &["2025-04-30", "2025-07-31", "2025-10-31", "2026-01-31"],
        ),
    ] {
        assert_sequence(case, RecurrenceCadence::Quarterly, anchor, expected);
    }
}

#[test]
fn annual_calendar_examples_rc10_through_rc13_obey_gregorian_century_rules() {
    for (case, anchor, expected) in [
        (
            "RC10",
            "2024-02-29",
            &["2025-02-28", "2026-02-28", "2027-02-28", "2028-02-29"][..],
        ),
        ("RC11", "2025-04-30", &["2026-04-30", "2027-04-30"]),
        (
            "RC12",
            "2096-02-29",
            &["2097-02-28", "2098-02-28", "2099-02-28", "2100-02-28"],
        ),
        (
            "RC13",
            "1996-02-29",
            &["1997-02-28", "1998-02-28", "1999-02-28", "2000-02-29"],
        ),
    ] {
        assert_sequence(case, RecurrenceCadence::Annual, anchor, expected);
    }
}

#[test]
fn weekly_sequences_cross_february_and_year_ends_in_seven_calendar_days() {
    for (anchor, expected) in [
        ("2025-12-26", &["2026-01-02", "2026-01-09"][..]),
        ("2024-02-22", &["2024-02-29", "2024-03-07"]),
        ("2025-02-22", &["2025-03-01", "2025-03-08"]),
        ("1900-02-22", &["1900-03-01", "1900-03-08"]),
        ("2000-02-22", &["2000-02-29", "2000-03-07"]),
        ("2100-02-22", &["2100-03-01", "2100-03-08"]),
    ] {
        assert_sequence("weekly", RecurrenceCadence::Weekly, anchor, expected);
    }
}

#[test]
fn month_based_cadences_cross_december_without_resetting_the_anchor() {
    for (cadence, anchor, expected) in [
        (
            RecurrenceCadence::Monthly,
            "2025-11-30",
            &["2025-12-30", "2026-01-30", "2026-02-28"][..],
        ),
        (
            RecurrenceCadence::Quarterly,
            "2025-10-31",
            &["2026-01-31", "2026-04-30", "2026-07-31"],
        ),
        (
            RecurrenceCadence::Annual,
            "2025-12-31",
            &["2026-12-31", "2027-12-31"],
        ),
    ] {
        assert_sequence("year transition", cadence, anchor, expected);
    }
}

#[test]
fn each_cadence_includes_the_start_and_excludes_the_search_date() {
    let anchor = date("2025-01-15");
    for (cadence, first, second) in [
        (RecurrenceCadence::Weekly, "2025-01-22", "2025-01-29"),
        (RecurrenceCadence::Monthly, "2025-02-15", "2025-03-15"),
        (RecurrenceCadence::Quarterly, "2025-04-15", "2025-07-15"),
        (RecurrenceCadence::Annual, "2026-01-15", "2027-01-15"),
    ] {
        assert_eq!(next_occurrence(anchor, cadence, None), Some(anchor));
        assert_eq!(
            next_occurrence(anchor, cadence, Some(date("2025-01-14"))),
            Some(anchor)
        );
        for after in [anchor, date("2025-01-16")] {
            assert_eq!(
                next_occurrence(anchor, cadence, Some(after)),
                Some(date(first))
            );
        }
        assert_eq!(
            next_occurrence(anchor, cadence, Some(date(first))),
            Some(date(second))
        );
    }
}

#[test]
fn all_cadences_respect_both_supported_year_boundaries() {
    for (cadence, first, last_anchor) in [
        (RecurrenceCadence::Weekly, "0001-01-08", "9999-12-24"),
        (RecurrenceCadence::Monthly, "0001-02-01", "9999-11-30"),
        (RecurrenceCadence::Quarterly, "0001-04-01", "9999-09-30"),
        (RecurrenceCadence::Annual, "0002-01-01", "9998-12-30"),
    ] {
        let start = date("0001-01-01");
        assert_eq!(next_occurrence(start, cadence, None), Some(start));
        assert_eq!(
            next_occurrence(start, cadence, Some(start)),
            Some(date(first))
        );
        let anchor = date(last_anchor);
        let last = next_occurrence(anchor, cadence, Some(anchor)).unwrap();
        let expected = if cadence == RecurrenceCadence::Weekly {
            "9999-12-31"
        } else {
            "9999-12-30"
        };
        assert_eq!(last, date(expected));
        assert_eq!(next_occurrence(anchor, cadence, Some(last)), None);
    }
}
