use cuenora_domain::{BusinessDate, Currency, Money, NominalAnnualRate, daily_interest};

fn accrual(balance: i64, rate: &str, date: &str) -> (String, String, Currency) {
    let result = daily_interest(
        Money::from_minor_units(balance, Currency::Sek).unwrap(),
        &NominalAnnualRate::parse(rate).unwrap(),
        BusinessDate::parse(date).unwrap(),
    );
    (
        result.numerator().to_string(),
        result.denominator().to_string(),
        result.currency(),
    )
}

#[test]
fn daily_policy_examples_are_exact_minor_unit_ratios() {
    assert_eq!(
        accrual(130_000, "3.65", "2025-03-15"),
        ("13".into(), "1".into(), Currency::Sek)
    );
    assert_eq!(
        accrual(100_000, "3.66", "2024-02-29"),
        ("10".into(), "1".into(), Currency::Sek)
    );
    assert_eq!(
        accrual(100, "1", "2025-01-01"),
        ("1".into(), "365".into(), Currency::Sek)
    );
}

#[test]
fn year_day_count_uses_gregorian_calendar() {
    for (year, denominator) in [
        ("1900", "365"),
        ("2000", "366"),
        ("2100", "365"),
        ("0001", "365"),
        ("9999", "365"),
    ] {
        assert_eq!(accrual(100, "1", &format!("{year}-01-01")).1, denominator);
    }
}

#[test]
fn nonpositive_bases_and_zero_rates_accrue_zero() {
    for (base, rate) in [(-20000, "3.65"), (0, "3.65"), (100000, "0.000000")] {
        assert_eq!(
            accrual(base, rate, "2025-01-01"),
            ("0".into(), "1".into(), Currency::Sek)
        );
    }
}

#[test]
fn rate_parser_matches_configuration_decimal_bounds() {
    for input in ["0", "0001.250000", "3.123456", &"9".repeat(1024)] {
        assert!(NominalAnnualRate::parse(input).is_ok());
    }
    for input in [
        "",
        "-1",
        "+1",
        " 1",
        "1 ",
        "1e2",
        ".1",
        "1.",
        "1.0000000",
        "1.2.3",
        "NaN",
        "１",
        &"9".repeat(1025),
    ] {
        assert!(NominalAnnualRate::parse(input).is_err(), "{input}");
    }
}

#[test]
fn large_rates_do_not_overflow_or_round_and_preserve_currency() {
    let rate = NominalAnnualRate::parse(&"9".repeat(1024)).unwrap();
    for currency in [Currency::Sek, Currency::Eur, Currency::Usd] {
        let balance = Money::from_minor_units(i64::MAX, currency).unwrap();
        let result = daily_interest(balance, &rate, BusinessDate::parse("2025-01-01").unwrap());
        // Verify the defining equation without finite-width intermediate values.
        let coefficient = num_bigint::BigInt::parse_bytes("9".repeat(1024).as_bytes(), 10).unwrap();
        assert_eq!(
            result.numerator() * 36500,
            coefficient * i64::MAX * result.denominator()
        );
        assert_eq!(result.currency(), currency);
        assert_eq!(balance.minor_units(), i64::MAX);
    }
}

#[test]
fn six_fractional_digits_are_kept_exactly() {
    assert_eq!(
        accrual(100, "0.000001", "2025-01-01"),
        ("1".into(), "365000000".into(), Currency::Sek)
    );
}
