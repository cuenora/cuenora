//! Atomic corrections through public HTTP, using only disposable synthetic data.
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
    method: &str,
    path: &str,
    key: Option<&str>,
    raw: &str,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(key) = key {
        request = request.header("idempotency-key", key);
    }
    let response = router
        .oneshot(request.body(Body::from(raw.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    let mut body: Value = serde_json::from_slice(&bytes).unwrap();
    if body.get("code").is_some() {
        assert_eq!(
            body["request_id"],
            headers["x-request-id"].to_str().unwrap()
        );
        assert_eq!(body["details"], Value::Null);
        body["request_id"] = json!("<request-id>");
    }
    (status, body)
}
async fn edit(
    pool: &PgPool,
    token: &str,
    id: &str,
    key: &str,
    patch: Value,
) -> (StatusCode, Value) {
    send(
        app(Some(pool.clone())),
        Some(token),
        "PATCH",
        &format!("/v1/transactions/{id}"),
        Some(key),
        &patch.to_string(),
    )
    .await
}
async fn void(pool: &PgPool, token: &str, id: &str, key: &str) -> (StatusCode, Value) {
    send(
        app(Some(pool.clone())),
        Some(token),
        "POST",
        &format!("/v1/transactions/{id}/void"),
        Some(key),
        "{}",
    )
    .await
}
fn ok(reply: (StatusCode, Value), id: &str, state: &str) {
    assert_eq!(reply, (StatusCode::OK, json!({"id":id,"state":state})));
}
fn error(reply: (StatusCode, Value), status: StatusCode, code: &str) {
    assert_eq!(reply.0, status, "{}", reply.1);
    assert_eq!(reply.1["code"], code);
}
async fn seed(pool: &PgPool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let user: String = sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at,profile_time_zone) VALUES($1,$1,statement_timestamp(),'Etc/UTC') RETURNING id::text").bind(email).fetch_one(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&user)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at) VALUES($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')").bind(session).bind(&user).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    (user, tokens.access.expose_secret().to_owned())
}
async fn account(pool: &PgPool, user: &str, kind: &str, currency: &str) -> String {
    sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Synthetic',$2,$3) RETURNING id::text").bind(user).bind(kind).bind(currency).fetch_one(pool).await.unwrap()
}
async fn event(
    pool: &PgPool,
    user: &str,
    kind: &str,
    account: Option<&str>,
    destination: Option<&str>,
) -> String {
    let id: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,occurred_on,amount_minor,currency,account_id,destination_account_id,received_amount_minor,received_currency) VALUES($1::uuid,$2,'2024-02-29',10000,'SEK',$3::uuid,$4::uuid,CASE WHEN $2='transfer' THEN 1800 END,CASE WHEN $2='transfer' THEN 'EUR' END) RETURNING id::text").bind(user).bind(kind).bind(account).bind(destination).fetch_one(pool).await.unwrap();
    if let Some(account) = account {
        let amount = if matches!(kind, "expense" | "transfer") {
            -10000_i64
        } else {
            10000
        };
        sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,$4,'SEK','2024-02-29')").bind(user).bind(&id).bind(account).bind(amount).execute(pool).await.unwrap();
    }
    if let Some(destination) = destination {
        sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,1800,'EUR','2024-02-29')").bind(user).bind(&id).bind(destination).execute(pool).await.unwrap();
    }
    id
}
async fn balance(pool: &PgPool, account: &str) -> i64 {
    sqlx::query_scalar("SELECT coalesce(sum(signed_amount_minor),0)::bigint FROM account_movements WHERE account_id=$1::uuid").bind(account).fetch_one(pool).await.unwrap()
}
async fn snapshot(pool: &PgPool) -> String {
    let financial: String = sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM transactions t),(SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM account_movements m),(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM transaction_audit a),(SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM transaction_mutation_keys k),(SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM income_creation_keys k),(SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM expense_creation_keys k),(SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash) FROM account_creation_keys k),(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM accounts a))::text)").fetch_one(pool).await.unwrap();
    format!("{}:{financial}", common::snapshot(pool).await)
}

