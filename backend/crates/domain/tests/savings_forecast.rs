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
fn monthly_forecasts_use_actual_month_lengths_and_calendar_year_divisors() {
    // Constant 1,000 SEK at 3.65%: 10 minor units/day in a common year,
    // but 3,650/366 minor units/day in a leap year (the rate is unchanged).
    for (start, count, end, numerator, denominator, payout) in [
        ("2025-01-01", 31, "2025-01-31", 310, 1, 310),
        ("2025-02-01", 28, "2025-02-28", 280, 1, 280),
        ("2025-04-01", 30, "2025-04-30", 300, 1, 300),
        ("2024-02-01", 29, "2024-02-29", 105850, 366, 289),
    ] {
        let result = forecast_interest(
            &days(start, count, 100000, "3.65"),
            PayoutFrequency::Monthly,
            Capitalization::Exclude,
        )
        .unwrap();
        assert_eq!(result.currency, Currency::Sek);
        assert_eq!(result.periods.len(), 1, "{start}");
        let period = &result.periods[0];
        assert_eq!(period.start.to_string(), start);
        assert_eq!(period.end.to_string(), end);
        assert_eq!(
            period.exact_minor_units,
            BigRational::new(numerator.into(), denominator.into()),
            "{start}"
        );
        assert_eq!(period.payout_minor_units, Some(payout.into()), "{start}");
    }
}

#[test]
fn mid_period_deposit_and_withdrawal_affect_their_closing_day_onward() {
    let mut input = days("2025-04-01", 10, 100000, "3.65");
    // Deposit 1,000 SEK on April 11; withdraw 1,500 SEK on April 21.
    input.extend(days("2025-04-11", 10, 200000, "3.65"));
    input.extend(days("2025-04-21", 10, 50000, "3.65"));
    let before = input.clone();
    let daily = forecast_interest(&input, PayoutFrequency::Daily, Capitalization::Exclude).unwrap();
    assert_eq!(daily.periods.len(), 30);
    for (period, expected) in daily
        .periods
        .iter()
        .zip([10; 10].into_iter().chain([20; 10]).chain([5; 10]))
    {
        assert_eq!(
            period.exact_minor_units,
            BigRational::from_integer(expected.into())
        );
        assert_eq!(period.payout_minor_units, Some(expected.into()));
    }
    for capitalization in [Capitalization::Exclude, Capitalization::Reinvest] {
        let monthly = forecast_interest(&input, PayoutFrequency::Monthly, capitalization).unwrap();
        assert_eq!(monthly.currency, Currency::Sek);
        assert_eq!(monthly.periods.len(), 1);
        let period = &monthly.periods[0];
        assert_eq!(period.start.to_string(), "2025-04-01");
        assert_eq!(period.end.to_string(), "2025-04-30");
        // 10 days * 10 + 10 days * 20 + 10 days * 5 = 350 minor units.
        assert_eq!(
            period.exact_minor_units,
            BigRational::from_integer(350.into())
        );
        assert_eq!(period.payout_minor_units, Some(350.into()));
    }
    assert_eq!(input, before);
}

#[test]
fn zero_rate_forecasts_have_no_accrual_or_projected_capital() {
    let input = days("2025-01-01", 365, 100000, "0.000000");
    let before = input.clone();
    for (frequency, count) in [
        (PayoutFrequency::Daily, 365),
        (PayoutFrequency::Monthly, 12),
        (PayoutFrequency::Quarterly, 4),
        (PayoutFrequency::Annual, 1),
    ] {
        for capitalization in [Capitalization::Exclude, Capitalization::Reinvest] {
            let result = forecast_interest(&input, frequency, capitalization).unwrap();
            assert_eq!(result.currency, Currency::Sek);
            assert_eq!(result.periods.len(), count);
            for period in &result.periods {
                assert_eq!(
                    period.exact_minor_units,
                    BigRational::from_integer(0.into())
                );
                assert_eq!(period.payout_minor_units, Some(0.into()));
            }
        }
    }
    assert_eq!(input, before);
}

