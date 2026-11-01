//! Actual-payment confirmation through HTTP, using synthetic disposable data only.
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

async fn request(
    router: &Router,
    token: Option<&str>,
    path: &str,
    keys: &[&str],
    body: &str,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    for key in keys {
        req = req.header("idempotency-key", *key);
    }
    let response = router
        .clone()
        .oneshot(req.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(headers["cache-control"], "no-store");
    if value.get("code").is_some() {
        assert_eq!(
            value["request_id"],
            headers["x-request-id"].to_str().unwrap()
        );
        assert!(value["details"].is_null());
    }
    (status, value)
}
async fn confirm(
    router: &Router,
    token: &str,
    occurrence: &str,
    key: &str,
    body: Value,
) -> (StatusCode, Value) {
    request(
        router,
        Some(token),
        &format!("/v1/recurring-occurrences/{occurrence}/confirm"),
        &[key],
        &body.to_string(),
    )
    .await
}
fn actual() -> Value {
    json!({"amount":"123.45","currency":"SEK","occurred_on":"2024-02-29","description":"Actual payment"})
}
fn error(reply: (StatusCode, Value), status: StatusCode, code: &str) {
    assert_eq!(reply.0, status, "{}", reply.1);
    assert_eq!(reply.1["code"], code);
}
async fn seed(pool: &PgPool, verified: bool, zone: Option<&str>) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let owner: String = sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at,profile_time_zone) VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END,$3) RETURNING id::text").bind(email).bind(verified).bind(zone).fetch_one(pool).await.unwrap();
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
async fn occurrence(pool: &PgPool, owner: &str) -> String {
    let rule: String = sqlx::query_scalar("INSERT INTO recurring_rules(user_id,amount_minor,currency,cadence,starts_on,ends_on,time_zone,state,paused_on) VALUES($1::uuid,10000,'SEK','monthly','2024-01-31','2024-01-31','Europe/Stockholm','paused','2024-02-01') RETURNING id::text").bind(owner).fetch_one(pool).await.unwrap();
    sqlx::query_scalar("INSERT INTO recurring_occurrences(rule_id,user_id,scheduled_on,due_on,amount_minor,currency,description) VALUES($1::uuid,$2::uuid,'2024-01-31','2024-01-31',10000,'SEK','Planned') RETURNING id::text").bind(rule).bind(owner).fetch_one(pool).await.unwrap()
}
async fn account(pool: &PgPool, owner: &str, currency: &str, archived: bool) -> String {
    sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency,archived_at) VALUES($1::uuid,'Synthetic','normal',$2,CASE WHEN $3 THEN statement_timestamp() END) RETURNING id::text").bind(owner).bind(currency).bind(archived).fetch_one(pool).await.unwrap()
}
async fn snapshot(pool: &PgPool) -> String {
    // Include every public table, including permanent retry rows, not just balances.
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename::text FROM pg_tables WHERE schemaname='public' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut rows = vec![common::snapshot(pool).await];
    for table in tables {
        assert!(
            table
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        );
        let hash: String = sqlx::query_scalar(&format!("SELECT md5(coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]')::text) FROM {table} t")).fetch_one(pool).await.unwrap();
        rows.push(hash);
    }
    rows.join(":")
}
async fn reject(
    pool: &PgPool,
    router: &Router,
    token: &str,
    id: &str,
    key: &str,
    body: Value,
    expected: (StatusCode, &str),
) {
    let before = snapshot(pool).await;
    error(
        confirm(router, token, id, key, body).await,
        expected.0,
        expected.1,
    );
    assert_eq!(snapshot(pool).await, before);
}

