//! Account history and payment planning through the public HTTP boundary.
mod common;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn send(
    router: Router,
    token: Option<&str>,
    uri: &str,
    body: &str,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::get(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(response.into_body(), 256 * 1024).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if body.get("code").is_some() {
        assert_eq!(
            body["request_id"],
            headers["x-request-id"].to_str().unwrap()
        );
        assert_eq!(body["details"], Value::Null);
    }
    (status, headers, body)
}

async fn seed(pool: &PgPool, verified: bool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let user: String = sqlx::query_scalar("INSERT INTO users (email_original,email_address,email_verified_at) VALUES ($1,$1,CASE WHEN $2 THEN statement_timestamp() END) RETURNING id::text")
        .bind(email).bind(verified).fetch_one(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions (user_id) VALUES ($1::uuid) RETURNING id::text",
    )
    .bind(&user)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens (session_id,user_id,token_hash,expires_at) VALUES ($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')")
        .bind(session).bind(&user).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    (user, tokens.access.expose_secret().to_owned())
}

async fn account(pool: &PgPool, owner: &str, name: &str, minor: i64, date: &str) -> String {
    let id: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,$2,'normal','SEK') RETURNING id::text").bind(owner).bind(name).fetch_one(pool).await.unwrap();
    let event: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,account_id,amount_minor,currency,occurred_on) VALUES($1::uuid,'opening_balance',$2::uuid,$3,'SEK',$4::date) RETURNING id::text").bind(owner).bind(&id).bind(minor).bind(date).fetch_one(pool).await.unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,$4,'SEK',$5::date)").bind(owner).bind(event).bind(&id).bind(minor).bind(date).execute(pool).await.unwrap();
    id
}

