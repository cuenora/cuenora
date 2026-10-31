use cuenora_domain::{Currency, Money, QuotedRate};

#[test]
fn exact_quote_grammar_and_canonical_spelling() {
    for (input, expected) in [
        ("000.0900", "0.09"),
        ("0.000000000001", "0.000000000001"),
        ("001.000", "1"),
    ] {
        assert_eq!(QuotedRate::parse(input).unwrap().to_string(), expected);
    }
    for input in [
        "",
        "0",
        "0.000",
        "-0.09",
        "+0.09",
        " 0.09",
        "0,09",
        "9e-2",
        ".09",
        "0.",
        "NaN",
        "0.1234567890123",
        "0.0900000000000",
        "１",
        "9999999999999999999999999999999999999999",
    ] {
        assert!(QuotedRate::parse(input).is_err(), "{input}");
    }
}

#[test]
fn indicative_amounts_use_checked_half_even_without_overriding_actuals() {
    for (rate, sent, expected) in [
        ("0.09", "200", "18.00"),
        ("0.125", "1", "0.12"),
        ("0.135", "1", "0.14"),
        ("0.000000000001", "1", "0.00"),
    ] {
        let sent = Money::parse(sent, Currency::Sek).unwrap();
        assert_eq!(
            QuotedRate::parse(rate)
                .unwrap()
                .indicative(sent, Currency::Eur)
                .unwrap()
                .to_string(),
            expected
        );
    }
    let maximum = Money::parse("92233720368547758.07", Currency::Sek).unwrap();
    assert!(
        QuotedRate::parse("2")
            .unwrap()
            .indicative(maximum, Currency::Usd)
            .is_err()
    );
    assert!(
        QuotedRate::parse("170141183460469231731687303715884105727")
            .unwrap()
            .indicative(maximum, Currency::Usd)
            .is_err()
    );
}
