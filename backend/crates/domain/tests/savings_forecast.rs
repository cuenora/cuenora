use cuenora_domain::{
    BusinessDate, Capitalization, Currency, ForecastDay, Money, NominalAnnualRate, PayoutFrequency,
    SavingsForecastError, forecast_interest,
};
use num_bigint::BigInt;
use num_rational::BigRational;

fn day(date: &str, minor: i64, rate: &str) -> ForecastDay {
    ForecastDay {
        date: BusinessDate::parse(date).unwrap(),
        closing_balance: Money::from_minor_units(minor, Currency::Sek).unwrap(),
        rate: NominalAnnualRate::parse(rate).unwrap(),
    }
}

fn days(start: &str, count: u64, minor: i64, rate: &str) -> Vec<ForecastDay> {
    let start = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap();
    (0..count)
        .map(|n| day(&(start + chrono::Days::new(n)).to_string(), minor, rate))
        .collect()
}

#[test]
fn daily_payouts_round_half_even_and_do_not_mutate_inputs() {
    let input = vec![
        day("2025-01-01", 1005, "3650"),
        day("2025-01-02", 1015, "3650"),
    ];
    let before = input.clone();
    let result =
        forecast_interest(&input, PayoutFrequency::Daily, Capitalization::Exclude).unwrap();
    assert_eq!(result.periods.len(), 2);
    assert_eq!(result.periods[0].payout_minor_units, Some(100.into()));
    assert_eq!(result.periods[1].payout_minor_units, Some(102.into()));
    assert_eq!(result.currency, Currency::Sek);
    assert_eq!(input, before);
}

#[test]
fn monthly_accumulation_is_not_daily_rounded_and_first_period_is_partial() {
    let result = forecast_interest(
        &days("2025-04-20", 12, 100, "182.5"),
        PayoutFrequency::Monthly,
        Capitalization::Exclude,
    )
    .unwrap();
    assert_eq!(result.periods.len(), 2);
    let first = &result.periods[0];
    assert_eq!(first.start.to_string(), "2025-04-20");
    assert_eq!(first.end.to_string(), "2025-04-30");
    assert_eq!(
        first.exact_minor_units,
        BigRational::new(11.into(), 2.into())
    );
    assert_eq!(first.payout_minor_units, Some(6.into()));
    let pending = &result.periods[1];
    assert_eq!(pending.start.to_string(), "2025-05-01");
    assert_eq!(pending.end.to_string(), "2025-05-01");
    assert_eq!(pending.payout_minor_units, None);
    assert_eq!(
        pending.exact_minor_units,
        BigRational::new(1.into(), 2.into())
    );
}

#[test]
fn natural_quarter_and_year_ends_include_partial_first_periods() {
    for (frequency, start, end) in [
        (PayoutFrequency::Quarterly, "2025-03-30", "2025-03-31"),
        (PayoutFrequency::Quarterly, "2025-06-29", "2025-06-30"),
        (PayoutFrequency::Quarterly, "2025-09-29", "2025-09-30"),
        (PayoutFrequency::Annual, "2025-12-30", "2025-12-31"),
    ] {
        let result = forecast_interest(
            &days(start, 3, 100000, "3.65"),
            frequency,
            Capitalization::Exclude,
        )
        .unwrap();
        assert_eq!(result.periods.len(), 2);
        assert_eq!(result.periods[0].end.to_string(), end);
        assert_eq!(result.periods[0].payout_minor_units, Some(20.into()));
        assert_eq!(result.periods[1].payout_minor_units, None);
    }
}

#[test]
fn capitalization_uses_only_rounded_payouts_starting_next_day() {
    let input = days("2025-01-31", 2, 100000, "109.5");
    let excluded =
        forecast_interest(&input, PayoutFrequency::Monthly, Capitalization::Exclude).unwrap();
    let reinvested =
        forecast_interest(&input, PayoutFrequency::Monthly, Capitalization::Reinvest).unwrap();
    assert_eq!(reinvested.periods[0].payout_minor_units, Some(300.into()));
    assert_eq!(
        excluded.periods[1].exact_minor_units,
        BigRational::from_integer(300.into())
    );
    assert_eq!(
        reinvested.periods[1].exact_minor_units,
        BigRational::new(3009.into(), 10.into())
    );
    let result = forecast_interest(
        &days("2025-01-01", 3, 100, "182.5"),
        PayoutFrequency::Daily,
        Capitalization::Reinvest,
    )
    .unwrap();
    assert!(
        result
            .periods
            .iter()
            .all(|p| p.payout_minor_units == Some(0.into()))
    );
}

#[test]
fn daily_capitalization_accumulates_without_fixed_width_overflow() {
    let input = days("2025-01-01", 3, i64::MAX, "36500");
    let result =
        forecast_interest(&input, PayoutFrequency::Daily, Capitalization::Reinvest).unwrap();
    for (period, multiple) in result.periods.iter().zip([1, 2, 4]) {
        assert_eq!(
            period.payout_minor_units,
            Some(BigInt::from(i64::MAX) * multiple)
        );
    }
    let huge = forecast_interest(
        &[day("2025-01-01", 100, &"9".repeat(1024))],
        PayoutFrequency::Daily,
        Capitalization::Reinvest,
    )
    .unwrap();
    assert!(huge.periods[0].payout_minor_units.as_ref().unwrap() > &BigInt::from(i64::MAX));
}