async fn rule(
    pool: &PgPool,
    owner: &str,
    account: Option<&str>,
    amount: i64,
    starts: &str,
    paused: bool,
) -> String {
    sqlx::query_scalar("INSERT INTO recurring_rules(user_id,account_id,amount_minor,currency,cadence,starts_on,time_zone,state,paused_on) VALUES($1::uuid,$2::uuid,$3,'SEK','weekly',$4::date,'Etc/UTC',CASE WHEN $5 THEN 'paused' ELSE 'active' END,CASE WHEN $5 THEN $4::date END) RETURNING id::text")
        .bind(owner).bind(account).bind(amount).bind(starts).bind(paused).fetch_one(pool).await.unwrap()
}
async fn occurrence(
    pool: &PgPool,
    owner: &str,
    rule: &str,
    account: Option<&str>,
    amount: i64,
    dates: (&str, &str),
    state: &str,
) {
    sqlx::query("INSERT INTO recurring_occurrences(user_id,rule_id,account_id,amount_minor,currency,scheduled_on,due_on,state) VALUES($1::uuid,$2::uuid,$3::uuid,$4,'SEK',$5::date,$6::date,$7)").bind(owner).bind(rule).bind(account).bind(amount).bind(dates.0).bind(dates.1).bind(state).execute(pool).await.unwrap();
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn daily_history_carries_forward_and_checks_exact_date_and_owner_boundaries() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (_, other) = seed(&pool, true).await;
    let id = account(&pool, &owner, "History", 12345, "2024-02-28").await;
    let router = app(Some(pool.clone()));
    let uri = format!("/v1/accounts/{id}/balance-history?date_from=2024-02-29&date_to=2024-03-01");
    let before = all_tables_snapshot(&pool).await;
    let (status, _, history) = send(router.clone(), Some(&token), &uri, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(history["history_basis"], "current_corrected_ledger");
    assert_eq!(history["opening_balance"], "123.45");
    assert_eq!(
        history["days"],
        json!([{"date":"2024-02-29","balance":"123.45"},{"date":"2024-03-01","balance":"123.45"}])
    );
    assert_eq!(
        send(router.clone(), Some(&other), &uri, "").await.0,
        StatusCode::NOT_FOUND
    );
    let foreign_plan = send(
        router.clone(),
        Some(&other),
        "/v1/reports/account-planning?through=2024-03-01",
        "",
    )
    .await;
    assert_eq!(foreign_plan.0, StatusCode::OK);
    assert!(foreign_plan.2["accounts"].as_array().unwrap().is_empty());
    for row in foreign_plan.2["unassigned"].as_array().unwrap() {
        assert_eq!(row["pending_total"], "0.00");
        assert_eq!(row["projected_total"], "0.00");
    }
    for (from, to, expected) in [
        ("2024-01-01", "2024-12-31", StatusCode::OK),
        ("2024-01-01", "2025-01-01", StatusCode::BAD_REQUEST),
        ("9999-12-31", "9999-12-31", StatusCode::OK),
        ("0001-01-01", "0001-01-01", StatusCode::OK),
    ] {
        assert_eq!(
            send(
                router.clone(),
                Some(&token),
                &format!("/v1/accounts/{id}/balance-history?date_from={from}&date_to={to}"),
                ""
            )
            .await
            .0,
            expected
        );
    }
    assert_eq!(before, all_tables_snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn planning_preserves_pending_snapshots_deduplicates_future_slots_and_marks_backlog_incomplete()
 {
    use chrono::{DateTime, Days, Utc};
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let today = DateTime::<Utc>::from(std::time::SystemTime::now()).date_naive();
    let future = today.checked_add_days(Days::new(1)).unwrap().to_string();
    let cutoff = today.checked_add_days(Days::new(7)).unwrap().to_string();
    let later = today.checked_add_days(Days::new(8)).unwrap().to_string();
    let yesterday = today.checked_sub_days(Days::new(1)).unwrap().to_string();
    let a = account(&pool, &owner, "A", 30000, &yesterday).await;
    let b = account(&pool, &owner, "B", 200000, &yesterday).await;
    let projected = rule(&pool, &owner, Some(&a), 80000, &future, false).await;
    let saved = rule(&pool, &owner, Some(&b), 99999, &yesterday, true).await;
    occurrence(
        &pool,
        &owner,
        &saved,
        Some(&a),
        20000,
        (&yesterday, &yesterday),
        "planned",
    )
    .await;
    let unassigned = rule(&pool, &owner, None, 5000, &future, false).await;
    let _ = unassigned;
    let unassigned_saved = rule(&pool, &owner, None, 3000, &yesterday, true).await;
    occurrence(
        &pool,
        &owner,
        &unassigned_saved,
        None,
        3000,
        (&yesterday, &yesterday),
        "planned",
    )
    .await;
    rule(&pool, &owner, Some(&b), 10000, &today.to_string(), false).await;
    let skipped = rule(&pool, &owner, Some(&a), 90000, &future, false).await;
    occurrence(
        &pool,
        &owner,
        &skipped,
        Some(&a),
        90000,
        (&future, &future),
        "skipped",
    )
    .await;
    let postponed = rule(&pool, &owner, Some(&a), 70000, &future, false).await;
    occurrence(
        &pool,
        &owner,
        &postponed,
        Some(&a),
        70000,
        (&future, &later),
        "planned",
    )
    .await;
    let router = app(Some(pool.clone()));
    let uri = format!("/v1/reports/account-planning?through={cutoff}");
    let before = all_tables_snapshot(&pool).await;
    let (status, _, report) = send(router.clone(), Some(&token), &uri, "").await;
    assert_eq!(status, StatusCode::OK, "{report}");
    let rows = report["accounts"].as_array().unwrap();
    let ar = rows.iter().find(|r| r["account_id"] == a).unwrap();
    let br = rows.iter().find(|r| r["account_id"] == b).unwrap();
    assert_eq!(ar["current_balance"], "300.00");
    assert_eq!(ar["pending_total"], "200.00");
    assert_eq!(ar["projected_total"], "800.00");
    assert_eq!(ar["planning_difference"], "-700.00");
    assert_eq!(ar["complete"], true);
    assert_eq!(br["complete"], false);
    assert!(br["planning_difference"].is_null());
    assert_eq!(report["complete"], false);
    assert_eq!(
        report["unassigned"][0],
        json!({"currency":"SEK","pending_total":"30.00","projected_total":"50.00","complete":true})
    );
    assert_eq!(before, all_tables_snapshot(&pool).await);
    occurrence(
        &pool,
        &owner,
        &projected,
        Some(&a),
        80000,
        (&future, &future),
        "planned",
    )
    .await;
    let credentials = common::snapshot(&pool).await;
    let next = send(router, Some(&token), &uri, "").await.2;
    let ar = next["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account_id"] == a)
        .unwrap();
    assert_eq!(ar["pending_total"], "1000.00");
    assert_eq!(ar["projected_total"], "0.00");
    assert_eq!(ar["planning_difference"], "-700.00");
    assert_eq!(credentials, common::snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn strict_inputs_verified_identity_and_profile_default_are_enforced() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (_, pending) = seed(&pool, false).await;
    let id = account(&pool, &owner, "Defaults", 100, "2024-01-01").await;
    let router = app(Some(pool.clone()));
    let explicit = "/v1/reports/account-planning?through=2024-01-01";
    assert_eq!(
        send(router.clone(), None, explicit, "").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(router.clone(), Some(&pending), explicit, "").await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            router.clone(),
            Some(&token),
            "/v1/reports/account-planning",
            ""
        )
        .await
        .2["code"],
        "profile_zone_unconfirmed"
    );
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let default = send(
        router.clone(),
        Some(&token),
        "/v1/reports/account-planning",
        "",
    )
    .await;
    assert_eq!(default.0, StatusCode::OK);
    use chrono::{DateTime, Datelike, NaiveDate, Utc};
    let today = DateTime::<Utc>::from(std::time::SystemTime::now()).date_naive();
    let end = (1..=31)
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(today.year(), today.month(), day))
        .unwrap()
        .to_string();
    assert_eq!(default.2["through"], end);
    for uri in [
        format!("{explicit}&through=2024-02-01"),
        format!("{explicit}&user_id={owner}"),
        "/v1/reports/account-planning?through=2024-02-30".to_owned(),
        format!("/v1/accounts/{id}/balance-history"),
        format!("/v1/accounts/{id}/balance-history?date_from=2024-02-30&date_to=2024-03-01"),
        format!("{explicit}&x={}", "a".repeat(1024)),
    ] {
        assert_eq!(
            send(router.clone(), Some(&token), &uri, "").await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        send(router, Some(&token), explicit, "{}").await.0,
        StatusCode::BAD_REQUEST
    );
}

async fn all_tables_snapshot(pool: &PgPool) -> Vec<(String, String)> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename::text FROM pg_tables WHERE schemaname='public' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut result = Vec::new();
    for table in tables {
        let query = format!(
            "SELECT md5(coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]')::text) FROM \"{table}\" t"
        );
        result.push((
            table,
            sqlx::query_scalar(&query).fetch_one(pool).await.unwrap(),
        ));
    }
    result
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn paused_backlog_is_incomplete_until_excluded_and_slot_budget_fails_closed() {
    use chrono::{DateTime, Days, Utc};
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let today = DateTime::<Utc>::from(std::time::SystemTime::now()).date_naive();
    let start = today.checked_sub_days(Days::new(14)).unwrap().to_string();
    let pause = today.checked_sub_days(Days::new(7)).unwrap().to_string();
    let id = rule(&pool, &owner, None, 500, &start, true).await;
    sqlx::query("UPDATE recurring_rules SET paused_on=$2::date WHERE id=$1::uuid")
        .bind(&id)
        .bind(&pause)
        .execute(&pool)
        .await
        .unwrap();
    let router = app(Some(pool.clone()));
    let uri = format!("/v1/reports/account-planning?through={today}");
    let before = all_tables_snapshot(&pool).await;
    let report = send(router.clone(), Some(&token), &uri, "").await;
    assert_eq!(report.0, StatusCode::OK);
    assert_eq!(report.2["unassigned"][0]["complete"], false);
    assert_eq!(report.2["unassigned"][0]["projected_total"], "0.00");
    assert_eq!(before, all_tables_snapshot(&pool).await);
    sqlx::query("UPDATE recurring_rules SET excluded_dates=datemultirange(daterange($2::date,$3::date,'[)')) WHERE id=$1::uuid")
        .bind(&id).bind(&start).bind(&pause).execute(&pool).await.unwrap();
    assert_eq!(
        send(router.clone(), Some(&token), &uri, "").await.2["complete"],
        true
    );
    let (large, large_token) = seed(&pool, true).await;
    rule(&pool, &large, None, 500, "0001-01-01", false).await;
    let before = all_tables_snapshot(&pool).await;
    let rejected = send(
        router,
        Some(&large_token),
        "/v1/reports/account-planning?through=9999-12-31",
        "",
    )
    .await;
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(rejected.2["code"], "planning_too_complex");
    assert_eq!(before, all_tables_snapshot(&pool).await);
}

async fn mutate_probe(
    router: Router,
    token: &str,
    method: &str,
    path: &str,
    key: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("idempotency-key", key)
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn regression_corrected_linked_expense_reassignment_void_and_large_exact_sums() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let a = account(&pool, &owner, "Original", 100000, "2023-12-01").await;
    let b = account(&pool, &owner, "Replacement", 200000, "2023-12-01").await;
    let router = app(Some(pool.clone()));
    let created=mutate_probe(router.clone(),&token,"POST","/v1/transactions","probe-create",json!({"kind":"expense","amount":"100.00","currency":"SEK","account_id":a,"occurred_on":"2023-12-30"})).await;
    assert_eq!(created.0, StatusCode::CREATED, "{:?}", created);
    let id = created.1["id"].as_str().unwrap();
    let r = rule(&pool, &owner, Some(&a), 10000, "2023-12-30", true).await;
    sqlx::query("INSERT INTO recurring_occurrences(user_id,rule_id,account_id,amount_minor,currency,scheduled_on,due_on,state,transaction_id) VALUES($1::uuid,$2::uuid,$3::uuid,10000,'SEK','2023-12-30','2023-12-30','confirmed',$4::uuid)").bind(&owner).bind(r).bind(&a).bind(id).execute(&pool).await.unwrap();
    let edited = mutate_probe(
        router.clone(),
        &token,
        "PATCH",
        &format!("/v1/transactions/{id}"),
        "probe-edit",
        json!({"account_id":b,"occurred_on":"2024-01-02","amount":"150.00"}),
    )
    .await;
    assert_eq!(edited.0, StatusCode::OK, "{:?}", edited);
    for (account, opening, days) in [
        (&a, "1000.00", ["1000.00", "1000.00", "1000.00", "1000.00"]),
        (&b, "2000.00", ["2000.00", "2000.00", "1850.00", "1850.00"]),
    ] {
        let before = all_tables_snapshot(&pool).await;
        let reply = send(
            router.clone(),
            Some(&token),
            &format!(
                "/v1/accounts/{account}/balance-history?date_from=2023-12-31&date_to=2024-01-03"
            ),
            "",
        )
        .await;
        assert_eq!(reply.0, StatusCode::OK, "{}", reply.2);
        assert_eq!(reply.2["opening_balance"], opening);
        assert_eq!(
            reply.2["days"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["balance"].as_str().unwrap())
                .collect::<Vec<_>>(),
            days
        );
        assert_eq!(before, all_tables_snapshot(&pool).await);
    }
    let voided = mutate_probe(
        router.clone(),
        &token,
        "POST",
        &format!("/v1/transactions/{id}/void"),
        "probe-void",
        json!({}),
    )
    .await;
    assert_eq!(voided.0, StatusCode::OK, "{:?}", voided);
    let before = all_tables_snapshot(&pool).await;
    let history = send(
        router.clone(),
        Some(&token),
        &format!("/v1/accounts/{b}/balance-history?date_from=2024-01-02&date_to=2024-01-03"),
        "",
    )
    .await;
    assert_eq!(history.2["days"][0]["balance"], "2000.00");
    assert_eq!(history.2["days"][1]["balance"], "2000.00");
    assert_eq!(before, all_tables_snapshot(&pool).await);
    let huge = account(&pool, &owner, "Huge", i64::MAX, "2024-01-01").await;
    for _ in 0..2 {
        sqlx::query("WITH t AS (INSERT INTO transactions(user_id,kind,account_id,amount_minor,currency,occurred_on) VALUES($1::uuid,'income',$2::uuid,9223372036854775807,'SEK','2024-01-02') RETURNING id) INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on) SELECT $1::uuid,id,$2::uuid,9223372036854775807,'SEK','2024-01-02' FROM t").bind(&owner).bind(&huge).execute(&pool).await.unwrap();
    }
    let before = all_tables_snapshot(&pool).await;
    let history = send(
        router.clone(),
        Some(&token),
        &format!("/v1/accounts/{huge}/balance-history?date_from=2024-01-01&date_to=2024-01-03"),
        "",
    )
    .await;
    assert_eq!(history.2["days"][1]["balance"], "276701161105643274.21");
    let planning = send(
        router,
        Some(&token),
        "/v1/reports/account-planning?through=2024-01-03",
        "",
    )
    .await;
    assert_eq!(planning.0, StatusCode::OK, "{}", planning.2);
    let huge_row = planning.2["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["account_id"] == huge)
        .unwrap();
    assert_eq!(huge_row["current_balance"], "276701161105643274.21");
    assert_eq!(huge_row["planning_difference"], "276701161105643274.21");
    assert_eq!(before, all_tables_snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn regression_cadences_anchor_exclusions_end_archived_and_currency_projection() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let a = account(&pool, &owner, "Archived", 100000, "2024-01-01").await;
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&a)
        .execute(&pool)
        .await
        .unwrap();
    // 2096 is a leap year; each rule has an inclusive end and original anchor.
    for (cadence, anchor, end, amount) in [
        ("monthly", "2096-01-31", "2096-03-31", 100),
        ("quarterly", "2096-01-31", "2096-07-31", 200),
        ("annual", "2096-02-29", "2100-02-28", 300),
        ("weekly", "2096-02-22", "2096-03-07", 400),
    ] {
        let r = rule(&pool, &owner, Some(&a), amount, anchor, false).await;
        sqlx::query("UPDATE recurring_rules SET cadence=$2,ends_on=$3::date WHERE id=$1::uuid")
            .bind(&r)
            .bind(cadence)
            .bind(end)
            .execute(&pool)
            .await
            .unwrap();
        if cadence == "monthly" {
            sqlx::query("UPDATE recurring_rules SET excluded_dates='{[2096-02-29,2096-03-01)}'::datemultirange WHERE id=$1::uuid").bind(&r).execute(&pool).await.unwrap();
        }
    }
    // Saved postponed slot must win even though current rule terms differ.
    let r = rule(&pool, &owner, None, 700, "2096-01-01", false).await;
    sqlx::query("UPDATE recurring_rules SET currency='EUR',ends_on='2096-01-01' WHERE id=$1::uuid")
        .bind(&r)
        .execute(&pool)
        .await
        .unwrap();
    occurrence(
        &pool,
        &owner,
        &r,
        None,
        900,
        ("2096-01-01", "2101-01-01"),
        "planned",
    )
    .await;
    let usd = rule(&pool, &owner, None, 1100, "2096-01-01", false).await;
    sqlx::query("UPDATE recurring_rules SET currency='USD',ends_on='2096-01-01',time_zone='Pacific/Kiritimati' WHERE id=$1::uuid").bind(usd).execute(&pool).await.unwrap();
    let before = all_tables_snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let result = send(
        router.clone(),
        Some(&token),
        "/v1/reports/account-planning?through=2100-02-28",
        "",
    )
    .await;
    assert_eq!(result.0, StatusCode::OK, "{}", result.2);
    let row = &result.2["accounts"][0];
    assert_eq!(row["status"], "archived");
    // monthly 2 + quarterly 3*2 + annual 5*3 + weekly 3*4 = 35.
    assert_eq!(row["projected_total"], "35.00");
    assert_eq!(row["planning_difference"], "965.00");
    assert_eq!(result.2["unassigned"][0]["pending_total"], "0.00");
    assert_eq!(result.2["unassigned"][1]["projected_total"], "0.00");
    assert_eq!(result.2["unassigned"][2]["projected_total"], "11.00");
    assert_eq!(result.2["complete"], true);
    let rejected = send(
        router,
        Some(&token),
        "/v1/reports/account-planning?through=2100-02-28&through=2100-03-01",
        "",
    )
    .await;
    assert_eq!(rejected.0, StatusCode::BAD_REQUEST);
    assert_eq!(before, all_tables_snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn regression_rule_local_today_and_locked_reads_fail_without_mutation() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let a = account(&pool, &owner, "Ahead", 10000, "2024-01-01").await;
    let b = account(&pool, &owner, "Behind", 10000, "2024-01-01").await;
    let day: String=sqlx::query_scalar("SELECT to_char((statement_timestamp() AT TIME ZONE 'Pacific/Kiritimati')::date,'YYYY-MM-DD')").fetch_one(&pool).await.unwrap();
    for (account, zone) in [(&a, "Pacific/Kiritimati"), (&b, "Pacific/Honolulu")] {
        let r = rule(&pool, &owner, Some(account), 2500, &day, false).await;
        sqlx::query("UPDATE recurring_rules SET time_zone=$2,ends_on=$3::date WHERE id=$1::uuid")
            .bind(r)
            .bind(zone)
            .bind(&day)
            .execute(&pool)
            .await
            .unwrap();
    }
    let router = app(Some(pool.clone()));
    let before = all_tables_snapshot(&pool).await;
    let report = send(
        router.clone(),
        Some(&token),
        &format!("/v1/reports/account-planning?through={day}"),
        "",
    )
    .await;
    assert_eq!(report.0, StatusCode::OK, "{}", report.2);
    for (id, complete, projected) in [(&a, false, "0.00"), (&b, true, "25.00")] {
        let row = report.2["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["account_id"] == *id)
            .unwrap();
        assert_eq!(row["complete"], complete, "{row}");
        assert_eq!(row["projected_total"], projected, "{row}");
        if !complete {
            assert!(row["planning_difference"].is_null());
        }
    }
    assert_eq!(before, all_tables_snapshot(&pool).await);
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE account_movements IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    for path in [
        format!("/v1/accounts/{a}/balance-history?date_from=2024-01-01&date_to=2024-01-01"),
        format!("/v1/reports/account-planning?through={day}"),
    ] {
        let failure = send(router.clone(), Some(&token), &path, "").await;
        assert_eq!(failure.0, StatusCode::SERVICE_UNAVAILABLE, "{}", failure.2);
        assert_eq!(failure.2["code"], "account_report_unavailable");
        assert!(failure.2.get("accounts").is_none());
        assert!(failure.2.get("days").is_none());
    }
    lock.rollback().await.unwrap();
    assert_eq!(before, all_tables_snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn empty_owner_planning_returns_complete_zero_currency_rows_without_writes() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let before = all_tables_snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let (status, _, body) = send(
        router,
        Some(&token),
        "/v1/reports/account-planning?through=2024-02-29",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["through"], "2024-02-29");
    assert_eq!(body["complete"], true);
    assert_eq!(body["accounts"], json!([]));
    assert_eq!(
        body["unassigned"],
        json!([
            {"currency":"SEK","pending_total":"0.00","projected_total":"0.00","complete":true},
            {"currency":"EUR","pending_total":"0.00","projected_total":"0.00","complete":true},
            {"currency":"USD","pending_total":"0.00","projected_total":"0.00","complete":true}
        ])
    );
    assert!(chrono::DateTime::parse_from_rfc3339(body["as_of"].as_str().unwrap()).is_ok());
    assert_eq!(before, all_tables_snapshot(&pool).await);
}
