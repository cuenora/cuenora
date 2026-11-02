//! P08.09/P08.11 public HTTP acceptance against disposable PostgreSQL only.
mod common;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn send(
    router: Router,
    token: Option<&str>,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    if value.get("code").is_some() {
        assert_eq!(value["request_id"], id);
        assert!(value["details"].is_null());
    }
    (status, value)
}
async fn seed(
    pool: &PgPool,
    verified: bool,
    kind: &str,
    currency: &str,
) -> (String, String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let owner: String = sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at,profile_time_zone) VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END,'Europe/Stockholm') RETURNING id::text").bind(email).bind(verified).fetch_one(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&owner)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at) VALUES($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')").bind(session).bind(&owner).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    let account: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Synthetic forecast',$2,$3) RETURNING id::text").bind(&owner).bind(kind).bind(currency).fetch_one(pool).await.unwrap();
    (owner, tokens.access.expose_secret().to_owned(), account)
}
async fn terms(
    pool: &PgPool,
    owner: &str,
    account: &str,
    date: &str,
    frequency: &str,
    capitalize: bool,
) {
    sqlx::query("INSERT INTO savings_terms(user_id,account_id,effective_on,payout_frequency,capitalize,rate_basis,day_count,balance_timing,rounding) VALUES($1::uuid,$2::uuid,$3::date,$4,$5,'nominal_annual','actual_calendar_year','daily_close','period_end_half_even')").bind(owner).bind(account).bind(date).bind(frequency).bind(capitalize).execute(pool).await.unwrap();
}
async fn rate(pool: &PgPool, owner: &str, account: &str, date: &str, percent: &str) {
    sqlx::query("INSERT INTO savings_rate_periods(user_id,account_id,effective_on,nominal_annual_percent) VALUES($1::uuid,$2::uuid,$3::date,$4::numeric)").bind(owner).bind(account).bind(date).bind(percent).execute(pool).await.unwrap();
}
async fn fund(pool: &PgPool, owner: &str, account: &str, currency: &str, amount: i64) {
    let event: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,account_id) VALUES($1::uuid,'opening_balance','2020-01-01',$2,$3,$4::uuid) RETURNING id::text").bind(owner).bind(amount).bind(currency).bind(account).fetch_one(pool).await.unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5,'2020-01-01')").bind(owner).bind(event).bind(account).bind(amount).bind(currency).execute(pool).await.unwrap();
}
async fn snapshot(pool: &PgPool) -> String {
    let users = common::snapshot(pool).await;
    let financial: String = sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM savings_terms t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM savings_rate_periods t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM account_movements t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transaction_audit t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM accounts t))::text)").fetch_one(pool).await.unwrap();
    format!("{users}{financial}")
}
fn uri(account: &str, start: &str, end: &str) -> String {
    format!("/v1/accounts/{account}/savings-forecast?start_on={start}&end_on={end}")
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn explains_flat_balance_inclusive_rates_and_capitalization_without_writes() {
    let pool = common::pool().await;
    let (owner, token, account) = seed(&pool, true, "savings", "SEK").await;
    fund(&pool, &owner, &account, "SEK", 100000).await;
    terms(&pool, &owner, &account, "2099-01-01", "monthly", true).await;
    rate(&pool, &owner, &account, "2099-01-01", "109.5").await;
    rate(&pool, &owner, &account, "2099-02-01", "36.5").await;
    // Outside-window versions must not alter the initial applicable version or result.
    terms(&pool, &owner, &account, "2099-03-01", "annual", false).await;
    rate(&pool, &owner, &account, "2099-03-01", "0").await;
    let before = snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let path = uri(&account, "2099-01-30", "2099-02-02");
    let (status, result) = send(router.clone(), Some(&token), "GET", &path, "").await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["account_id"], account);
    assert_eq!(result["currency"], "SEK");
    assert_eq!(result["posted_balance"], "1000.00");
    assert_eq!(result["start_on"], "2099-01-30");
    assert_eq!(result["end_on"], "2099-02-02");
    assert_eq!(result["balance_assumption"], "constant_posted_snapshot");
    assert_eq!(result["includes_prior_accrual"], false);
    assert_eq!(result["partial_first_period"], true);
    assert_eq!(result["terms"]["capitalize"], true);
    assert_eq!(result["rate_segments"].as_array().unwrap().len(), 2);
    assert_eq!(result["rate_segments"][0]["end_on"], "2099-01-31");
    assert_eq!(result["rate_segments"][1]["start_on"], "2099-02-01");
    assert_eq!(result["rate_segments"][1]["nominal_annual_percent"], "36.5");
    assert_eq!(result["rate_segments"][1]["days_in_year"], 365);
    assert_eq!(
        result["periods"],
        json!([
            {"start_on":"2099-01-30","end_on":"2099-01-31","exact_accrual_minor_units":{"numerator":"600","denominator":"1"},"projected_payout":"6.00"},
            {"start_on":"2099-02-01","end_on":"2099-02-02","exact_accrual_minor_units":{"numerator":"1006","denominator":"5"},"projected_payout":null}
        ])
    );
    assert_eq!(
        send(router, Some(&token), "GET", &path, "").await,
        (status, result)
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn rejects_missing_coverage_and_changed_terms_but_accepts_equivalent_versions() {
    let pool = common::pool().await;
    let (owner, token, account) = seed(&pool, true, "savings", "SEK").await;
    let router = app(Some(pool.clone()));
    let path = uri(&account, "2099-03-01", "2099-03-31");
    let response = send(router.clone(), Some(&token), "GET", &path, "").await;
    assert_eq!(response.0, StatusCode::CONFLICT);
    assert_eq!(response.1["code"], "savings_forecast_configuration_missing");
    terms(&pool, &owner, &account, "2099-03-01", "monthly", false).await;
    rate(&pool, &owner, &account, "2099-03-02", "3.65").await;
    assert_eq!(
        send(router.clone(), Some(&token), "GET", &path, "").await.0,
        StatusCode::CONFLICT
    );
    rate(&pool, &owner, &account, "2099-01-01", "3.65").await;
    terms(&pool, &owner, &account, "2099-03-15", "monthly", false).await;
    assert_eq!(
        send(router.clone(), Some(&token), "GET", &path, "").await.0,
        StatusCode::OK
    );
    terms(&pool, &owner, &account, "2099-03-31", "monthly", true).await;
    let before = snapshot(&pool).await;
    let response = send(router.clone(), Some(&token), "GET", &path, "").await;
    assert_eq!(response.0, StatusCode::CONFLICT);
    assert_eq!(
        response.1["code"],
        "savings_forecast_terms_change_unsupported"
    );
    assert_eq!(snapshot(&pool).await, before);
    // A window starting on the change uses the new mode rather than rejecting it.
    assert_eq!(
        send(
            router.clone(),
            Some(&token),
            "GET",
            &uri(&account, "2099-03-31", "2099-03-31"),
            ""
        )
        .await
        .0,
        StatusCode::OK
    );
    terms(&pool, &owner, &account, "2099-04-02", "annual", true).await;
    assert_eq!(
        send(
            router,
            Some(&token),
            "GET",
            &uri(&account, "2099-04-01", "2099-04-02"),
            ""
        )
        .await
        .1["code"],
        "savings_forecast_terms_change_unsupported"
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn enforces_identity_verification_ownership_and_profile_zone() {
    let pool = common::pool().await;
    let (owner, token, account) = seed(&pool, true, "savings", "SEK").await;
    let (_, foreign_token, _) = seed(&pool, true, "savings", "SEK").await;
    let (_, pending, _) = seed(&pool, false, "savings", "SEK").await;
    let (_, normal_token, normal) = seed(&pool, true, "normal", "SEK").await;
    let router = app(Some(pool.clone()));
    let path = uri(&account, "2099-01-01", "2099-01-01");
    let before = snapshot(&pool).await;
    assert_eq!(
        send(router.clone(), None, "GET", &path, "").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(router.clone(), Some("ca1_invalid"), "GET", &path, "")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(router.clone(), Some(&pending), "GET", &path, "")
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let foreign = send(router.clone(), Some(&foreign_token), "GET", &path, "").await;
    let missing = send(
        router.clone(),
        Some(&token),
        "GET",
        &uri(
            "00000000-0000-0000-0000-000000000000",
            "2099-01-01",
            "2099-01-01",
        ),
        "",
    )
    .await;
    let normal = send(
        router.clone(),
        Some(&normal_token),
        "GET",
        &uri(&normal, "2099-01-01", "2099-01-01"),
        "",
    )
    .await;
    for response in [foreign, missing, normal] {
        assert_eq!(response.0, StatusCode::NOT_FOUND);
        assert_eq!(response.1["code"], "not_found");
    }
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET profile_time_zone=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        send(router.clone(), Some(&token), "GET", &path, "").await.1["code"],
        "profile_zone_unconfirmed"
    );
    sqlx::query("UPDATE users SET profile_time_zone='Invalid/Zone' WHERE id=$1::uuid")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        send(router, Some(&token), "GET", &path, "").await.1["code"],
        "unsupported_time_zone"
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn validates_bounded_dates_query_body_and_method() {
    let pool = common::pool().await;
    let (owner, token, account) = seed(&pool, true, "savings", "SEK").await;
    terms(&pool, &owner, &account, "0001-01-01", "annual", false).await;
    rate(&pool, &owner, &account, "0001-01-01", "0").await;
    let router = app(Some(pool.clone()));
    let root = format!("/v1/accounts/{account}/savings-forecast");
    let before = snapshot(&pool).await;
    for suffix in [
        "",
        "?start_on=2099-01-01",
        "?start_on=2099-01-02&end_on=2099-01-01",
        "?start_on=2099-01-01&end_on=2100-01-02",
        "?start_on=2020-01-01&end_on=2020-01-02",
        "?start_on=2099-02-29&end_on=2099-03-01",
        "?start_on=2099-01-01&end_on=2099-01-01&start_on=2099-01-02",
        "?start_on=2099-01-01&end_on=2099-01-01&balance=1000",
    ] {
        assert_eq!(
            send(
                router.clone(),
                Some(&token),
                "GET",
                &format!("{root}{suffix}"),
                ""
            )
            .await
            .0,
            StatusCode::BAD_REQUEST,
            "{suffix}"
        );
    }
    let path = uri(&account, "2099-01-01", "2099-01-01");
    assert_eq!(
        send(router.clone(), Some(&token), "GET", &path, "{}")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(router.clone(), Some(&token), "POST", &path, "")
            .await
            .0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        send(
            router.clone(),
            Some(&token),
            "GET",
            &uri("invalid", "2099-01-01", "2099-01-01"),
            ""
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            router.clone(),
            Some(&token),
            "GET",
            &uri(&account, "2099-01-01", "2100-01-01"),
            ""
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(
            router,
            Some(&token),
            "GET",
            &uri(&account, "9999-12-31", "9999-12-31"),
            ""
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn supports_all_calendars_currencies_zero_floor_and_archived_accounts() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    for currency in ["SEK", "EUR", "USD"] {
        for frequency in ["daily", "monthly", "quarterly", "annual"] {
            let (owner, token, account) = seed(&pool, true, "savings", currency).await;
            fund(&pool, &owner, &account, currency, -10000).await;
            terms(&pool, &owner, &account, "2099-01-01", frequency, false).await;
            rate(&pool, &owner, &account, "2099-01-01", "3.65").await;
            sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
                .bind(&account)
                .execute(&pool)
                .await
                .unwrap();
            let before = snapshot(&pool).await;
            let response = send(
                router.clone(),
                Some(&token),
                "GET",
                &uri(&account, "2099-12-31", "2099-12-31"),
                "",
            )
            .await;
            assert_eq!(response.0, StatusCode::OK, "{}", response.1);
            assert_eq!(response.1["currency"], currency);
            assert_eq!(response.1["posted_balance"], "-100.00");
            assert_eq!(response.1["periods"][0]["projected_payout"], "0.00");
            assert_eq!(snapshot(&pool).await, before);
        }
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn explains_leap_day_count_and_bounds_arbitrary_precision_compounding() {
    let pool = common::pool().await;
    let (owner, token, account) = seed(&pool, true, "savings", "EUR").await;
    fund(&pool, &owner, &account, "EUR", 100000).await;
    terms(&pool, &owner, &account, "2096-01-01", "monthly", false).await;
    rate(&pool, &owner, &account, "2096-01-01", "3.66").await;
    let router = app(Some(pool.clone()));
    let response = send(
        router.clone(),
        Some(&token),
        "GET",
        &uri(&account, "2096-02-28", "2096-02-29"),
        "",
    )
    .await;
    assert_eq!(response.0, StatusCode::OK, "{}", response.1);
    assert_eq!(response.1["rate_segments"][0]["days_in_year"], 366);
    assert_eq!(
        response.1["periods"][0]["exact_accrual_minor_units"],
        json!({"numerator":"20","denominator":"1"})
    );
    assert_eq!(response.1["periods"][0]["projected_payout"], "0.20");
    terms(&pool, &owner, &account, "2099-01-01", "daily", true).await;
    rate(&pool, &owner, &account, "2099-01-01", &"9".repeat(1024)).await;
    let before = snapshot(&pool).await;
    let response = send(
        router.clone(),
        Some(&token),
        "GET",
        &uri(&account, "2099-01-01", "2099-12-31"),
        "",
    )
    .await;
    assert_eq!(response.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.1["code"], "savings_forecast_too_complex");
    // The accepted rate is not narrowed: a short calculation still succeeds.
    let response = send(
        router,
        Some(&token),
        "GET",
        &uri(&account, "2099-01-01", "2099-01-01"),
        "",
    )
    .await;
    assert_eq!(response.0, StatusCode::OK, "{}", response.1);
    assert!(
        response.1["periods"][0]["projected_payout"]
            .as_str()
            .unwrap()
            .len()
            > 1000
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn concurrent_postings_and_rate_appends_do_not_mix_snapshot_versions() {
    let pool = common::pool().await;
    let (owner, token, account) = seed(&pool, true, "savings", "SEK").await;
    fund(&pool, &owner, &account, "SEK", 100000).await;
    terms(&pool, &owner, &account, "2099-01-01", "monthly", false).await;
    rate(&pool, &owner, &account, "2099-01-01", "109.5").await;
    let router = app(Some(pool.clone()));
    // Pause the terms read only after the handler has taken its owner/account
    // snapshot. Other tables remain writable by a concurrent transaction.
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE savings_terms IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let reader_router = router.clone();
    let reader_token = token.clone();
    let path = uri(&account, "2099-01-31", "2099-01-31");
    let reader_path = path.clone();
    let reader = tokio::spawn(async move {
        send(reader_router, Some(&reader_token), "GET", &reader_path, "").await
    });
    let mut blocked = false;
    for _ in 0..100 {
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT effective_on::text,payout_frequency%'").fetch_one(&pool).await.unwrap();
        if count > 0 {
            blocked = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    if !blocked {
        gate.rollback().await.unwrap();
        panic!("forecast did not reach the controlled read boundary");
    }
    fund(&pool, &owner, &account, "SEK", 50000).await;
    rate(&pool, &owner, &account, "2099-01-31", "36.5").await;
    gate.commit().await.unwrap();
    let before = snapshot(&pool).await;
    let response = reader.await.unwrap();
    assert_eq!(response.0, StatusCode::OK, "{}", response.1);
    assert_eq!(response.1["posted_balance"], "1000.00");
    assert_eq!(
        response.1["rate_segments"][0]["nominal_annual_percent"],
        "109.5"
    );
    assert_eq!(response.1["periods"][0]["projected_payout"], "3.00");
    let response = send(router, Some(&token), "GET", &path, "").await;
    assert_eq!(response.0, StatusCode::OK, "{}", response.1);
    assert_eq!(response.1["posted_balance"], "1500.00");
    assert_eq!(
        response.1["rate_segments"][0]["nominal_annual_percent"],
        "36.5"
    );
    assert_eq!(response.1["periods"][0]["projected_payout"], "1.50");
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn mid_period_rate_switch_preserves_accrual_and_owner_guards_are_read_only() {
    let pool = common::pool().await;
    let (owner, token, account) = seed(&pool, true, "savings", "USD").await;
    fund(&pool, &owner, &account, "USD", 100000).await;
    terms(&pool, &owner, &account, "2099-01-01", "monthly", true).await;
    rate(&pool, &owner, &account, "2099-01-01", "36.5").await;
    rate(&pool, &owner, &account, "2099-01-31", "73").await;
    let router = app(Some(pool.clone()));
    let path = uri(&account, "2099-01-30", "2099-02-01");
    let before = snapshot(&pool).await;
    let response = send(router.clone(), Some(&token), "GET", &path, "").await;
    assert_eq!(response.0, StatusCode::OK, "{}", response.1);
    assert_eq!(
        response.1["periods"],
        json!([
            {"start_on":"2099-01-30","end_on":"2099-01-31","exact_accrual_minor_units":{"numerator":"300","denominator":"1"},"projected_payout":"3.00"},
            {"start_on":"2099-02-01","end_on":"2099-02-01","exact_accrual_minor_units":{"numerator":"1003","denominator":"5"},"projected_payout":null}
        ])
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    let response = send(router.clone(), Some(&token), "GET", &path, "").await;
    assert_eq!(response.0, StatusCode::FORBIDDEN);
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET email_verified_at=statement_timestamp(),profile_time_zone=NULL WHERE id=$1::uuid").bind(&owner).execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    let response = send(router, Some(&token), "GET", &path, "").await;
    assert_eq!(response.0, StatusCode::CONFLICT);
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn every_payout_and_capitalization_mode_preserves_posted_balances_and_history() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    for frequency in ["daily", "monthly", "quarterly", "annual"] {
        for capitalize in [false, true] {
            let (owner, token, account) = seed(&pool, true, "savings", "SEK").await;
            fund(&pool, &owner, &account, "SEK", 100000).await;
            terms(&pool, &owner, &account, "2099-01-01", frequency, capitalize).await;
            rate(&pool, &owner, &account, "2099-01-01", "36.5").await;
            let balances = send(router.clone(), Some(&token), "GET", "/v1/accounts", "").await;
            assert_eq!(balances.0, StatusCode::OK, "{}", balances.1);
            assert_eq!(balances.1["accounts"][0]["balance"], "1000.00");
            let history = send(router.clone(), Some(&token), "GET", "/v1/transactions", "").await;
            assert_eq!(history.0, StatusCode::OK, "{}", history.1);
            let before = snapshot(&pool).await;
            let path = uri(&account, "2099-01-01", "2099-12-31");
            let first = send(router.clone(), Some(&token), "GET", &path, "").await;
            assert_eq!(first.0, StatusCode::OK, "{}", first.1);
            assert_eq!(first.1["posted_balance"], "1000.00");
            assert_eq!(first.1["terms"]["capitalize"], capitalize);
            // Positive payouts cross real period ends, not just an unrounded open tail.
            let payout = match frequency {
                "daily" => "1.00",
                "monthly" => "31.00",
                "quarterly" => "90.00",
                "annual" => "365.00",
                _ => unreachable!(),
            };
            assert_eq!(first.1["periods"][0]["projected_payout"], payout);
            let (second, third) = tokio::join!(
                send(router.clone(), Some(&token), "GET", &path, ""),
                send(router.clone(), Some(&token), "GET", &path, "")
            );
            assert_eq!(second, first);
            assert_eq!(third, first);
            assert_eq!(snapshot(&pool).await, before);
            assert_eq!(
                send(router.clone(), Some(&token), "GET", "/v1/accounts", "").await,
                balances
            );
            assert_eq!(
                send(router.clone(), Some(&token), "GET", "/v1/transactions", "").await,
                history
            );
            let actual: i64 = sqlx::query_scalar("SELECT count(*) FROM transactions WHERE user_id=$1::uuid AND kind='interest_income'")
                .bind(&owner).fetch_one(&pool).await.unwrap();
            assert_eq!(actual, 0);
        }
    }
}