#[test]
fn calendar_year_day_count_rates_and_closing_balances_are_per_day() {
    let input = vec![
        day("2023-12-31", 100000, "3.65"),
        day("2024-01-01", 200000, "3.66"),
    ];
    let result =
        forecast_interest(&input, PayoutFrequency::Annual, Capitalization::Exclude).unwrap();
    assert_eq!(result.periods[0].payout_minor_units, Some(10.into()));
    assert_eq!(
        result.periods[1].exact_minor_units,
        BigRational::from_integer(20.into())
    );
    for (year, count, expected) in [("2024", 29, 290), ("2025", 28, 280)] {
        let rate = if year == "2024" { "3.66" } else { "3.65" };
        let result = forecast_interest(
            &days(&format!("{year}-02-01"), count, 100000, rate),
            PayoutFrequency::Monthly,
            Capitalization::Exclude,
        )
        .unwrap();
        assert_eq!(result.periods[0].payout_minor_units, Some(expected.into()));
    }
    let input = vec![
        day("2025-03-29", 100000, "3.65"),
        day("2025-03-30", 200000, "7.3"),
        day("2025-03-31", -100, "7.3"),
    ];
    let result =
        forecast_interest(&input, PayoutFrequency::Monthly, Capitalization::Exclude).unwrap();
    assert_eq!(result.periods[0].payout_minor_units, Some(50.into()));
}

#[test]
fn input_requires_complete_ordered_single_currency_days() {
    assert_eq!(
        forecast_interest(&[], PayoutFrequency::Daily, Capitalization::Exclude),
        Err(SavingsForecastError::EmptyInput)
    );
    for input in [
        vec![day("2025-01-01", 100, "1"), day("2025-01-03", 100, "1")],
        vec![day("2025-01-01", 100, "1"); 2],
        vec![day("2025-01-02", 100, "1"), day("2025-01-01", 100, "1")],
    ] {
        assert_eq!(
            forecast_interest(&input, PayoutFrequency::Monthly, Capitalization::Exclude),
            Err(SavingsForecastError::NonConsecutiveDates)
        );
    }
    let mut input = days("2025-01-01", 2, 100, "1");
    input[1].closing_balance = Money::from_minor_units(100, Currency::Usd).unwrap();
    assert_eq!(
        forecast_interest(&input, PayoutFrequency::Daily, Capitalization::Exclude),
        Err(SavingsForecastError::CurrencyMismatch)
    );
}

#[test]
fn supported_calendar_bounds_zero_and_negative_bases_are_safe() {
    for date in ["0001-01-01", "9999-12-31"] {
        for currency in [Currency::Sek, Currency::Eur, Currency::Usd] {
            let mut input = day(date, -100, "3.65");
            input.closing_balance = Money::from_minor_units(-100, currency).unwrap();
            let result =
                forecast_interest(&[input], PayoutFrequency::Daily, Capitalization::Reinvest)
                    .unwrap();
            assert_eq!(result.currency, currency);
            assert_eq!(result.periods[0].payout_minor_units, Some(0.into()));
        }
    }
    let result = forecast_interest(
        &days("2025-01-01", 365, 100000, "0"),
        PayoutFrequency::Annual,
        Capitalization::Reinvest,
    )
    .unwrap();
    assert_eq!(result.periods[0].payout_minor_units, Some(0.into()));
}

#[test]
fn complete_year_and_quarters_accumulate_without_daily_rounding() {
    let annual = forecast_interest(
        &days("2024-01-01", 366, 100000, "3.66"),
        PayoutFrequency::Annual,
        Capitalization::Exclude,
    )
    .unwrap();
    assert_eq!(annual.periods.len(), 1);
    assert_eq!(annual.periods[0].payout_minor_units, Some(3660.into()));
    let quarters = forecast_interest(
        &days("2025-01-01", 365, 100000, "3.65"),
        PayoutFrequency::Quarterly,
        Capitalization::Exclude,
    )
    .unwrap();
    assert_eq!(quarters.periods.len(), 4);
    for (period, expected) in quarters.periods.iter().zip([900, 910, 920, 920]) {
        assert_eq!(period.payout_minor_units, Some(expected.into()));
    }
}

#[test]
fn quarterly_and_annual_capitalization_retains_only_forecast_capital() {
    for (frequency, start) in [
        (PayoutFrequency::Quarterly, "2025-03-31"),
        (PayoutFrequency::Annual, "2025-12-31"),
    ] {
        let mut input = days(start, 2, 1000, "3650");
        // 100 minor units projected on day one offset a negative posted base
        // on day two. Floor after combining, not before capitalization.
        input[1].closing_balance = Money::from_minor_units(-50, Currency::Sek).unwrap();
        let result = forecast_interest(&input, frequency, Capitalization::Reinvest).unwrap();
        assert_eq!(result.periods[0].payout_minor_units, Some(100.into()));
        assert_eq!(
            result.periods[1].exact_minor_units,
            BigRational::from_integer(5.into())
        );
        assert_eq!(result.periods[1].payout_minor_units, None);
    }
}
