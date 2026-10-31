//! P06.02/P06.03/P06.07 public transfer acceptance and permanent retry tests.
//! Only identity/account fixtures use SQL; transfer effects must come from HTTP.
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

async fn seed(pool: &PgPool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let owner: String = sqlx::query_scalar(
        "INSERT INTO users(email_original,email_address,email_verified_at,profile_time_zone)
         VALUES($1,$1,statement_timestamp(),'Etc/UTC') RETURNING id::text",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&owner)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at)
         VALUES($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')",
    )
    .bind(session)
    .bind(&owner)
    .bind(tokens.access.digest().as_slice())
    .execute(pool)
    .await
    .unwrap();
    (owner, tokens.access.expose_secret().to_owned())
}

async fn account(pool: &PgPool, owner: &str, currency: &str, kind: &str) -> String {
    sqlx::query_scalar(
        "INSERT INTO accounts(user_id,name,account_type,currency)
         VALUES($1::uuid,'Synthetic transfer account',$2,$3) RETURNING id::text",
    )
    .bind(owner)
    .bind(kind)
    .bind(currency)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn request(
    router: Router,
    token: &str,
    method: &str,
    path: &str,
    input: Value,
) -> (StatusCode, Value) {
    request_key(router, token, method, path, input, "synthetic-transfer").await
}

async fn request_key(
    router: Router,
    token: &str,
    method: &str,
    path: &str,
    input: Value,
    key: &str,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let body = if method == "GET" {
        Body::empty()
    } else {
        builder = builder
            .header(header::CONTENT_TYPE, "application/json")
            .header("idempotency-key", key);
        Body::from(input.to_string())
    };
    let response = router.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 256 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn assert_transfer(source_currency: &str, destination_currency: &str) {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let source = account(&pool, &owner, source_currency, "normal").await;
    let destination = account(&pool, &owner, destination_currency, "savings").await;
    let same_currency = source_currency == destination_currency;
    let (received, received_minor) = if same_currency {
        ("200.00", 20000_i64)
    } else {
        ("17.95", 1795_i64)
    };
    let identity_before = common::snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let (status, body) = request(
        router.clone(),
        &token,
        "POST",
        "/v1/transactions",
        json!({
            "kind":"transfer", "amount":"200.00", "currency":source_currency,
            "account_id":source, "destination_account_id":destination,
            "received_amount":received, "received_currency":destination_currency,
            "occurred_on":"2024-02-29"
        }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "transfer creation must succeed: {body}"
    );
    assert_eq!(common::snapshot(&pool).await, identity_before);
    let event = body["id"].as_str().expect("created transfer id");
    for (field, expected) in [
        ("kind", "transfer"),
        ("state", "posted"),
        ("amount", "200.00"),
        ("currency", source_currency),
        ("account_id", source.as_str()),
        ("destination_account_id", destination.as_str()),
        ("received_amount", received),
        ("received_currency", destination_currency),
        ("occurred_on", "2024-02-29"),
    ] {
        assert_eq!(body[field], expected, "response field {field}");
    }
    let stored: (
        String,
        String,
        String,
        String,
        i64,
        String,
        i64,
        String,
        String,
    ) = sqlx::query_as(
        "SELECT user_id::text,kind,state,account_id::text,amount_minor,currency,
         received_amount_minor,received_currency,destination_account_id::text
         FROM transactions WHERE id=$1::uuid AND occurred_on=DATE '2024-02-29'",
    )
    .bind(event)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored,
        (
            owner.clone(),
            "transfer".into(),
            "posted".into(),
            source.clone(),
            20000,
            source_currency.into(),
            received_minor,
            destination_currency.into(),
            destination.clone()
        )
    );
    let events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM transactions WHERE user_id=$1::uuid")
            .bind(&owner)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        events, 1,
        "one business event, not separate income/expense events"
    );
    let movements: Vec<(String, String, String, i64, String)> = sqlx::query_as(
        "SELECT user_id::text,transaction_id::text,account_id::text,signed_amount_minor,currency
         FROM account_movements WHERE user_id=$1::uuid ORDER BY signed_amount_minor",
    )
    .bind(&owner)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        movements,
        vec![
            (
                owner.clone(),
                event.into(),
                source.clone(),
                -20000,
                source_currency.into()
            ),
            (
                owner.clone(),
                event.into(),
                destination.clone(),
                received_minor,
                destination_currency.into()
            ),
        ],
        "exactly one source debit and one destination credit with their own currencies"
    );
    let dated: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM account_movements WHERE user_id=$1::uuid AND occurred_on=DATE '2024-02-29'",
    ).bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(dated, 2);
    let (status, listed) = request(router, &token, "GET", "/v1/accounts", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    for (id, balance, currency) in [
        (&source, "-200.00", source_currency),
        (&destination, received, destination_currency),
    ] {
        let account = listed["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|account| account["id"] == *id)
            .unwrap();
        assert_eq!(account["balance"], balance);
        assert_eq!(account["currency"], currency);
    }
}

macro_rules! transfer_case {
    ($name:ident, $source:literal, $destination:literal) => {
        #[tokio::test]
        #[ignore = "disposable PostgreSQL harness only"]
        async fn $name() {
            assert_transfer($source, $destination).await;
        }
    };
}

transfer_case!(sek_to_sek, "SEK", "SEK");
transfer_case!(eur_to_eur, "EUR", "EUR");
transfer_case!(usd_to_usd, "USD", "USD");
transfer_case!(sek_to_eur, "SEK", "EUR");
transfer_case!(sek_to_usd, "SEK", "USD");
transfer_case!(eur_to_sek, "EUR", "SEK");
transfer_case!(eur_to_usd, "EUR", "USD");
transfer_case!(usd_to_sek, "USD", "SEK");
transfer_case!(usd_to_eur, "USD", "EUR");

async fn fixture() -> (PgPool, String, String, Value) {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let source = account(&pool, &owner, "SEK", "normal").await;
    let destination = account(&pool, &owner, "EUR", "savings").await;
    let input = json!({"kind":"transfer", "amount":"200.00", "currency":"SEK",
        "account_id":source, "destination_account_id":destination,
        "received_amount":"17.95", "received_currency":"EUR", "occurred_on":"2024-02-29"});
    (pool, owner, token, input)
}

async fn financial_snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar(
        "SELECT md5(jsonb_build_array(
        (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM transactions t),
        (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM account_movements m),
        (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM transaction_audit a),
        (SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM transfer_creation_keys k),
        (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM accounts a),
        (SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM categories c),
        (SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM account_creation_keys k),
        (SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM income_creation_keys k),
        (SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM expense_creation_keys k),
        (SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM transaction_mutation_keys k)
    )::text)",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn retries_are_canonical_permanent_and_independent_of_income_expense() {
    let (pool, owner, token, input) = fixture().await;
    let router = app(Some(pool.clone()));
    let first = request(
        router.clone(),
        &token,
        "POST",
        "/v1/transactions",
        input.clone(),
    )
    .await;
    assert_eq!(first.0, StatusCode::CREATED, "{}", first.1);
    let before = financial_snapshot(&pool).await;
    let mut canonical = input.clone();
    canonical["description"] = Value::Null;
    canonical["amount"] = json!("0200.0");
    canonical["received_amount"] = json!("017.95");
    canonical["account_id"] = json!(input["account_id"].as_str().unwrap().to_uppercase());
    assert_eq!(
        request(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            canonical
        )
        .await,
        first
    );
    assert_eq!(financial_snapshot(&pool).await, before);
    for field in ["amount", "received_amount", "occurred_on", "description"] {
        let mut changed = input.clone();
        changed[field] = json!(match field {
            "amount" => "201.00",
            "received_amount" => "18.00",
            "occurred_on" => "2024-02-28",
            _ => "Changed",
        });
        let result = request(router.clone(), &token, "POST", "/v1/transactions", changed).await;
        assert_eq!(result.0, StatusCode::CONFLICT);
        assert_eq!(result.1["code"], "idempotency_conflict");
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    // The same opaque key is independent in the income and expense namespaces.
    for kind in ["income", "expense"] {
        let result = request(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            json!({"kind":kind,"amount":"1.00","currency":"SEK","occurred_on":"2024-02-29"}),
        )
        .await;
        assert_eq!(result.0, StatusCode::CREATED);
        assert_ne!(result.1["id"], first.1["id"]);
    }
    let path = format!("/v1/transactions/{}", first.1["id"].as_str().unwrap());
    let edited = request_key(
        router.clone(),
        &token,
        "PATCH",
        &path,
        json!({"amount":"201.00","received_amount":"18.00"}),
        "edit-transfer",
    )
    .await;
    assert_eq!(edited.0, StatusCode::OK, "{}", edited.1);
    let voided = request_key(
        router.clone(),
        &token,
        "POST",
        &format!("{path}/void"),
        json!({}),
        "void-transfer",
    )
    .await;
    assert_eq!(voided.0, StatusCode::OK, "{}", voided.1);
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE user_id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET profile_time_zone=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = financial_snapshot(&pool).await;
    assert_eq!(
        request(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            input.clone()
        )
        .await,
        first
    );
    assert_eq!(financial_snapshot(&pool).await, before);
    for statement in [
        "UPDATE transfer_creation_keys SET key_hash=key_hash",
        "DELETE FROM transfer_creation_keys",
        "TRUNCATE transfer_creation_keys",
    ] {
        let error = sqlx::query(statement).execute(&pool).await.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("55000")
        );
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    let (other, other_token) = seed(&pool).await;
    let source = account(&pool, &other, "SEK", "normal").await;
    let destination = account(&pool, &other, "EUR", "normal").await;
    let mut other_input = input;
    other_input["account_id"] = json!(source);
    other_input["destination_account_id"] = json!(destination);
    let other_reply = request(
        router,
        &other_token,
        "POST",
        "/v1/transactions",
        other_input,
    )
    .await;
    assert_eq!(other_reply.0, StatusCode::CREATED);
    assert_ne!(other_reply.1["id"], first.1["id"]);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_equal_keys_post_one_event_and_two_legs() {
    let (pool, owner, token, input) = fixture().await;
    let router = app(Some(pool.clone()));
    let (a, b) = tokio::join!(
        request(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            input.clone()
        ),
        request(router, &token, "POST", "/v1/transactions", input)
    );
    assert_eq!(a.0, StatusCode::CREATED, "{}", a.1);
    assert_eq!(a, b);
    let counts: (i64,i64,i64,i64) = sqlx::query_as("SELECT
        (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),
        (SELECT count(*) FROM account_movements WHERE user_id=$1::uuid),
        (SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid AND actor_id=user_id AND action='create' AND changed_fields=ARRAY[]::text[]),
        (SELECT count(*) FROM transfer_creation_keys WHERE user_id=$1::uuid)")
        .bind(owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (1, 2, 1, 1));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn late_audit_failure_rolls_back_all_effects_and_does_not_consume_key() {
    let (pool, owner, token, input) = fixture().await;
    let router = app(Some(pool.clone()));
    let before = financial_snapshot(&pool).await;
    sqlx::query(&format!("ALTER TABLE transaction_audit ADD CONSTRAINT transfer_test_failure CHECK (user_id <> '{owner}'::uuid)"))
        .execute(&pool).await.unwrap();
    let failed = request(
        router.clone(),
        &token,
        "POST",
        "/v1/transactions",
        input.clone(),
    )
    .await;
    sqlx::query("ALTER TABLE transaction_audit DROP CONSTRAINT transfer_test_failure")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(failed.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(failed.1["code"], "transfer_creation_unavailable");
    assert_eq!(financial_snapshot(&pool).await, before);
    let retry = request(router, &token, "POST", "/v1/transactions", input).await;
    assert_eq!(retry.0, StatusCode::CREATED, "{}", retry.1);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn transfer_fields_remain_invalid_for_income_and_expense() {
    let (pool, _owner, token, transfer) = fixture().await;
    let router = app(Some(pool.clone()));
    for kind in ["income", "expense"] {
        for field in [
            "destination_account_id",
            "received_amount",
            "received_currency",
        ] {
            let mut input =
                json!({"kind":kind,"amount":"1.00","currency":"SEK","occurred_on":"2024-02-29"});
            input[field] = transfer[field].clone();
            let result = request(router.clone(), &token, "POST", "/v1/transactions", input).await;
            assert_eq!(result.0, StatusCode::BAD_REQUEST);
            assert_eq!(result.1["code"], "invalid_request");
        }
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn invalid_inputs_and_authority_never_write_or_consume_keys() {
    let (pool, owner, token, input) = fixture().await;
    let router = app(Some(pool.clone()));
    let (other, _) = seed(&pool).await;
    let foreign = account(&pool, &other, "EUR", "normal").await;
    let before = financial_snapshot(&pool).await;
    let cases = [
        ("amount", json!("0"), StatusCode::BAD_REQUEST),
        ("received_amount", json!("-1"), StatusCode::BAD_REQUEST),
        ("received_amount", json!("17.950"), StatusCode::BAD_REQUEST),
        (
            "received_amount",
            json!("92233720368547758.08"),
            StatusCode::BAD_REQUEST,
        ),
        ("currency", json!("GBP"), StatusCode::BAD_REQUEST),
        ("currency", json!("EUR"), StatusCode::BAD_REQUEST),
        (
            "destination_account_id",
            input["account_id"].clone(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "destination_account_id",
            json!(foreign),
            StatusCode::NOT_FOUND,
        ),
        ("quoted_rate", json!("0.09"), StatusCode::BAD_REQUEST),
        ("user_id", json!(other), StatusCode::BAD_REQUEST),
        ("description", json!("bad\ntext"), StatusCode::BAD_REQUEST),
        ("occurred_on", json!("2024-02-30"), StatusCode::BAD_REQUEST),
        ("occurred_on", json!("9999-01-01"), StatusCode::BAD_REQUEST),
    ];
    for (field, value, status) in cases {
        let mut body = input.clone();
        body[field] = value;
        let result = request(router.clone(), &token, "POST", "/v1/transactions", body).await;
        assert_eq!(result.0, status, "field {field}: {}", result.1);
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    for (raw, keys, path) in [
        (input.to_string(), vec![], "/v1/transactions"),
        (input.to_string(), vec!["one", "two"], "/v1/transactions"),
        (
            format!("{{\"amount\":\"1.00\",{}", &input.to_string()[1..]),
            vec!["duplicate-json"],
            "/v1/transactions",
        ),
        (input.to_string(), vec!["query"], "/v1/transactions?extra=1"),
    ] {
        let mut builder = Request::post(path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json");
        for key in keys {
            builder = builder.header("idempotency-key", key);
        }
        let reply = router
            .clone()
            .oneshot(builder.body(Body::from(raw)).unwrap())
            .await
            .unwrap();
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    for key in ["", "contains space", &"x".repeat(129)] {
        let result = request_key(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            input.clone(),
            key,
        )
        .await;
        assert_eq!(result.0, StatusCode::BAD_REQUEST);
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    for (sql, code) in [
        (
            "UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid",
            "email_not_verified",
        ),
        (
            "UPDATE users SET email_verified_at=statement_timestamp(),profile_time_zone=NULL WHERE id=$1::uuid",
            "profile_zone_unconfirmed",
        ),
    ] {
        sqlx::query(sql).bind(&owner).execute(&pool).await.unwrap();
        let result = request(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            input.clone(),
        )
        .await;
        assert_eq!(
            result.0,
            if code == "email_not_verified" {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::CONFLICT
            }
        );
        assert_eq!(result.1["code"], code);
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    sqlx::query("UPDATE users SET profile_time_zone='Etc/UTC' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(input["destination_account_id"].as_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    // Fixture archival is intentional; compare against state immediately before HTTP.
    let before = financial_snapshot(&pool).await;
    let result = request(
        router.clone(),
        &token,
        "POST",
        "/v1/transactions",
        input.clone(),
    )
    .await;
    assert_eq!(result.0, StatusCode::BAD_REQUEST);
    assert_eq!(financial_snapshot(&pool).await, before);
    sqlx::query("UPDATE accounts SET archived_at=NULL WHERE id=$1::uuid")
        .bind(input["destination_account_id"].as_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let accepted = request(router, &token, "POST", "/v1/transactions", input).await;
    assert_eq!(accepted.0, StatusCode::CREATED, "{}", accepted.1);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn destination_and_retry_failures_roll_back_and_replay_requires_verification() {
    let (pool, owner, token, input) = fixture().await;
    let router = app(Some(pool.clone()));
    // Each constraint fires after the source debit or after both postings and audit.
    for (table, constraint) in [
        (
            "account_movements",
            format!(
                "CHECK (account_id <> '{}'::uuid)",
                input["destination_account_id"].as_str().unwrap()
            ),
        ),
        (
            "transfer_creation_keys",
            format!("CHECK (user_id <> '{owner}'::uuid)"),
        ),
    ] {
        let before = financial_snapshot(&pool).await;
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD CONSTRAINT transfer_regression_failure {constraint}"
        ))
        .execute(&pool)
        .await
        .unwrap();
        let r = request_key(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            input.clone(),
            "late-independent",
        )
        .await;
        sqlx::query(&format!(
            "ALTER TABLE {table} DROP CONSTRAINT transfer_regression_failure"
        ))
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(r.0, StatusCode::SERVICE_UNAVAILABLE, "{}", r.1);
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    let first = request_key(
        router.clone(),
        &token,
        "POST",
        "/v1/transactions",
        input.clone(),
        "late-independent",
    )
    .await;
    assert_eq!(first.0, StatusCode::CREATED);
    let second = request_key(
        router.clone(),
        &token,
        "POST",
        "/v1/transactions",
        input.clone(),
        "another-key",
    )
    .await;
    assert_eq!(second.0, StatusCode::CREATED);
    assert_ne!(first.1["id"], second.1["id"]);
    let (foreign, _) = seed(&pool).await;
    let other_source = account(&pool, &foreign, "SEK", "normal").await;
    let other_destination = account(&pool, &foreign, "EUR", "normal").await;
    for (field, value) in [
        ("account_id", other_source),
        ("destination_account_id", other_destination),
    ] {
        let before = financial_snapshot(&pool).await;
        let mut bad = input.clone();
        bad[field] = json!(value);
        let r = request_key(
            router.clone(),
            &token,
            "POST",
            "/v1/transactions",
            bad,
            "foreign-side",
        )
        .await;
        assert_eq!(r.0, StatusCode::NOT_FOUND);
        assert_eq!(financial_snapshot(&pool).await, before);
    }
    let before = financial_snapshot(&pool).await;
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let r = request_key(
        router,
        &token,
        "POST",
        "/v1/transactions",
        input,
        "late-independent",
    )
    .await;
    assert_eq!(r.0, StatusCode::FORBIDDEN);
    assert_eq!(r.1["code"], "email_not_verified");
    assert_eq!(financial_snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn ordinary_transport_preserves_verified_baseline_cases() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool).await;
    let router = app(Some(pool.clone()));
    for kind in ["income", "expense"] {
        let valid =
            json!({"kind":kind,"amount":"1.00","currency":"SEK","occurred_on":"2024-02-29"});
        let mut inputs = vec![valid.to_string()];
        for field in [
            "destination_account_id",
            "received_amount",
            "received_currency",
            "quoted_rate",
            "unexpected",
        ] {
            let mut body = valid.clone();
            body[field] = json!("synthetic");
            inputs.push(body.to_string());
        }
        for field in ["amount", "kind", "description", "account_id", "category_id"] {
            inputs.push(format!("{{\"{}\":null,{}", field, &valid.to_string()[1..]));
        }
        let mut bad = valid.clone();
        bad["amount"] = json!(1);
        inputs.push(bad.to_string());
        let mut bad = valid;
        bad["received_amount"] = Value::Null;
        inputs.push(bad.to_string());
        for (index, raw) in inputs.into_iter().enumerate() {
            let before = financial_snapshot(&pool).await;
            let reply = router
                .clone()
                .oneshot(
                    Request::post("/v1/transactions")
                        .header(header::AUTHORIZATION, format!("Bearer {token}"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header("idempotency-key", format!("baseline-{kind}-{index}"))
                        .body(Body::from(raw))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = reply.status();
            let bytes = to_bytes(reply.into_body(), 16384).await.unwrap();
            let mut body: Value = serde_json::from_slice(&bytes).unwrap();
            // Expected outputs were independently compared with the previous implementation.
            // No test depends on git history or an external output artifact.
            if matches!(index, 0 | 8 | 9 | 10) {
                assert_eq!(status, StatusCode::CREATED);
                assert!(body["id"].as_str().is_some());
                body["id"] = json!("event-id");
                assert_eq!(
                    body,
                    json!({"id":"event-id","kind":kind,"state":"posted",
                    "amount":"1.00","currency":"SEK","occurred_on":"2024-02-29",
                    "account_id":null,"category_id":null,"description":null})
                );
            } else {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert!(body["request_id"].as_str().is_some());
                body["request_id"] = json!("request-id");
                assert_eq!(
                    body,
                    json!({"code":"invalid_request","message":"The request is invalid.",
                    "details":null,"request_id":"request-id"})
                );
                assert_eq!(financial_snapshot(&pool).await, before);
            }
        }
    }
}
