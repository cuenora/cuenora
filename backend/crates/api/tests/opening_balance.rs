//! Opening posting acceptance through the public API, with disposable data only.
mod common;
#[path = "common/http.rs"]
mod http;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use chrono::Utc;
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn send(
    router: Router,
    token: Option<&str>,
    key: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut req = Request::post("/v1/accounts").header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(key) = key {
        req = req.header("idempotency-key", key);
    }
    let reply = router
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = reply.status();
    let headers = reply.headers().clone();
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(reply.into_body(), 16384).await.unwrap();
    let body = http::checked_body(&headers, String::from_utf8(bytes.to_vec()).unwrap());
    (status, serde_json::from_str(&body).unwrap())
}
fn input(amount: &str, date: &str, kind: &str, currency: &str) -> Value {
    json!({"name":"Opening","account_type":kind,"currency":currency,
           "opening_balance":{"amount":amount,"occurred_on":date}})
}
async fn seed(pool: &PgPool, verified: bool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let owner:String=sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at) VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END) RETURNING id::text").bind(email).bind(verified).fetch_one(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&owner)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at) VALUES($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')").bind(session).bind(&owner).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    (owner, tokens.access.expose_secret().to_owned())
}
async fn confirm(router: Router, token: &str, zone: &str) {
    let reply = router
        .oneshot(
            Request::patch("/v1/me")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"time_zone":zone}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reply.status(), StatusCode::OK);
}
async fn snapshot(pool: &PgPool) -> String {
    let financial:String=sqlx::query_scalar("SELECT jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM accounts a),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),(SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id),'[]') FROM account_movements m),(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM transaction_audit a),(SELECT coalesce(jsonb_agg(to_jsonb(k) ORDER BY account_id),'[]') FROM account_creation_keys k))::text").fetch_one(pool).await.unwrap();
    format!("{}{}", common::snapshot(pool).await, financial)
}
fn error(reply: &(StatusCode, Value), status: StatusCode, code: &str) {
    assert_eq!(reply.0, status);
    assert_eq!(reply.1["code"], code);
    assert_eq!(reply.1["details"], Value::Null);
    assert_eq!(reply.1["request_id"], "<request-id>");
    if code == "invalid_request"
        || code == "email_not_verified"
        || code == "invalid_access_token"
        || code == "account_creation_unavailable"
        || code == "idempotency_conflict"
    {
        assert_eq!(
            reply.1,
            serde_json::from_str::<Value>(&http::error_body(code)).unwrap()
        );
    }
}

