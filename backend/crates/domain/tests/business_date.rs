use cuenora_domain::{BusinessDate, BusinessDateError};

#[test]
fn parses_and_displays_approved_business_dates() {
    for input in ["2024-02-29", "2000-02-29", "0001-01-01", "9999-12-31"] {
        let date = BusinessDate::parse(input).expect("date should be valid");
        assert_eq!(date.to_string(), input);
    }
}

#[test]
fn rejects_invalid_calendar_dates_without_normalizing_them() {
    for input in ["2026-02-29", "1900-02-29", "2026-04-31"] {
        assert_eq!(
            BusinessDate::parse(input),
            Err(BusinessDateError::InvalidDate),
            "{input} should be rejected"
        );
    }
}

#[test]
fn reports_years_outside_the_supported_range() {
    for input in ["0000-01-01", "10000-01-01"] {
        assert_eq!(
            BusinessDate::parse(input),
            Err(BusinessDateError::OutOfRange),
            "{input} should be out of range"
        );
    }
}

#[test]
fn rejects_noncanonical_business_date_inputs() {
    for input in [
        "2026-2-01",
        "2026/02/01",
        " 2026-02-01",
        "2026-02-01T00:00:00Z",
        "２０２６-02-01",
        "2026-０2-01",
        "2026-02-0１",
    ] {
        assert_eq!(
            BusinessDate::parse(input),
            Err(BusinessDateError::InvalidDate),
            "{input:?} should be rejected"
        );
    }
}

#[test]
fn accepts_past_and_today_but_rejects_future_dates() {
    let today = BusinessDate::parse("2026-10-29").unwrap();
    let past = BusinessDate::parse("2026-10-28").unwrap();
    let future = BusinessDate::parse("2026-10-30").unwrap();

    assert_eq!(past.ensure_not_future(today), Ok(past));
    assert_eq!(today.ensure_not_future(today), Ok(today));
    assert_eq!(
        future.ensure_not_future(today),
        Err(BusinessDateError::FutureDate)
    );
}