#[test]
fn historical_and_future_mid_period_rate_changes_preserve_the_payout_calendar() {
    // Fixed past/future fixtures, not dates derived from a system clock.
    // The caller selects effective rates; this primitive does not query history.
    for year in [2001, 2099] {
        let mut input = days(&format!("{year}-03-01"), 14, 100000, "3.65");
        input.extend(days(&format!("{year}-03-15"), 5, 100000, "0"));
        input.extend(days(&format!("{year}-03-20"), 13, 100000, "7.3"));
        let before = input.clone();
        let daily =
            forecast_interest(&input, PayoutFrequency::Daily, Capitalization::Exclude).unwrap();
        assert_eq!(daily.periods.len(), 32);
        for (period, expected) in daily
            .periods
            .iter()
            .zip([10; 14].into_iter().chain([0; 5]).chain([20; 13]))
        {
            assert_eq!(
                period.exact_minor_units,
                BigRational::from_integer(expected.into())
            );
            assert_eq!(period.payout_minor_units, Some(expected.into()));
        }
        let result =
            forecast_interest(&input, PayoutFrequency::Monthly, Capitalization::Exclude).unwrap();
        assert_eq!(result.currency, Currency::Sek);
        assert_eq!(result.periods.len(), 2);
        let march = &result.periods[0];
        assert_eq!(march.start.to_string(), format!("{year}-03-01"));
        assert_eq!(march.end.to_string(), format!("{year}-03-31"));
        // 14 * 10 + 5 * 0 + 12 * 20 = 380 minor units, one payout.
        assert_eq!(
            march.exact_minor_units,
            BigRational::from_integer(380.into())
        );
        assert_eq!(march.payout_minor_units, Some(380.into()));
        let april = &result.periods[1];
        assert_eq!(april.start.to_string(), format!("{year}-04-01"));
        assert_eq!(april.end.to_string(), format!("{year}-04-01"));
        assert_eq!(
            april.exact_minor_units,
            BigRational::from_integer(20.into())
        );
        assert_eq!(april.payout_minor_units, None);
        assert_eq!(input, before);
    }
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
    assert_eq!(annual.periods[0].start.to_string(), "2024-01-01");
    assert_eq!(annual.periods[0].end.to_string(), "2024-12-31");
    assert_eq!(
        annual.periods[0].exact_minor_units,
        BigRational::from_integer(3660.into())
    );
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

#[test]
fn hand_worked_two_to_three_percent_change_closes_one_monthly_payout() {
    let mut input = days("2025-03-01", 14, 100000, "2");
    input.extend(days("2025-03-15", 17, 100000, "3"));
    let before = input.clone();
    let result =
        forecast_interest(&input, PayoutFrequency::Monthly, Capitalization::Exclude).unwrap();
    assert_eq!(result.currency, Currency::Sek);
    assert_eq!(result.periods.len(), 1);
    let period = &result.periods[0];
    assert_eq!(period.start.to_string(), "2025-03-01");
    assert_eq!(period.end.to_string(), "2025-03-31");
    // 1,000 * (0.02 * 14 + 0.03 * 17) / 365 = 158/73 SEK.
    // Exact minor units are 15,800/73; round once to 216 (2.16 SEK).
    assert_eq!(
        period.exact_minor_units,
        BigRational::new(15800.into(), 73.into())
    );
    assert_eq!(period.payout_minor_units, Some(216.into()));
    assert_eq!(input, before);
}

#[test]
fn hand_worked_monthly_half_even_ties_preserve_exact_accrual() {
    // One day ending a monthly period: 10.05/10.15 SEK at 3,650% nominal
    // accrues exactly 1.005/1.015 SEK. High rates make the ties inspectable.
    for (base, numerator, payout) in [(1005, 201, 100), (1015, 203, 102)] {
        let input = vec![day("2025-04-30", base, "3650")];
        let before = input.clone();
        let result =
            forecast_interest(&input, PayoutFrequency::Monthly, Capitalization::Exclude).unwrap();
        assert_eq!(result.currency, Currency::Sek);
        assert_eq!(result.periods.len(), 1);
        let period = &result.periods[0];
        assert_eq!(period.start.to_string(), "2025-04-30");
        assert_eq!(period.end.to_string(), "2025-04-30");
        assert_eq!(
            period.exact_minor_units,
            BigRational::new(numerator.into(), 2.into())
        );
        assert_eq!(period.payout_minor_units, Some(payout.into()));
        assert_eq!(input, before);
    }
}