#[tokio::test]
async fn mutations_authenticate_before_parsing() {
    for (method, path) in [
        ("PATCH", "/v1/transactions/not-a-uuid"),
        ("POST", "/v1/transactions/not-a-uuid/void"),
    ] {
        error(
            send(app(None), None, method, path, None, "{").await,
            StatusCode::UNAUTHORIZED,
            "invalid_access_token",
        );
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn edits_and_voids_all_five_kinds_preserving_original_postings() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    for kind in [
        "income",
        "expense",
        "opening_balance",
        "transfer",
        "interest_income",
    ] {
        let a = account(
            &pool,
            &owner,
            if kind == "income" {
                "normal"
            } else {
                "savings"
            },
            "SEK",
        )
        .await;
        let b = account(&pool, &owner, "normal", "EUR").await;
        let id = event(
            &pool,
            &owner,
            kind,
            Some(&a),
            if kind == "transfer" { Some(&b) } else { None },
        )
        .await;
        let original: String = sqlx::query_scalar("SELECT md5(jsonb_agg(to_jsonb(m) ORDER BY id)::text) FROM account_movements m WHERE transaction_id=$1::uuid").bind(&id).fetch_one(&pool).await.unwrap();
        let mut patch = json!({"amount":"125.50","occurred_on":"2024-03-01"});
        if kind == "transfer" {
            patch["received_amount"] = json!("20.05");
        }
        ok(edit(&pool, &token, &id, kind, patch).await, &id, "posted");
        let signed = if matches!(kind, "expense" | "transfer") {
            -12550
        } else {
            12550
        };
        assert_eq!(balance(&pool, &a).await, signed);
        assert_eq!(
            balance(&pool, &b).await,
            if kind == "transfer" { 2005 } else { 0 }
        );
        let originals: String = sqlx::query_scalar("SELECT md5(jsonb_agg(to_jsonb(m) ORDER BY id)::text) FROM account_movements m WHERE transaction_id=$1::uuid AND reverses_movement_id IS NULL AND occurred_on='2024-02-29'").bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(originals, original);
        let bad_links: i64 = sqlx::query_scalar("SELECT count(*) FROM account_movements r JOIN account_movements m ON m.id=r.reverses_movement_id WHERE r.transaction_id=$1::uuid AND (r.signed_amount_minor <> -m.signed_amount_minor OR r.currency<>m.currency OR r.account_id<>m.account_id OR r.occurred_on<>m.occurred_on)").bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(bad_links, 0);
        let fields: Vec<String> = sqlx::query_scalar("SELECT changed_fields FROM transaction_audit WHERE transaction_id=$1::uuid AND action='edit'").bind(&id).fetch_one(&pool).await.unwrap();
        assert!(fields.contains(&"amount_minor".into()) && fields.contains(&"occurred_on".into()));
        ok(
            void(&pool, &token, &id, &format!("void-{kind}")).await,
            &id,
            "voided",
        );
        assert_eq!(balance(&pool, &a).await, 0);
        assert_eq!(balance(&pool, &b).await, 0);
        let before = snapshot(&pool).await;
        ok(
            void(&pool, &token, &id, &format!("void-{kind}")).await,
            &id,
            "voided",
        );
        assert_eq!(snapshot(&pool).await, before);
        error(
            edit(
                &pool,
                &token,
                &id,
                &format!("after-{kind}"),
                json!({"description":"No restore"}),
            )
            .await,
            StatusCode::CONFLICT,
            "transaction_voided",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn accountless_link_unlink_and_savings_restrictions() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let normal = account(&pool, &owner, "normal", "SEK").await;
    let savings = account(&pool, &owner, "savings", "SEK").await;
    for kind in ["income", "expense"] {
        let id = event(&pool, &owner, kind, None, None).await;
        ok(
            edit(
                &pool,
                &token,
                &id,
                &format!("unlinked-{kind}"),
                json!({"amount":"25"}),
            )
            .await,
            &id,
            "posted",
        );
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM account_movements WHERE transaction_id=$1::uuid",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0);
        if kind == "income" {
            let before = snapshot(&pool).await;
            error(
                edit(
                    &pool,
                    &token,
                    &id,
                    "savings-reject",
                    json!({"account_id":savings}),
                )
                .await,
                StatusCode::BAD_REQUEST,
                "invalid_request",
            );
            assert_eq!(snapshot(&pool).await, before);
        }
        let target = if kind == "expense" { &savings } else { &normal };
        ok(
            edit(
                &pool,
                &token,
                &id,
                &format!("link-{kind}"),
                json!({"account_id":target}),
            )
            .await,
            &id,
            "posted",
        );
        assert_eq!(
            balance(&pool, target).await,
            if kind == "expense" { -2500 } else { 2500 }
        );
        if kind == "income" {
            for (key, patch) in [
                ("reassign-savings", json!({"account_id":savings})),
                (
                    "edit-and-reassign-savings",
                    json!({"account_id":savings,"amount":"75.50"}),
                ),
            ] {
                let before = snapshot(&pool).await;
                error(
                    edit(&pool, &token, &id, key, patch).await,
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                );
                assert_eq!(snapshot(&pool).await, before);
                assert_eq!(balance(&pool, &normal).await, 2500);
                assert_eq!(balance(&pool, &savings).await, 0);
            }
        }
        ok(
            edit(
                &pool,
                &token,
                &id,
                &format!("unlink-{kind}"),
                json!({"account_id":null}),
            )
            .await,
            &id,
            "posted",
        );
        assert_eq!(balance(&pool, target).await, 0);
        ok(
            void(&pool, &token, &id, &format!("void-{kind}")).await,
            &id,
            "voided",
        );
    }
    let opening = event(&pool, &owner, "opening_balance", Some(&savings), None).await;
    ok(
        edit(
            &pool,
            &token,
            &opening,
            "negative-opening",
            json!({"amount":"-40"}),
        )
        .await,
        &opening,
        "posted",
    );
    assert_eq!(balance(&pool, &savings).await, -4000);
    ok(
        edit(
            &pool,
            &token,
            &opening,
            "zero-opening",
            json!({"amount":"0"}),
        )
        .await,
        &opening,
        "posted",
    );
    assert_eq!(balance(&pool, &savings).await, 0);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn archived_accounts_allow_metadata_but_reject_movement_changes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let a = account(&pool, &owner, "normal", "SEK").await;
    let b = account(&pool, &owner, "normal", "SEK").await;
    let income = event(&pool, &owner, "income", Some(&a), None).await;
    event(&pool, &owner, "expense", Some(&a), None).await;
    assert_eq!(balance(&pool, &a).await, 0);
    let path = format!("/v1/accounts/{a}");
    assert_eq!(
        send(
            app(Some(pool.clone())),
            Some(&token),
            "PATCH",
            &path,
            None,
            "{\"status\":\"archived\"}"
        )
        .await
        .0,
        StatusCode::OK
    );
    ok(
        edit(
            &pool,
            &token,
            &income,
            "archived-meta",
            json!({"description":"Historical correction"}),
        )
        .await,
        &income,
        "posted",
    );
    for (key, patch) in [
        ("archived-amount", json!({"amount":"50"})),
        ("archived-date", json!({"occurred_on":"2024-03-01"})),
        ("archived-unlink", json!({"account_id":null})),
        ("archived-reassign", json!({"account_id":b})),
    ] {
        let before = snapshot(&pool).await;
        error(
            edit(&pool, &token, &income, key, patch).await,
            StatusCode::CONFLICT,
            "account_archived",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let before = snapshot(&pool).await;
    error(
        void(&pool, &token, &income, "archived-void").await,
        StatusCode::CONFLICT,
        "account_archived",
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(
        send(
            app(Some(pool.clone())),
            Some(&token),
            "PATCH",
            &path,
            None,
            "{\"status\":\"active\"}"
        )
        .await
        .0,
        StatusCode::OK
    );
    ok(
        void(&pool, &token, &income, "archived-void").await,
        &income,
        "voided",
    );
    assert_eq!(balance(&pool, &a).await, -10000);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn retries_conflicts_concurrent_edits_and_voids_are_exactly_once() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let a = account(&pool, &owner, "normal", "SEK").await;
    let id = event(&pool, &owner, "income", Some(&a), None).await;
    let (first, second) = tokio::join!(
        edit(&pool, &token, &id, "race-edit", json!({"amount":"25.00"})),
        edit(&pool, &token, &id, "race-edit", json!({"amount":"025"}))
    );
    ok(first, &id, "posted");
    ok(second, &id, "posted");
    assert_eq!(balance(&pool, &a).await, 2500);
    let before = snapshot(&pool).await;
    error(
        edit(&pool, &token, &id, "race-edit", json!({"amount":"26"})).await,
        StatusCode::CONFLICT,
        "idempotency_conflict",
    );
    assert_eq!(snapshot(&pool).await, before);
    ok(
        edit(&pool, &token, &id, "later", json!({"amount":"30"})).await,
        &id,
        "posted",
    );
    let before = snapshot(&pool).await;
    ok(
        edit(&pool, &token, &id, "race-edit", json!({"amount":"25"})).await,
        &id,
        "posted",
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(balance(&pool, &a).await, 3000);
    let (first, second) = tokio::join!(
        void(&pool, &token, &id, "race-void"),
        void(&pool, &token, &id, "race-void")
    );
    ok(first, &id, "voided");
    ok(second, &id, "voided");
    assert_eq!(balance(&pool, &a).await, 0);
    let before = snapshot(&pool).await;
    ok(
        edit(&pool, &token, &id, "race-edit", json!({"amount":"25"})).await,
        &id,
        "posted",
    );
    assert_eq!(snapshot(&pool).await, before);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM transaction_audit WHERE transaction_id=$1::uuid AND action='void'",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let id2 = event(&pool, &owner, "expense", None, None).await;
    let before = snapshot(&pool).await;
    error(
        edit(&pool, &token, &id2, "race-edit", json!({"amount":"25"})).await,
        StatusCode::CONFLICT,
        "idempotency_conflict",
    );
    assert_eq!(snapshot(&pool).await, before);
    let (first, second) = tokio::join!(
        edit(
            &pool,
            &token,
            &id2,
            "different-edit",
            json!({"amount":"20"})
        ),
        void(&pool, &token, &id2, "different-void")
    );
    ok(second, &id2, "voided");
    assert!(first.0 == StatusCode::OK || first.0 == StatusCode::CONFLICT);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn gates_invalid_fields_dates_and_relationships_leave_no_writes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let (other, other_token) = seed(&pool).await;
    let a = account(&pool, &owner, "normal", "SEK").await;
    let foreign = account(&pool, &other, "normal", "SEK").await;
    let eur = account(&pool, &owner, "normal", "EUR").await;
    let id = event(&pool, &owner, "income", Some(&a), None).await;
    for patch in [
        json!({"amount":"0"}),
        json!({"amount":"-1"}),
        json!({"currency":"EUR"}),
        json!({"kind":"interest_income"}),
        json!({"amount":null}),
        json!({"occurred_on":null}),
        json!({"state":"voided"}),
        json!({}),
        json!({"description":"\n"}),
        json!({"destination_account_id":eur}),
        json!({"received_amount":"1"}),
    ] {
        let before = snapshot(&pool).await;
        error(
            edit(&pool, &token, &id, "bad", patch).await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for (patch, code) in [
        (json!({"occurred_on":"2026-02-29"}), "invalid_business_date"),
        (json!({"occurred_on":"0000-01-01"}), "date_out_of_range"),
        (json!({"occurred_on":"9999-01-01"}), "future_business_date"),
    ] {
        let before = snapshot(&pool).await;
        error(
            edit(&pool, &token, &id, "date", patch).await,
            StatusCode::BAD_REQUEST,
            code,
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let before = snapshot(&pool).await;
    error(
        edit(
            &pool,
            &other_token,
            &id,
            "foreign-event",
            json!({"amount":"50"}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "not_found",
    );
    error(
        void(&pool, &other_token, &id, "foreign-void").await,
        StatusCode::NOT_FOUND,
        "not_found",
    );
    error(
        edit(
            &pool,
            &token,
            &id,
            "foreign-account",
            json!({"account_id":foreign}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "not_found",
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    error(
        edit(&pool, &token, &id, "pending", json!({"description":"No"})).await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    error(
        void(&pool, &token, &id, "pending").await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET email_verified_at=statement_timestamp(),profile_time_zone=NULL WHERE id=$1::uuid").bind(&owner).execute(&pool).await.unwrap();
    error(
        edit(
            &pool,
            &token,
            &id,
            "zone",
            json!({"occurred_on":"2024-03-01"}),
        )
        .await,
        StatusCode::CONFLICT,
        "profile_zone_unconfirmed",
    );
    ok(
        edit(
            &pool,
            &token,
            &id,
            "no-zone-meta",
            json!({"description":"Allowed"}),
        )
        .await,
        &id,
        "posted",
    );
    let before = snapshot(&pool).await;
    for raw in [
        "{\"description\":\"a\",\"description\":\"b\"}",
        "{\"balance\":\"0\"}",
        "{\"user_id\":null}",
    ] {
        error(
            send(
                app(Some(pool.clone())),
                Some(&token),
                "PATCH",
                &format!("/v1/transactions/{id}"),
                Some("malformed"),
                raw,
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
    }
    error(
        send(
            app(Some(pool.clone())),
            Some(&token),
            "PATCH",
            &format!("/v1/transactions/{id}?extra=1"),
            Some("query"),
            "{\"amount\":\"1\"}",
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    error(
        send(
            app(Some(pool.clone())),
            Some(&token),
            "PATCH",
            &format!("/v1/transactions/{id}"),
            None,
            "{\"amount\":\"1\"}",
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn transfer_reassignment_and_same_currency_rules_and_metadata_noops() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let a = account(&pool, &owner, "normal", "SEK").await;
    let b = account(&pool, &owner, "savings", "EUR").await;
    let c = account(&pool, &owner, "savings", "SEK").await;
    let id = event(&pool, &owner, "transfer", Some(&a), Some(&b)).await;
    for patch in [
        json!({"destination_account_id":a,"received_currency":"SEK","received_amount":"100"}),
        json!({"destination_account_id":null}),
        json!({"received_currency":null}),
        json!({"received_amount":"0"}),
        json!({"destination_account_id":c,"received_currency":"SEK","received_amount":"99"}),
    ] {
        let before = snapshot(&pool).await;
        error(
            edit(&pool, &token, &id, "invalid-transfer", patch).await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    ok(
        edit(
            &pool,
            &token,
            &id,
            "same-currency",
            json!({"destination_account_id":c,"received_currency":"SEK","received_amount":"100"}),
        )
        .await,
        &id,
        "posted",
    );
    assert_eq!(balance(&pool, &a).await, -10000);
    assert_eq!(balance(&pool, &b).await, 0);
    assert_eq!(balance(&pool, &c).await, 10000);
    ok(
        edit(&pool, &token, &id, "desc", json!({"description":"Changed"})).await,
        &id,
        "posted",
    );
    let before: (i64,i64,String) = sqlx::query_as("SELECT (SELECT count(*) FROM account_movements WHERE transaction_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE transaction_id=$1::uuid),updated_at::text FROM transactions WHERE id=$1::uuid").bind(&id).fetch_one(&pool).await.unwrap();
    ok(
        edit(
            &pool,
            &token,
            &id,
            "noop",
            json!({"amount":"100","description":"Changed"}),
        )
        .await,
        &id,
        "posted",
    );
    let after: (i64,i64,String) = sqlx::query_as("SELECT (SELECT count(*) FROM account_movements WHERE transaction_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE transaction_id=$1::uuid),updated_at::text FROM transactions WHERE id=$1::uuid").bind(&id).fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn late_failures_rollback_both_transfer_legs_event_audit_and_retry() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let a = account(&pool, &owner, "normal", "SEK").await;
    let b = account(&pool, &owner, "normal", "EUR").await;
    let id = event(&pool, &owner, "transfer", Some(&a), Some(&b)).await;
    for table in [
        "account_movements",
        "transactions",
        "transaction_audit",
        "transaction_mutation_keys",
    ] {
        let trigger = if table == "transactions" {
            "UPDATE"
        } else {
            "INSERT"
        };
        // Failure after a reversal or replacement leg, or at a later write.
        sqlx::raw_sql(&format!("CREATE FUNCTION fail_mutation_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF TG_TABLE_NAME = 'account_movements' THEN IF NEW.account_id = '{b}'::uuid THEN RAISE EXCEPTION 'synthetic failure'; END IF; ELSE RAISE EXCEPTION 'synthetic failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_mutation_test BEFORE {trigger} ON {table} FOR EACH ROW EXECUTE FUNCTION fail_mutation_test();")).execute(&pool).await.unwrap();
        let before = snapshot(&pool).await;
        error(
            edit(
                &pool,
                &token,
                &id,
                "rollback",
                json!({"amount":"150","received_amount":"27"}),
            )
            .await,
            StatusCode::SERVICE_UNAVAILABLE,
            "transaction_update_unavailable",
        );
        error(
            void(&pool, &token, &id, "rollback-void").await,
            StatusCode::SERVICE_UNAVAILABLE,
            "transaction_update_unavailable",
        );
        assert_eq!(snapshot(&pool).await, before);
        sqlx::raw_sql(&format!(
            "DROP TRIGGER fail_mutation_test ON {table}; DROP FUNCTION fail_mutation_test();"
        ))
        .execute(&pool)
        .await
        .unwrap();
    }
    ok(
        edit(
            &pool,
            &token,
            &id,
            "rollback",
            json!({"amount":"150","received_amount":"27"}),
        )
        .await,
        &id,
        "posted",
    );
    assert_eq!(balance(&pool, &a).await, -15000);
    assert_eq!(balance(&pool, &b).await, 2700);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn creation_replays_remain_original_after_edit_and_void() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool).await;
    for kind in ["income", "expense"] {
        let raw = json!({"kind":kind,"amount":"100","currency":"SEK","occurred_on":"2024-02-29"})
            .to_string();
        let original = send(
            app(Some(pool.clone())),
            Some(&token),
            "POST",
            "/v1/transactions",
            Some(kind),
            &raw,
        )
        .await;
        assert_eq!(original.0, StatusCode::CREATED);
        let id = original.1["id"].as_str().unwrap();
        ok(
            edit(
                &pool,
                &token,
                id,
                &format!("edit-{kind}"),
                json!({"amount":"30"}),
            )
            .await,
            id,
            "posted",
        );
        ok(
            void(&pool, &token, id, &format!("void-{kind}")).await,
            id,
            "voided",
        );
        let before = snapshot(&pool).await;
        assert_eq!(
            send(
                app(Some(pool.clone())),
                Some(&token),
                "POST",
                "/v1/transactions",
                Some(kind),
                &raw
            )
            .await,
            original
        );
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn independent_archives_dates_cancel_and_audit() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let a = account(&pool, &owner, "normal", "SEK").await;
    let b = account(&pool, &owner, "savings", "EUR").await;
    let c = account(&pool, &owner, "normal", "SEK").await;
    let id = event(&pool, &owner, "transfer", Some(&a), Some(&b)).await;
    // Zero both balances using independent offset records, then use the public archive API.
    let offset = event(&pool, &owner, "income", Some(&a), None).await;
    let e = event(&pool, &owner, "expense", None, None).await;
    ok(
        edit(
            &pool,
            &token,
            &e,
            "offset-dest",
            json!({"account_id":b,"currency":"EUR","amount":"18"}),
        )
        .await,
        &e,
        "posted",
    );
    for account in [&a, &b] {
        assert_eq!(balance(&pool, account).await, 0);
        assert_eq!(
            send(
                app(Some(pool.clone())),
                Some(&token),
                "PATCH",
                &format!("/v1/accounts/{account}"),
                None,
                "{\"status\":\"archived\"}"
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let before = snapshot(&pool).await;
    for patch in [
        json!({"amount":"101"}),
        json!({"received_amount":"19"}),
        json!({"account_id":c}),
        json!({"destination_account_id":c,"received_currency":"SEK","received_amount":"100"}),
    ] {
        error(
            edit(&pool, &token, &id, "archived-side", patch).await,
            StatusCode::CONFLICT,
            "account_archived",
        );
        assert_eq!(before, snapshot(&pool).await);
    }
    ok(
        edit(
            &pool,
            &token,
            &id,
            "arch-meta",
            json!({"description":"Archive annotation"}),
        )
        .await,
        &id,
        "posted",
    );
    let fields:Vec<String>=sqlx::query_scalar("SELECT changed_fields FROM transaction_audit WHERE transaction_id=$1::uuid AND action='edit' ORDER BY created_at DESC LIMIT 1").bind(&id).fetch_one(&pool).await.unwrap();
    assert_eq!(fields, vec!["description"]);
    // No confirmed zone: unrelated metadata still allowed, explicit unchanged date rejected.
    sqlx::query("UPDATE users SET profile_time_zone=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    ok(
        edit(
            &pool,
            &token,
            &offset,
            "zone-meta",
            json!({"description":"No date supplied"}),
        )
        .await,
        &offset,
        "posted",
    );
    let unlinked = event(&pool, &owner, "expense", None, None).await;
    let before = snapshot(&pool).await;
    error(
        edit(
            &pool,
            &token,
            &unlinked,
            "archived-new",
            json!({"account_id":a}),
        )
        .await,
        StatusCode::CONFLICT,
        "account_archived",
    );
    assert_eq!(before, snapshot(&pool).await);
    let noops_before:(i64,i64,String)=sqlx::query_as("SELECT (SELECT count(*) FROM account_movements WHERE transaction_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE transaction_id=$1::uuid),updated_at::text FROM transactions WHERE id=$1::uuid").bind(&offset).fetch_one(&pool).await.unwrap();
    ok(
        edit(
            &pool,
            &token,
            &offset,
            "archive-noop",
            json!({"amount":"100","description":"No date supplied"}),
        )
        .await,
        &offset,
        "posted",
    );
    let noops_after:(i64,i64,String)=sqlx::query_as("SELECT (SELECT count(*) FROM account_movements WHERE transaction_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE transaction_id=$1::uuid),updated_at::text FROM transactions WHERE id=$1::uuid").bind(&offset).fetch_one(&pool).await.unwrap();
    assert_eq!(noops_before, noops_after);
    let before = snapshot(&pool).await;
    error(
        edit(
            &pool,
            &token,
            &offset,
            "zone-explicit",
            json!({"occurred_on":"2024-02-29"}),
        )
        .await,
        StatusCode::CONFLICT,
        "profile_zone_unconfirmed",
    );
    assert_eq!(before, snapshot(&pool).await);
    // Cancel a public operation while it waits on owner serialization. Drop must rollback without financial/retry writes.
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
        .bind(&owner)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    let request = void(&pool, &token, &e, "cancelled");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), request)
            .await
            .is_err()
    );
    lock.rollback().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(before, snapshot(&pool).await);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn independent_archive_race_preserves_zero_balance_invariant() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool).await;
    let a = account(&pool, &owner, "normal", "SEK").await;
    let id = event(&pool, &owner, "opening_balance", Some(&a), None).await;
    ok(
        edit(&pool, &token, &id, "zero-start", json!({"amount":"0"})).await,
        &id,
        "posted",
    );
    let path = format!("/v1/accounts/{a}");
    let (edited, archived) = tokio::join!(
        edit(
            &pool,
            &token,
            &id,
            "race-against-archive",
            json!({"amount":"1"})
        ),
        send(
            app(Some(pool.clone())),
            Some(&token),
            "PATCH",
            &path,
            None,
            "{\"status\":\"archived\"}"
        )
    );
    assert!(edited.0 == StatusCode::OK || archived.0 == StatusCode::OK);
    assert_ne!(edited.0 == StatusCode::OK, archived.0 == StatusCode::OK);
    let active: bool =
        sqlx::query_scalar("SELECT archived_at IS NULL FROM accounts WHERE id=$1::uuid")
            .bind(&a)
            .fetch_one(&pool)
            .await
            .unwrap();
    if !active {
        assert_eq!(balance(&pool, &a).await, 0);
        error(edited, StatusCode::CONFLICT, "account_archived");
    } else {
        assert_eq!(balance(&pool, &a).await, 100);
    }
}