#[tokio::test]
async fn authentication_precedes_payload_validation() {
    error(
        request(
            &app(None),
            None,
            "/v1/recurring-occurrences/invalid/confirm",
            &[],
            "{",
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn accountless_actual_terms_post_once_and_preserve_planning() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    for currency in ["SEK", "EUR", "USD"] {
        let id = occurrence(&pool, &owner).await;
        let mut body = actual();
        body["currency"] = json!(currency);
        let first = confirm(&router, &token, &id, currency, body.clone()).await;
        assert_eq!(first.0, StatusCode::CREATED, "{}", first.1);
        let transaction = first.1["id"].as_str().unwrap();
        assert_eq!(
            first.1,
            json!({"id":transaction,"kind":"expense","state":"posted","amount":"123.45","currency":currency,"occurred_on":"2024-02-29","account_id":null,"category_id":null,"description":"Actual payment"})
        );
        let linked: bool = sqlx::query_scalar("SELECT state='confirmed' AND transaction_id=$2::uuid AND scheduled_on=DATE '2024-01-31' AND due_on=scheduled_on AND amount_minor=10000 AND currency='SEK' AND description='Planned' FROM recurring_occurrences WHERE id=$1::uuid").bind(&id).bind(transaction).fetch_one(&pool).await.unwrap();
        assert!(linked);
        let stored: bool = sqlx::query_scalar("SELECT kind='expense' AND state='posted' AND amount_minor=12345 AND currency=$2 AND occurred_on=DATE '2024-02-29' AND account_id IS NULL FROM transactions WHERE id=$1::uuid").bind(transaction).bind(currency).fetch_one(&pool).await.unwrap();
        assert!(stored);
        let before = snapshot(&pool).await;
        body["amount"] = json!("0123.45");
        assert_eq!(
            confirm(&router, &token, &id.to_uppercase(), currency, body).await,
            first
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM account_movements WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid AND actor_id=user_id AND action='create' AND changed_fields=ARRAY[]::text[])").bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (3, 0, 3));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn linked_confirmation_uses_reviewed_account_and_category_not_planned_reference() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let planned = account(&pool, &owner, "SEK", true).await;
    let actual_account = account(&pool, &owner, "EUR", false).await;
    sqlx::query("UPDATE recurring_occurrences SET account_id=$2::uuid WHERE id=$1::uuid")
        .bind(&id)
        .bind(&planned)
        .execute(&pool)
        .await
        .unwrap();
    let category: String = sqlx::query_scalar(
        "SELECT id::text FROM categories WHERE kind='expense' ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut body = actual();
    body["currency"] = json!("EUR");
    body["account_id"] = json!(actual_account);
    body["category_id"] = json!(category);
    let reply = confirm(&app(Some(pool.clone())), &token, &id, "linked", body).await;
    assert_eq!(reply.0, StatusCode::CREATED, "{}", reply.1);
    let row: (i64, String, String, String) = sqlx::query_as("SELECT signed_amount_minor,currency,account_id::text,to_char(occurred_on,'YYYY-MM-DD') FROM account_movements WHERE transaction_id=$1::uuid").bind(reply.1["id"].as_str().unwrap()).fetch_one(&pool).await.unwrap();
    assert_eq!(
        row,
        (
            -12345,
            "EUR".into(),
            actual_account.clone(),
            "2024-02-29".into()
        )
    );
    assert_eq!(reply.1["account_id"], actual_account);
    assert_eq!(reply.1["category_id"], category);
    let planned_movements: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_movements WHERE account_id=$1::uuid")
            .bind(planned)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(planned_movements, 0);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn validation_and_identity_rejections_change_no_public_rows() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (_, foreign) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    reject(
        &pool,
        &router,
        &foreign,
        &id,
        "foreign",
        actual(),
        (StatusCode::NOT_FOUND, "not_found"),
    )
    .await;
    for (verified, zone, code) in [
        (false, Some("Etc/UTC"), "email_not_verified"),
        (true, None, "profile_zone_unconfirmed"),
    ] {
        let (other, bearer) = seed(&pool, verified, zone).await;
        let target = occurrence(&pool, &other).await;
        reject(
            &pool,
            &router,
            &bearer,
            &target,
            "guard",
            actual(),
            (
                if verified {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::FORBIDDEN
                },
                code,
            ),
        )
        .await;
    }
    for (field, value, code) in [
        ("amount", json!(0.1), "invalid_request"),
        ("amount", json!("0"), "invalid_request"),
        ("kind", json!("income"), "invalid_request"),
        ("user_id", json!(owner), "invalid_request"),
        ("occurred_on", json!("2024-02-30"), "invalid_business_date"),
        ("occurred_on", json!("9999-12-31"), "future_business_date"),
    ] {
        let mut body = actual();
        body[field] = value;
        reject(
            &pool,
            &router,
            &token,
            &id,
            "invalid",
            body,
            (StatusCode::BAD_REQUEST, code),
        )
        .await;
    }
    for keys in [vec![], vec!["a", "b"], vec!["bad key"]] {
        let before = snapshot(&pool).await;
        error(
            request(
                &router,
                Some(&token),
                &format!("/v1/recurring-occurrences/{id}/confirm"),
                &keys,
                &actual().to_string(),
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let before = snapshot(&pool).await;
    error(
        request(
            &router,
            Some(&token),
            &format!("/v1/recurring-occurrences/{id}/confirm?x=1"),
            &["query"],
            &actual().to_string(),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn invalid_account_category_and_terminal_states_reject_atomically() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let (other, _) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    for (account, status, code) in [
        (
            account(&pool, &other, "SEK", false).await,
            StatusCode::NOT_FOUND,
            "not_found",
        ),
        (
            account(&pool, &owner, "SEK", true).await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
        (
            account(&pool, &owner, "USD", false).await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
    ] {
        let mut body = actual();
        body["account_id"] = json!(account);
        reject(
            &pool,
            &router,
            &token,
            &id,
            "bad-account",
            body,
            (status, code),
        )
        .await;
    }
    let income_category: String = sqlx::query_scalar(
        "SELECT id::text FROM categories WHERE kind='income' ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut body = actual();
    body["category_id"] = json!(income_category);
    reject(
        &pool,
        &router,
        &token,
        &id,
        "bad-category",
        body,
        (StatusCode::BAD_REQUEST, "invalid_request"),
    )
    .await;
    sqlx::query("UPDATE recurring_occurrences SET state='skipped' WHERE id=$1::uuid")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    reject(
        &pool,
        &router,
        &token,
        &id,
        "skipped",
        actual(),
        (StatusCode::CONFLICT, "occurrence_state_conflict"),
    )
    .await;
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn retries_preserve_original_result_and_independent_expense_namespace() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let second = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    let first = confirm(&router, &token, &id, "shared", actual()).await;
    assert_eq!(first.0, StatusCode::CREATED, "{}", first.1);
    for field in [
        "amount",
        "currency",
        "occurred_on",
        "description",
        "account_id",
        "category_id",
    ] {
        let mut body = actual();
        body[field] = match field {
            "amount" => json!("124"),
            "currency" => json!("EUR"),
            "occurred_on" => json!("2024-02-28"),
            "account_id" | "category_id" => json!("00000000-0000-0000-0000-000000000001"),
            _ => json!("changed"),
        };
        reject(
            &pool,
            &router,
            &token,
            &id,
            "shared",
            body,
            (StatusCode::CONFLICT, "idempotency_conflict"),
        )
        .await;
    }
    reject(
        &pool,
        &router,
        &token,
        &second,
        "shared",
        actual(),
        (StatusCode::CONFLICT, "idempotency_conflict"),
    )
    .await;
    reject(
        &pool,
        &router,
        &token,
        &id,
        "new-key",
        actual(),
        (StatusCode::CONFLICT, "occurrence_state_conflict"),
    )
    .await;
    let mut ordinary = actual();
    ordinary["kind"] = json!("expense");
    let ordinary = request(
        &router,
        Some(&token),
        "/v1/transactions",
        &["shared"],
        &ordinary.to_string(),
    )
    .await;
    assert_eq!(ordinary.0, StatusCode::CREATED, "{}", ordinary.1);
    assert_ne!(ordinary.1["id"], first.1["id"]);
    // Existing correction/void workflow does not reopen occurrence or rewrite retry intent.
    let path = format!("/v1/transactions/{}/void", first.1["id"].as_str().unwrap());
    assert_eq!(
        request(&router, Some(&token), &path, &["void"], "{}")
            .await
            .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE users SET profile_time_zone=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    assert_eq!(
        confirm(&router, &token, &id, "shared", actual()).await,
        first
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_confirmations_have_one_atomic_winner() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let id = occurrence(&pool, &owner).await;
    let (a, b) = tokio::join!(
        confirm(&router, &token, &id, "same", actual()),
        confirm(&router, &token, &id, "same", actual())
    );
    assert_eq!(a.0, StatusCode::CREATED, "{}", a.1);
    assert_eq!(a, b);
    let id = occurrence(&pool, &owner).await;
    let (a, b) = tokio::join!(
        confirm(&router, &token, &id, "different-a", actual()),
        confirm(&router, &token, &id, "different-b", actual())
    );
    let mut statuses = [a.0.as_u16(), b.0.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [201, 409]);
    let a_id = occurrence(&pool, &owner).await;
    let b_id = occurrence(&pool, &owner).await;
    let (a, b) = tokio::join!(
        confirm(&router, &token, &a_id, "collision", actual()),
        confirm(&router, &token, &b_id, "collision", actual())
    );
    let mut statuses = [a.0.as_u16(), b.0.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [201, 409]);
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid)").bind(owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (3, 3));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn late_failures_roll_back_expense_movement_audit_and_occurrence() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let router = app(Some(pool.clone()));
    let id = occurrence(&pool, &owner).await;
    let linked = account(&pool, &owner, "SEK", false).await;
    let mut body = actual();
    body["account_id"] = json!(linked);
    for (table, action) in [
        ("account_movements", "INSERT"),
        ("transaction_audit", "INSERT"),
        ("recurring_occurrences", "UPDATE"),
        ("recurring_confirmation_keys", "INSERT"),
    ] {
        sqlx::query("CREATE OR REPLACE FUNCTION fail_confirmation_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Synthetic late failure'; END; $$").execute(&pool).await.unwrap();
        sqlx::query(&format!("CREATE TRIGGER fail_confirmation_test BEFORE {action} ON {table} FOR EACH ROW EXECUTE FUNCTION fail_confirmation_test()" )).execute(&pool).await.unwrap();
        reject(
            &pool,
            &router,
            &token,
            &id,
            "late",
            body.clone(),
            (StatusCode::SERVICE_UNAVAILABLE, "recurrence_unavailable"),
        )
        .await;
        sqlx::query(&format!("DROP TRIGGER fail_confirmation_test ON {table}"))
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("DROP FUNCTION fail_confirmation_test()")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        confirm(&router, &token, &id, "late", body).await.0,
        StatusCode::CREATED
    );
}

async fn wait_for_lock(pool: &PgPool, marker: &str) {
    for _ in 0..100 {
        let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE $1)").bind(format!("%{marker}%")).fetch_one(pool).await.unwrap();
        if waiting {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("No observed DB lock wait for {marker}");
}
async fn wait_for_idle(pool: &PgPool) {
    for _ in 0..350 {
        let busy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND state IN ('active','idle in transaction','idle in transaction (aborted)'))").fetch_one(pool).await.unwrap();
        if !busy {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("Cancelled request retained a database transaction");
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn cancellation_during_owner_wait_and_posting_preserves_all_rows() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    for posting in [false, true] {
        let mut lock = pool.begin().await.unwrap();
        if posting {
            sqlx::query("SELECT pg_advisory_xact_lock(871001)")
                .execute(&mut *lock)
                .await
                .unwrap();
            sqlx::raw_sql("CREATE FUNCTION probe_block_post() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(871001); RETURN NEW; END $$; CREATE TRIGGER probe_block_post BEFORE INSERT ON transaction_audit FOR EACH ROW EXECUTE FUNCTION probe_block_post();").execute(&pool).await.unwrap();
        } else {
            sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
                .bind(&owner)
                .execute(&mut *lock)
                .await
                .unwrap();
        }
        let before = snapshot(&pool).await;
        let (r, t, i) = (router.clone(), token.clone(), id.clone());
        let handle = tokio::spawn(async move { confirm(&r, &t, &i, "cancelled", actual()).await });
        wait_for_lock(
            &pool,
            if posting {
                "INSERT INTO transaction_audit"
            } else {
                "SELECT email_verified_at"
            },
        )
        .await;
        handle.abort();
        assert!(handle.await.unwrap_err().is_cancelled());
        // Hold blocker past statement timeout: cancellation must not become a commit.
        tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        lock.rollback().await.unwrap();
        wait_for_idle(&pool).await;
        assert_eq!(snapshot(&pool).await, before);
        if posting {
            sqlx::raw_sql("DROP TRIGGER probe_block_post ON transaction_audit; DROP FUNCTION probe_block_post();").execute(&pool).await.unwrap();
        }
    }
    assert_eq!(
        confirm(&router, &token, &id, "cancelled", actual()).await.0,
        StatusCode::CREATED
    );
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn verification_is_rechecked_after_owner_wait() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&mut *lock)
        .await
        .unwrap();
    let (r, t, i) = (router.clone(), token.clone(), id.clone());
    let handle = tokio::spawn(async move { confirm(&r, &t, &i, "reverify", actual()).await });
    wait_for_lock(&pool, "SELECT email_verified_at").await;
    lock.commit().await.unwrap();
    let before = snapshot(&pool).await;
    error(
        handle.await.unwrap(),
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn archived_account_is_rechecked_after_serialized_wait() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let account = account(&pool, &owner, "SEK", false).await;
    let mut body = actual();
    body["account_id"] = json!(account);
    let router = app(Some(pool.clone()));
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
        .bind(&owner)
        .execute(&mut *lock)
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&account)
        .execute(&mut *lock)
        .await
        .unwrap();
    let (r, t, i) = (router.clone(), token.clone(), id.clone());
    let handle = tokio::spawn(async move { confirm(&r, &t, &i, "archive-wait", body).await });
    wait_for_lock(&pool, "SELECT email_verified_at").await;
    lock.commit().await.unwrap();
    let before = snapshot(&pool).await;
    error(
        handle.await.unwrap(),
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    assert_eq!(snapshot(&pool).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn raw_key_is_owner_scoped() {
    let pool = common::pool().await;
    let router = app(Some(pool.clone()));
    let mut ids = vec![];
    for _ in 0..2 {
        let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
        let occurrence = occurrence(&pool, &owner).await;
        let result = confirm(&router, &token, &occurrence, "identical-raw-key", actual()).await;
        assert_eq!(result.0, StatusCode::CREATED, "{}", result.1);
        ids.push(result.1["id"].clone());
    }
    assert_ne!(ids[0], ids[1]);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn linked_replay_after_edit_archive_and_zone_change_preserves_history() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let untouched = occurrence(&pool, &owner).await;
    let original_history: String=sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM recurring_rules r),(SELECT to_jsonb(o) FROM recurring_occurrences o WHERE id=$1::uuid))::text)").bind(&untouched).fetch_one(&pool).await.unwrap();
    let account = account(&pool, &owner, "SEK", false).await;
    let mut body = actual();
    body["account_id"] = json!(account);
    let router = app(Some(pool.clone()));
    let first = confirm(&router, &token, &id, "linked-replay", body.clone()).await;
    assert_eq!(first.0, StatusCode::CREATED);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/v1/transactions/{}",
                    first.1["id"].as_str().unwrap()
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "edit-linked")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"amount":"120","account_id":null,"description":"Edited actual expense"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let archive = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/v1/accounts/{account}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"status":"archived"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(archive.status(), StatusCode::OK);
    sqlx::query("UPDATE users SET profile_time_zone='Pacific/Auckland' WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    assert_eq!(
        confirm(&router, &token, &id, "linked-replay", body).await,
        first
    );
    assert_eq!(snapshot(&pool).await, before);
    let history: String=sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM recurring_rules r),(SELECT to_jsonb(o) FROM recurring_occurrences o WHERE id=$1::uuid))::text)").bind(&untouched).fetch_one(&pool).await.unwrap();
    assert_eq!(history, original_history);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_public_archive_and_confirmation_have_consistent_winner() {
    let pool = common::pool().await;
    for archive_first in [true, false] {
        let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
        let id = occurrence(&pool, &owner).await;
        let account = account(&pool, &owner, "SEK", false).await;
        let mut body = actual();
        body["account_id"] = json!(account);
        let router = app(Some(pool.clone()));
        let mut lock = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
            .bind(&owner)
            .execute(&mut *lock)
            .await
            .unwrap();
        let (r, t, a) = (router.clone(), token.clone(), account.clone());
        let archive = async move {
            r.oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/accounts/{a}"))
                    .header("authorization", format!("Bearer {t}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"status":"archived"}"#))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
        };
        let (r, t, i) = (router.clone(), token.clone(), id.clone());
        let confirmation = async move { confirm(&r, &t, &i, "public-race", body).await };
        let (archive, confirmation) = if archive_first {
            let a = tokio::spawn(archive);
            wait_for_lock(&pool, "FROM users").await;
            let c = tokio::spawn(confirmation);
            (a, c)
        } else {
            let c = tokio::spawn(confirmation);
            wait_for_lock(&pool, "SELECT email_verified_at").await;
            let a = tokio::spawn(archive);
            (a, c)
        };
        lock.commit().await.unwrap();
        let (a, c) = (archive.await.unwrap(), confirmation.await.unwrap());
        if archive_first {
            assert_eq!(a, StatusCode::OK);
            error(c, StatusCode::BAD_REQUEST, "invalid_request");
        } else {
            assert_eq!(c.0, StatusCode::CREATED);
            assert_eq!(a, StatusCode::CONFLICT);
        }
        let (archived,state,balance):(bool,String,i64)=sqlx::query_as("SELECT a.archived_at IS NOT NULL,o.state,(SELECT coalesce(sum(signed_amount_minor),0)::bigint FROM account_movements WHERE account_id=a.id) FROM accounts a CROSS JOIN recurring_occurrences o WHERE a.id=$1::uuid AND o.id=$2::uuid").bind(&account).bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(
            (archived, state.as_str(), balance),
            if archive_first {
                (true, "planned", 0)
            } else {
                (false, "confirmed", -12345)
            }
        );
    }
}