#[tokio::test]
async fn opening_without_identity_never_reaches_financial_validation() {
    error(
        &send(
            app(None),
            None,
            Some("absent"),
            input("10.00", "2024-01-01", "normal", "SEK"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn nonzero_openings_are_owned_signed_special_events_with_minimal_audit() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    confirm(router.clone(), &token, "Europe/Stockholm").await;
    for kind in ["normal", "savings"] {
        for currency in ["SEK", "EUR", "USD"] {
            for (amount, minor) in [("12.30", 1230_i64), ("-9.45", -945)] {
                let reply = send(
                    router.clone(),
                    Some(&token),
                    Some(&format!("{kind}-{currency}-{minor}")),
                    input(amount, "2024-02-29", kind, currency),
                )
                .await;
                assert_eq!(reply.0, StatusCode::CREATED);
                assert_eq!(reply.1.as_object().unwrap().len(), 5);
                assert_eq!(reply.1["account_type"], kind);
                assert_eq!(reply.1["currency"], currency);
                let id = reply.1["id"].as_str().unwrap();
                let event:(String,String,i64,String,String,bool)=sqlx::query_as("SELECT user_id::text,kind,amount_minor,currency,to_char(occurred_on,'YYYY-MM-DD'),category_id IS NULL AND destination_account_id IS NULL AND description IS NULL AND state='posted' FROM transactions WHERE account_id=$1::uuid").bind(id).fetch_one(&pool).await.unwrap();
                assert_eq!(
                    event,
                    (
                        owner.clone(),
                        "opening_balance".into(),
                        minor,
                        currency.into(),
                        "2024-02-29".into(),
                        true
                    )
                );
                let movement:(String,i64,String,String,bool)=sqlx::query_as("SELECT m.user_id::text,m.signed_amount_minor,m.currency,to_char(m.occurred_on,'YYYY-MM-DD'),m.transaction_id=t.id FROM account_movements m JOIN transactions t ON t.account_id=m.account_id WHERE m.account_id=$1::uuid").bind(id).fetch_one(&pool).await.unwrap();
                assert_eq!(
                    movement,
                    (
                        owner.clone(),
                        minor,
                        currency.into(),
                        "2024-02-29".into(),
                        true
                    )
                );
                let audit:(String,String,Vec<String>)=sqlx::query_as("SELECT a.user_id::text,a.action,a.changed_fields FROM transaction_audit a JOIN transactions t ON t.id=a.transaction_id WHERE t.account_id=$1::uuid AND a.actor_id=a.user_id").bind(id).fetch_one(&pool).await.unwrap();
                assert_eq!(audit, (owner.clone(), "create".into(), vec![]));
            }
        }
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn zero_and_absent_openings_have_no_financial_postings() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let mut plain = input("0", "2024-01-01", "normal", "SEK");
    plain.as_object_mut().unwrap().remove("opening_balance");
    assert_eq!(
        send(router.clone(), Some(&token), Some("absent"), plain)
            .await
            .0,
        StatusCode::CREATED
    );
    confirm(router.clone(), &token, "Etc/UTC").await;
    for amount in ["0", "-0.00", "00.00"] {
        assert_eq!(
            send(
                router.clone(),
                Some(&token),
                Some(amount.replace('.', "_").as_str()),
                input(amount, "2024-01-01", "savings", "USD")
            )
            .await
            .0,
            StatusCode::CREATED
        );
    }
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM account_movements WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid)").bind(owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn invalid_dates_money_zones_and_authority_reject_without_writes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (_, pending) = seed(&pool, false).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    error(
        &send(
            router.clone(),
            Some(&token),
            Some("no-zone"),
            input("10", "2024-01-01", "normal", "SEK"),
        )
        .await,
        StatusCode::CONFLICT,
        "profile_zone_unconfirmed",
    );
    error(
        &send(
            router.clone(),
            Some(&pending),
            Some("pending"),
            input("10", "2024-01-01", "normal", "SEK"),
        )
        .await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
    confirm(router.clone(), &token, "Etc/UTC").await;
    let before = snapshot(&pool).await;
    for (date, code) in [
        ("2026-02-29", "invalid_business_date"),
        ("2026-2-01", "invalid_business_date"),
        ("2026-01-01T00:00:00Z", "invalid_business_date"),
        (" 2026-01-01", "invalid_business_date"),
        ("0000-01-01", "date_out_of_range"),
        ("10000-01-01", "date_out_of_range"),
        ("9999-12-31", "future_business_date"),
    ] {
        error(
            &send(
                router.clone(),
                Some(&token),
                Some("bad-date"),
                input("10", date, "normal", "SEK"),
            )
            .await,
            StatusCode::BAD_REQUEST,
            code,
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for amount in ["1.000", "NaN", "1e2", "+1", "92233720368547758.08"] {
        error(
            &send(
                router.clone(),
                Some(&token),
                Some("bad-money"),
                input(amount, "2024-01-01", "normal", "SEK"),
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for value in [
        Value::Null,
        json!({"amount":"10"}),
        json!({"amount":"10","occurred_on":"2024-01-01","currency":"EUR"}),
    ] {
        let mut body = input("10", "2024-01-01", "normal", "SEK");
        body["opening_balance"] = value;
        error(
            &send(router.clone(), Some(&token), Some("bad-shape"), body).await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let mut forged = input("10", "2024-01-01", "normal", "SEK");
    forged["user_id"] = json!(owner);
    error(
        &send(router.clone(), Some(&token), Some("forged"), forged).await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET profile_time_zone='Not/AZone' WHERE id=$1::uuid")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    error(
        &send(
            router,
            Some(&token),
            Some("unsupported"),
            input("10", "2024-01-01", "normal", "SEK"),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "unsupported_time_zone",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn replay_compares_opening_content_and_does_not_revalidate_historical_dates() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    confirm(router.clone(), &token, "Pacific/Kiritimati").await;
    let today = chrono::DateTime::<Utc>::from(std::time::SystemTime::now())
        .with_timezone(&chrono_tz::Pacific::Kiritimati)
        .format("%Y-%m-%d")
        .to_string();
    let body = input("10.00", &today, "normal", "SEK");
    let first = send(router.clone(), Some(&token), Some("original"), body.clone()).await;
    assert_eq!(first.0, StatusCode::CREATED);
    confirm(router.clone(), &token, "Etc/GMT+12").await;
    let before = snapshot(&pool).await;
    assert_eq!(
        send(router.clone(), Some(&token), Some("original"), body.clone()).await,
        first
    );
    let mut normalized = body.clone();
    normalized["opening_balance"]["amount"] = json!("0010.0");
    assert_eq!(
        send(router.clone(), Some(&token), Some("original"), normalized).await,
        first
    );
    error(
        &send(router.clone(), Some(&token), Some("new-key"), body.clone()).await,
        StatusCode::BAD_REQUEST,
        "future_business_date",
    );
    for change in [
        input("11.00", &today, "normal", "SEK"),
        input("10.00", "2024-01-01", "normal", "SEK"),
    ] {
        error(
            &send(router.clone(), Some(&token), Some("original"), change).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
    }
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_opening_retries_and_profile_updates_do_not_duplicate_postings() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    confirm(router.clone(), &token, "Etc/UTC").await;
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let router = router.clone();
        let token = token.clone();
        tasks.push(tokio::spawn(async move {
            send(
                router,
                Some(&token),
                Some("parallel"),
                input("-5.50", "2024-02-29", "savings", "EUR"),
            )
            .await
        }));
    }
    confirm(router.clone(), &token, "Europe/Stockholm").await;
    let first = tasks.remove(0).await.unwrap();
    assert_eq!(first.0, StatusCode::CREATED);
    for task in tasks {
        assert_eq!(task.await.unwrap(), first);
    }
    let counts:(i64,i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM accounts WHERE user_id=$1::uuid),(SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM account_movements WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid),(SELECT count(*) FROM account_creation_keys WHERE user_id=$1::uuid)").bind(owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (1, 1, 1, 1, 1));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn each_late_write_failure_rolls_back_the_complete_opening_operation() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    confirm(router.clone(), &token, "Etc/UTC").await;
    for table in [
        "transactions",
        "account_movements",
        "transaction_audit",
        "account_creation_keys",
    ] {
        let before = snapshot(&pool).await;
        sqlx::raw_sql(&format!("CREATE FUNCTION reject_opening_write() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END $$; CREATE TRIGGER reject_opening_write BEFORE INSERT ON {table} FOR EACH ROW EXECUTE FUNCTION reject_opening_write();")).execute(&pool).await.unwrap();
        let reply = send(
            router.clone(),
            Some(&token),
            Some(table),
            input("2.50", "2024-01-01", "normal", "SEK"),
        )
        .await;
        sqlx::raw_sql(&format!(
            "DROP TRIGGER reject_opening_write ON {table}; DROP FUNCTION reject_opening_write();"
        ))
        .execute(&pool)
        .await
        .unwrap();
        error(
            &reply,
            StatusCode::SERVICE_UNAVAILABLE,
            "account_creation_unavailable",
        );
        assert_eq!(snapshot(&pool).await, before);
        assert_eq!(
            send(
                router.clone(),
                Some(&token),
                Some(table),
                input("2.50", "2024-01-01", "normal", "SEK")
            )
            .await
            .0,
            StatusCode::CREATED
        );
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn legacy_creation_keys_replay_without_zone_and_conflict_with_added_opening() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let id:String=sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Legacy','normal','SEK') RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    use sha2::{Digest, Sha256};
    let key = Sha256::digest(b"legacy");
    sqlx::query("INSERT INTO account_creation_keys(user_id,key_hash,account_id,name,account_type,currency) VALUES($1::uuid,$2,$3::uuid,'Legacy','normal','SEK')").bind(&owner).bind(key.as_slice()).bind(&id).execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    let plain = json!({"name":"Legacy","account_type":"normal","currency":"SEK"});
    let reply = send(router.clone(), Some(&token), Some("legacy"), plain).await;
    assert_eq!(reply.0, StatusCode::CREATED);
    assert_eq!(reply.1["id"], id);
    let mut changed = input("1", "2024-01-01", "normal", "SEK");
    changed["name"] = json!("Legacy");
    error(
        &send(router, Some(&token), Some("legacy"), changed).await,
        StatusCode::CONFLICT,
        "idempotency_conflict",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn zero_requires_zone_and_date_and_replay_rechecks_email() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    error(
        &send(
            router.clone(),
            Some(&token),
            Some("zero"),
            input("0", "2024-01-01", "normal", "SEK"),
        )
        .await,
        StatusCode::CONFLICT,
        "profile_zone_unconfirmed",
    );
    assert_eq!(snapshot(&pool).await, before);
    confirm(router.clone(), &token, "Etc/UTC").await;
    let before = snapshot(&pool).await;
    error(
        &send(
            router.clone(),
            Some(&token),
            Some("zero"),
            input("0", "9999-12-31", "normal", "SEK"),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "future_business_date",
    );
    assert_eq!(snapshot(&pool).await, before);
    let first = send(
        router.clone(),
        Some(&token),
        Some("zero"),
        input("0.00", "2024-01-01", "normal", "SEK"),
    )
    .await;
    assert_eq!(first.0, StatusCode::CREATED);
    let before = snapshot(&pool).await;
    assert_eq!(
        send(
            router.clone(),
            Some(&token),
            Some("zero"),
            input("-0", "2024-01-01", "normal", "SEK")
        )
        .await,
        first
    );
    let plain = json!({"name":"Opening","account_type":"normal","currency":"SEK"});
    error(
        &send(router.clone(), Some(&token), Some("zero"), plain).await,
        StatusCode::CONFLICT,
        "idempotency_conflict",
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    error(
        &send(
            router,
            Some(&token),
            Some("zero"),
            input("0", "2024-01-01", "normal", "SEK"),
        )
        .await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
}
