use cuenora_domain::{Currency, Money, MoneyError};

const CURRENCIES: [Currency; 3] = [Currency::Sek, Currency::Eur, Currency::Usd];
const MAX: i64 = i64::MAX;

fn money(amount: &str, currency: Currency) -> Money {
    Money::parse(amount, currency).unwrap()
}

fn assert_value(value: Money, units: i64, currency: Currency, formatted: &str) {
    assert_eq!(value.minor_units(), units);
    assert_eq!(value.currency(), currency);
    assert_eq!(value.to_string(), formatted);
    assert_eq!(Money::parse(&value.to_string(), currency), Ok(value));
}

fn assert_error(result: Result<Money, MoneyError>, error: MoneyError) {
    assert_eq!(result, Err(error));
}

#[test]
fn c01_currency_codes_are_explicit_and_case_sensitive() {
    for (code, currency) in [
        ("SEK", Currency::Sek),
        ("EUR", Currency::Eur),
        ("USD", Currency::Usd),
    ] {
        assert_eq!(code.parse::<Currency>(), Ok(currency));
        assert_eq!(currency.to_string(), code);
    }
    for code in ["GBP", "sek", ""] {
        assert_eq!(
            code.parse::<Currency>(),
            Err(MoneyError::UnsupportedCurrency)
        );
    }
}

#[test]
fn m01_recorded_scale_is_exact() {
    for currency in CURRENCIES {
        for input in ["10", "10.0", "10.00"] {
            let value = money(input, currency);
            assert_value(value, 1000, currency, "10.00");
            assert_eq!(value, money("10.00", currency));
        }
        for input in ["10.5", "10.50"] {
            assert_value(money(input, currency), 1050, currency, "10.50");
        }
    }
}

#[test]
fn m02_leading_zeros_and_signed_values_are_normalized() {
    for currency in CURRENCIES {
        for (input, units, formatted) in [
            ("0010.50", 1050, "10.50"),
            ("0", 0, "0.00"),
            ("-0.00", 0, "0.00"),
            ("-0.01", -1, "-0.01"),
        ] {
            assert_value(money(input, currency), units, currency, formatted);
        }
    }
}

#[test]
fn m03_recorded_excess_scale_is_rejected_without_rounding() {
    for currency in CURRENCIES {
        for input in ["10.505", "10.500"] {
            assert_error(Money::parse(input, currency), MoneyError::ExcessiveScale);
        }
    }
}

#[test]
fn m04_recorded_parser_rejects_invalid_syntax() {
    for currency in CURRENCIES {
        for input in [
            "", " 10", "10 ", "+10", "10,50", "1e2", ".50", "10.", "NaN", "∞",
        ] {
            assert_error(Money::parse(input, currency), MoneyError::InvalidDecimal);
        }
    }
}

#[test]
fn m05_both_symmetric_endpoints_are_valid() {
    for currency in CURRENCIES {
        for (input, units) in [
            ("92233720368547758.07", MAX),
            ("-92233720368547758.07", -MAX),
        ] {
            let value = money(input, currency);
            assert_value(value, units, currency, input);
            assert_eq!(Money::from_minor_units(units, currency), Ok(value));
        }
    }
}

#[test]
fn m06_recorded_values_outside_the_range_are_rejected() {
    for currency in CURRENCIES {
        for input in [
            "92233720368547758.08",
            "-92233720368547758.08",
            "9999999999999999999999999999999999999999999999",
        ] {
            assert_error(Money::parse(input, currency), MoneyError::OutOfRange);
        }
    }
}

#[test]
fn m07_machine_minimum_is_not_a_valid_money_value() {
    for currency in CURRENCIES {
        assert_error(
            Money::from_minor_units(i64::MIN, currency),
            MoneyError::OutOfRange,
        );
    }
}

#[test]
fn a01_addition_and_subtraction_are_decimal_exact() {
    for currency in CURRENCIES {
        assert_value(
            money("0.10", currency)
                .checked_add(money("0.20", currency))
                .unwrap(),
            30,
            currency,
            "0.30",
        );
        assert_value(
            money("0.30", currency)
                .checked_sub(money("0.10", currency))
                .unwrap(),
            20,
            currency,
            "0.20",
        );
    }
}

#[test]
fn a02_arithmetic_can_cross_zero_and_represent_negative_balances() {
    for currency in CURRENCIES {
        assert_value(
            money("100.00", currency)
                .checked_sub(money("150.00", currency))
                .unwrap(),
            -5000,
            currency,
            "-50.00",
        );
        assert_value(
            money("-50.00", currency)
                .checked_add(money("50.00", currency))
                .unwrap(),
            0,
            currency,
            "0.00",
        );
    }
}

#[test]
fn a03_cross_currency_arithmetic_fails_without_changing_operands() {
    for left_currency in CURRENCIES {
        for right_currency in CURRENCIES {
            if left_currency == right_currency {
                continue;
            }
            for amount in ["10.00", "0.00"] {
                let left = money(amount, left_currency);
                let right = money(amount, right_currency);
                let before = (left, right);
                assert_ne!(left, right);
                assert_error(left.checked_add(right), MoneyError::CurrencyMismatch);
                assert_error(left.checked_sub(right), MoneyError::CurrencyMismatch);
                assert_eq!((left, right), before);
            }
        }
    }
}

#[test]
fn a04_overflow_and_excluded_minimum_leave_operands_unchanged() {
    for currency in CURRENCIES {
        let max = Money::from_minor_units(MAX, currency).unwrap();
        let min = Money::from_minor_units(-MAX, currency).unwrap();
        let cent = money("0.01", currency);
        let negative_cent = money("-0.01", currency);
        let before = (max, min, cent, negative_cent);
        assert_error(max.checked_add(cent), MoneyError::OutOfRange);
        assert_error(min.checked_sub(cent), MoneyError::OutOfRange);
        assert_error(max.checked_sub(negative_cent), MoneyError::OutOfRange);
        assert_error(max.checked_add(max), MoneyError::OutOfRange);
        assert_error(min.checked_add(min), MoneyError::OutOfRange);
        assert_eq!((max, min, cent, negative_cent), before);
    }
}

#[test]
fn a05_every_valid_amount_has_an_exact_negation() {
    for currency in CURRENCIES {
        for units in [MAX, -MAX, 0] {
            let value = Money::from_minor_units(units, currency).unwrap();
            let negated = value.checked_neg().unwrap();
            assert_eq!(negated.minor_units(), -units);
            assert_eq!(negated.currency(), currency);
            assert_eq!(negated.checked_neg(), Ok(value));
            assert_eq!(Money::parse(&negated.to_string(), currency), Ok(negated));
        }
    }
}

#[test]
fn a06_arithmetic_identities_hold_at_the_endpoints() {
    for currency in CURRENCIES {
        let max = Money::from_minor_units(MAX, currency).unwrap();
        let min = Money::from_minor_units(-MAX, currency).unwrap();
        let zero = money("0.00", currency);
        assert_eq!(max.checked_add(zero), Ok(max));
        assert_eq!(max.checked_sub(max), Ok(zero));
        assert_eq!(min.checked_add(max), Ok(zero));
    }
}

#[test]
fn r01_calculated_results_round_to_nearest_with_even_ties() {
    for currency in CURRENCIES {
        for (numerator, units, formatted) in [
            (10004, 1000, "10.00"),
            (10005, 1000, "10.00"),
            (10006, 1001, "10.01"),
            (10015, 1002, "10.02"),
        ] {
            assert_value(
                Money::from_minor_ratio(numerator, 10, currency).unwrap(),
                units,
                currency,
                formatted,
            );
        }
    }
}

#[test]
fn r02_rounding_is_symmetric_and_does_not_produce_negative_zero() {
    for currency in CURRENCIES {
        for (numerator, units, formatted) in [
            (-10005, -1000, "-10.00"),
            (-10015, -1002, "-10.02"),
            (5, 0, "0.00"),
            (15, 2, "0.02"),
            (-5, 0, "0.00"),
        ] {
            assert_value(
                Money::from_minor_ratio(numerator, 10, currency).unwrap(),
                units,
                currency,
                formatted,
            );
        }
    }
}

#[test]
fn r03_rounded_results_must_stay_inside_the_symmetric_range() {
    for currency in CURRENCIES {
        for sign in [-1, 1] {
            let inside =
                Money::from_minor_ratio(sign * 92233720368547758074, 10, currency).unwrap();
            assert_eq!(
                inside,
                Money::from_minor_units(sign as i64 * MAX, currency).unwrap()
            );
            assert_eq!(Money::parse(&inside.to_string(), currency), Ok(inside));
            assert_error(
                Money::from_minor_ratio(sign * 92233720368547758075, 10, currency),
                MoneyError::OutOfRange,
            );
        }
    }
}

#[test]
fn r04_nonpositive_denominators_return_a_typed_error() {
    for currency in CURRENCIES {
        for denominator in [0, -1, i128::MIN] {
            assert_error(
                Money::from_minor_ratio(1, denominator, currency),
                MoneyError::InvalidRatio,
            );
        }
    }
}

#[test]
fn r05_extreme_ratio_inputs_never_overflow_intermediate_values() {
    for currency in CURRENCIES {
        assert_error(
            Money::from_minor_ratio(i128::MIN, 1, currency),
            MoneyError::OutOfRange,
        );
        assert_value(
            Money::from_minor_ratio(1, i128::MAX, currency).unwrap(),
            0,
            currency,
            "0.00",
        );
    }
}

#[test]
fn r06_large_remainders_do_not_overflow_the_half_comparison() {
    for currency in CURRENCIES {
        assert_value(
            Money::from_minor_ratio(i128::MAX - 1, i128::MAX, currency).unwrap(),
            1,
            currency,
            "0.01",
        );
    }
}
