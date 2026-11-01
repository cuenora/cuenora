//! Planning-only actions through the public HTTP interface on disposable data.
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
async fn action(
    router: &Router,
    token: &str,
    id: &str,
    operation: &str,
    key: &str,
    body: Value,
) -> (StatusCode, Value) {
    request(
        router,
        Some(token),
        &format!("/v1/recurring-occurrences/{id}/{operation}"),
        &[key],
        &body.to_string(),
    )
    .await
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
#[tokio::test]
async fn authentication_precedes_action_payload_validation() {
    for operation in ["skip", "postpone"] {
        error(
            request(
                &app(None),
                None,
                &format!("/v1/recurring-occurrences/invalid/{operation}"),
                &[],
                "{",
            )
            .await,
            StatusCode::UNAUTHORIZED,
            "invalid_access_token",
        );
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn postpone_preserves_slot_terms_and_rule_with_strictly_later_due_dates() {
    let pool = common::pool().await;
    // No today-derived financial date: a confirmed profile zone is not needed.
    let (owner, token) = seed(&pool, true, None).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    let original: Value = sqlx::query_scalar(
        "SELECT to_jsonb(o)-'due_on'-'updated_at' FROM recurring_occurrences o WHERE id=$1::uuid",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let rules: Value = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(r)) FROM recurring_rules r WHERE user_id=$1::uuid",
    )
    .bind(&owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    for (index, date) in ["2024-02-05", "2024-02-06", "9999-12-31"]
        .into_iter()
        .enumerate()
    {
        let reply = action(
            &router,
            &token,
            &id,
            "postpone",
            &format!("postpone-{index}"),
            json!({"due_on":date}),
        )
        .await;
        assert_eq!(
            reply,
            (
                StatusCode::OK,
                json!({"id":id,"scheduled_on":"2024-01-31","due_on":date,"state":"planned"})
            )
        );
        let row: Value = sqlx::query_scalar("SELECT to_jsonb(o)-'due_on'-'updated_at' FROM recurring_occurrences o WHERE id=$1::uuid").bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(row, original);
        let before = snapshot(&pool).await;
        for rejected in [date, "2024-01-31", "0001-01-01"] {
            error(
                action(
                    &router,
                    &token,
                    &id,
                    "postpone",
                    "invalid-date",
                    json!({"due_on":rejected}),
                )
                .await,
                StatusCode::BAD_REQUEST,
                "invalid_request",
            );
            assert_eq!(snapshot(&pool).await, before);
        }
    }
    let after: Value = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(r)) FROM recurring_rules r WHERE user_id=$1::uuid",
    )
    .bind(&owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, rules);
    assert_no_money(&pool, &owner).await;
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn skip_is_terminal_and_has_no_financial_effects() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    let original: Value = sqlx::query_scalar(
        "SELECT to_jsonb(o)-'state'-'updated_at' FROM recurring_occurrences o WHERE id=$1::uuid",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        action(&router, &token, &id, "skip", "skip", json!({})).await,
        (
            StatusCode::OK,
            json!({"id":id,"scheduled_on":"2024-01-31","due_on":"2024-01-31","state":"skipped"})
        )
    );
    let after: Value = sqlx::query_scalar(
        "SELECT to_jsonb(o)-'state'-'updated_at' FROM recurring_occurrences o WHERE id=$1::uuid",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, original);
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM recurring_occurrences WHERE user_id=$1::uuid AND state='planned'",
    )
    .bind(&owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pending, 0);
    let before = snapshot(&pool).await;
    for (operation, body) in [
        ("skip", json!({})),
        ("postpone", json!({"due_on":"2024-02-05"})),
        (
            "confirm",
            json!({"amount":"1","currency":"SEK","occurred_on":"2024-01-31"}),
        ),
    ] {
        error(
            action(&router, &token, &id, operation, "new-key", body).await,
            StatusCode::CONFLICT,
            "occurrence_state_conflict",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    assert_no_money(&pool, &owner).await;
}
async fn assert_no_money(pool: &PgPool, owner: &str) {
    let counts: (i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM account_movements WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid)").bind(owner).fetch_one(pool).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn retries_replay_original_results_without_undoing_later_actions() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, Some("Etc/UTC")).await;
    let id = occurrence(&pool, &owner).await;
    let other = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    let body = json!({"due_on":"2024-02-05"});
    let first = action(&router, &token, &id, "postpone", "first", body.clone()).await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(
        action(
            &router,
            &token,
            &id,
            "postpone",
            "second",
            json!({"due_on":"2024-03-05"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let skipped = action(&router, &token, &id, "skip", "skip", json!({})).await;
    assert_eq!(skipped.0, StatusCode::OK);
    let before = snapshot(&pool).await;
    assert_eq!(
        action(
            &router,
            &token,
            &id.to_uppercase(),
            "postpone",
            "first",
            body
        )
        .await,
        first
    );
    assert_eq!(
        action(&router, &token, &id, "skip", "skip", json!({})).await,
        skipped
    );
    for (target, operation, body) in [
        (&id, "postpone", json!({"due_on":"2024-02-06"})),
        (&id, "skip", json!({})),
        (&other, "postpone", json!({"due_on":"2024-02-05"})),
    ] {
        error(
            action(&router, &token, target, operation, "first", body).await,
            StatusCode::CONFLICT,
            "idempotency_conflict",
        );
    }
    assert_eq!(snapshot(&pool).await, before);
    assert_no_money(&pool, &owner).await;
    // Same key belongs to a different namespace when confirming an actual payment.
    let postponed = action(
        &router,
        &token,
        &other,
        "postpone",
        "paid",
        json!({"due_on":"2024-02-05"}),
    )
    .await;
    let paid = action(
        &router,
        &token,
        &other,
        "confirm",
        "paid",
        json!({"amount":"1","currency":"SEK","occurred_on":"2024-01-31"}),
    )
    .await;
    assert_eq!(paid.0, StatusCode::CREATED, "{}", paid.1);
    let before = snapshot(&pool).await;
    assert_eq!(
        action(
            &router,
            &token,
            &other,
            "postpone",
            "paid",
            json!({"due_on":"2024-02-05"})
        )
        .await,
        postponed
    );
    for (operation, body) in [
        ("skip", json!({})),
        ("postpone", json!({"due_on":"2024-03-05"})),
    ] {
        error(
            action(&router, &token, &other, operation, "new", body).await,
            StatusCode::CONFLICT,
            "occurrence_state_conflict",
        );
    }
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn invalid_requests_and_owner_boundaries_preserve_all_public_rows() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let (_, foreign) = seed(&pool, true, None).await;
    let (pending, pending_token) = seed(&pool, false, None).await;
    let id = occurrence(&pool, &owner).await;
    let pending_id = occurrence(&pool, &pending).await;
    let router = app(Some(pool.clone()));
    for (operation, body) in [
        ("skip", json!({})),
        ("postpone", json!({"due_on":"2024-02-05"})),
    ] {
        for (bearer, target, status, code) in [
            (&foreign, id.as_str(), StatusCode::NOT_FOUND, "not_found"),
            (
                &token,
                "00000000-0000-0000-0000-000000000000",
                StatusCode::NOT_FOUND,
                "not_found",
            ),
            (
                &pending_token,
                pending_id.as_str(),
                StatusCode::FORBIDDEN,
                "email_not_verified",
            ),
        ] {
            let before = snapshot(&pool).await;
            error(
                action(&router, bearer, target, operation, "guard", body.clone()).await,
                status,
                code,
            );
            assert_eq!(snapshot(&pool).await, before);
        }
    }
    for (operation, body) in [("skip", "[]"), ("postpone", "[\"2024-02-05\"]")] {
        let before = snapshot(&pool).await;
        error(
            request(
                &router,
                Some(&token),
                &format!("/v1/recurring-occurrences/{id}/{operation}"),
                &["array"],
                body,
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let path = format!("/v1/recurring-occurrences/{id}/postpone");
    let cases = [
        ("{}", "invalid_request"),
        ("null", "invalid_request"),
        ("[]", "invalid_request"),
        ("{\"due_on\":null}", "invalid_request"),
        ("{\"due_on\":42}", "invalid_request"),
        ("{\"due_on\":\"2024-02-30\"}", "invalid_business_date"),
        ("{\"due_on\":\"0000-01-01\"}", "date_out_of_range"),
        ("{\"due_on\":\"10000-01-01\"}", "date_out_of_range"),
        ("{\"due_on\":\"2024-2-05\"}", "invalid_business_date"),
        (
            "{\"due_on\":\"2024-02-05\",\"due_on\":\"2024-03-05\"}",
            "invalid_request",
        ),
        (
            "{\"due_on\":\"2024-02-05\",\"user_id\":\"ignored\"}",
            "invalid_request",
        ),
    ];
    for (body, code) in cases {
        let before = snapshot(&pool).await;
        error(
            request(&router, Some(&token), &path, &["invalid"], body).await,
            StatusCode::BAD_REQUEST,
            code,
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for operation in ["skip", "postpone"] {
        let path = format!("/v1/recurring-occurrences/{id}/{operation}");
        let body = if operation == "skip" {
            "{}"
        } else {
            "{\"due_on\":\"2024-02-05\"}"
        };
        for keys in [vec![], vec!["a", "b"], vec!["bad key"], vec![""], vec!["é"]] {
            let before = snapshot(&pool).await;
            error(
                request(&router, Some(&token), &path, &keys, body).await,
                StatusCode::BAD_REQUEST,
                "invalid_request",
            );
            assert_eq!(snapshot(&pool).await, before);
        }
        for suffix in ["?x=1", "?"] {
            let before = snapshot(&pool).await;
            error(
                request(
                    &router,
                    Some(&token),
                    &(path.clone() + suffix),
                    &["query"],
                    body,
                )
                .await,
                StatusCode::BAD_REQUEST,
                "invalid_request",
            );
            assert_eq!(snapshot(&pool).await, before);
        }
    }
    let before = snapshot(&pool).await;
    error(
        action(
            &router,
            &token,
            &id,
            "skip",
            "extra",
            json!({"due_on":"2024-02-05"}),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    assert_eq!(snapshot(&pool).await, before);
    // Valid requests and retries still require verified email after authorization changes.
    assert_eq!(
        action(
            &router,
            &token,
            &id,
            "postpone",
            "verified",
            json!({"due_on":"2024-02-05"})
        )
        .await
        .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    error(
        action(
            &router,
            &token,
            &id,
            "postpone",
            "verified",
            json!({"due_on":"2024-02-05"}),
        )
        .await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn concurrent_actions_and_cross_owner_keys_are_serialized() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let (other, other_token) = seed(&pool, true, None).await;
    let router = app(Some(pool.clone()));
    let id = occurrence(&pool, &owner).await;
    let body = json!({"due_on":"2024-02-05"});
    let (a, b) = tokio::join!(
        action(&router, &token, &id, "postpone", "retry", body.clone()),
        action(&router, &token, &id, "postpone", "retry", body.clone())
    );
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(a, b);
    let foreign = occurrence(&pool, &other).await;
    assert_eq!(
        action(&router, &other_token, &foreign, "postpone", "retry", body)
            .await
            .0,
        StatusCode::OK
    );
    let id = occurrence(&pool, &owner).await;
    let (a, b) = tokio::join!(
        action(&router, &token, &id, "skip", "race-skip", json!({})),
        action(
            &router,
            &token,
            &id,
            "confirm",
            "race-confirm",
            json!({"amount":"1","currency":"SEK","occurred_on":"2024-01-31"})
        )
    );
    assert!(
        matches!(
            (a.0, b.0),
            (StatusCode::OK, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::CREATED)
        ),
        "{a:?} {b:?}"
    );
    let row: (String, Option<String>) = sqlx::query_as(
        "SELECT state,transaction_id::text FROM recurring_occurrences WHERE id=$1::uuid",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        row.0,
        if a.0 == StatusCode::OK {
            "skipped"
        } else {
            "confirmed"
        }
    );
    assert_eq!(row.1.is_some(), b.0 == StatusCode::CREATED);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn late_retry_insert_failure_rolls_back_both_actions() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let router = app(Some(pool.clone()));
    let id = occurrence(&pool, &owner).await;
    sqlx::query("CREATE FUNCTION fail_p0711_key() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END $$").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_p0711_key BEFORE INSERT ON recurring_occurrence_action_keys FOR EACH ROW EXECUTE FUNCTION fail_p0711_key()").execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    for (operation, body) in [
        ("skip", json!({})),
        ("postpone", json!({"due_on":"2024-02-05"})),
    ] {
        error(
            action(&router, &token, &id, operation, "rollback", body).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "recurrence_unavailable",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    sqlx::query("DROP TRIGGER fail_p0711_key ON recurring_occurrence_action_keys")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION fail_p0711_key()")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        action(
            &router,
            &token,
            &id,
            "postpone",
            "rollback",
            json!({"due_on":"2024-02-05"})
        )
        .await
        .0,
        StatusCode::OK
    );
}

async fn authenticated_identity(
    pool: &PgPool,
    token: &str,
) -> cuenora_api::auth_identity::AuthenticatedIdentity {
    use axum::{Extension, middleware, routing::get};
    use cuenora_api::auth_identity::{AuthenticatedIdentity, require_identity};
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
    let router = Router::new()
        .route(
            "/identity",
            get(
                move |Extension(id): Extension<AuthenticatedIdentity>| async move {
                    tx.lock().unwrap().take().unwrap().send(id).ok();
                },
            ),
        )
        .route_layer(middleware::from_fn_with_state(
            Some(pool.clone()),
            require_identity,
        ));
    assert_eq!(
        router
            .oneshot(
                Request::builder()
                    .uri("/identity")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    rx.await.unwrap()
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn generation_preserves_postponed_and_skipped_slots_and_next_month_anchor() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let identity = authenticated_identity(&pool, &token).await;
    let router = app(Some(pool.clone()));
    for operation in ["postpone", "skip"] {
        let id = occurrence(&pool, &owner).await;
        let rule: String =
            sqlx::query_scalar("SELECT rule_id::text FROM recurring_occurrences WHERE id=$1::uuid")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        sqlx::query("UPDATE recurring_rules SET state='active',paused_on=NULL,ends_on='2024-02-29' WHERE id=$1::uuid").bind(&rule).execute(&pool).await.unwrap();
        let body = if operation == "skip" {
            json!({})
        } else {
            json!({"due_on":"2024-02-05"})
        };
        let result = action(&router, &token, &id, operation, operation, body).await;
        assert_eq!(result.0, StatusCode::OK);
        let original: Value =
            sqlx::query_scalar("SELECT to_jsonb(o) FROM recurring_occurrences o WHERE id=$1::uuid")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        for expected in [1, 0] {
            assert_eq!(
                cuenora_api::recurring_rules::generate_due(&pool, &identity, &rule, || {
                    "2024-02-29T12:00:00Z".parse().unwrap()
                })
                .await
                .unwrap(),
                expected
            );
            let after: Value = sqlx::query_scalar(
                "SELECT to_jsonb(o) FROM recurring_occurrences o WHERE id=$1::uuid",
            )
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(after, original);
        }
        let slots:Vec<String>=sqlx::query_scalar("SELECT to_char(scheduled_on,'YYYY-MM-DD') FROM recurring_occurrences WHERE rule_id=$1::uuid ORDER BY scheduled_on").bind(&rule).fetch_all(&pool).await.unwrap();
        assert_eq!(slots, ["2024-01-31", "2024-02-29"]);
    }
    assert_no_money(&pool, &owner).await;
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn bounded_json_and_archived_planned_accounts_do_not_bypass_validation() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    let account:String=sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency,archived_at) VALUES($1::uuid,'Synthetic','normal','SEK',statement_timestamp()) RETURNING id::text").bind(&owner).fetch_one(&pool).await.unwrap();
    sqlx::query("UPDATE recurring_occurrences SET account_id=$2::uuid WHERE id=$1::uuid")
        .bind(&id)
        .bind(&account)
        .execute(&pool)
        .await
        .unwrap();
    for operation in ["skip", "postpone"] {
        let path = format!("/v1/recurring-occurrences/{id}/{operation}");
        let before = snapshot(&pool).await;
        error(
            request(
                &router,
                Some(&token),
                &path,
                &["large"],
                &format!("{{\"extra\":\"{}\"}}", "x".repeat(8192)),
            )
            .await,
            StatusCode::PAYLOAD_TOO_LARGE,
            "invalid_request",
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&path)
                    .header("authorization", format!("Bearer {token}"))
                    .header("idempotency-key", "media")
                    .header("content-type", "text/plain")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(snapshot(&pool).await, before);
    }
    assert_eq!(
        action(
            &router,
            &token,
            &id,
            "postpone",
            "archive-postpone",
            json!({"due_on":"2024-02-05"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        action(&router, &token, &id, "skip", "archive-skip", json!({}))
            .await
            .0,
        StatusCode::OK
    );
    assert_no_money(&pool, &owner).await;
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
async fn cancellation_before_and_after_planning_update_preserves_all_rows() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    for operation in ["skip", "postpone"] {
        for updated in [false, true] {
            let mut lock = pool.begin().await.unwrap();
            if updated {
                sqlx::query("SELECT pg_advisory_xact_lock(871111)")
                    .execute(&mut *lock)
                    .await
                    .unwrap();
                sqlx::raw_sql("CREATE FUNCTION probe_block_action() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(871111); RETURN NEW; END $$; CREATE TRIGGER probe_block_action BEFORE INSERT ON recurring_occurrence_action_keys FOR EACH ROW EXECUTE FUNCTION probe_block_action();").execute(&pool).await.unwrap();
            } else {
                sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
                    .bind(&owner)
                    .execute(&mut *lock)
                    .await
                    .unwrap();
            }
            let before = snapshot(&pool).await;
            let (r, t, i) = (router.clone(), token.clone(), id.clone());
            let handle = tokio::spawn(async move {
                action(
                    &r,
                    &t,
                    &i,
                    operation,
                    "cancelled",
                    if operation == "skip" {
                        json!({})
                    } else {
                        json!({"due_on":"2024-02-05"})
                    },
                )
                .await
            });
            wait_for_lock(
                &pool,
                if updated {
                    "INSERT INTO recurring_occurrence_action_keys"
                } else {
                    "SELECT email_verified_at"
                },
            )
            .await;
            handle.abort();
            assert!(handle.await.unwrap_err().is_cancelled());
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            lock.rollback().await.unwrap();
            wait_for_idle(&pool).await;
            assert_eq!(snapshot(&pool).await, before);
            if updated {
                sqlx::raw_sql("DROP TRIGGER probe_block_action ON recurring_occurrence_action_keys; DROP FUNCTION probe_block_action();").execute(&pool).await.unwrap();
            }
        }
    }
    assert_eq!(
        action(
            &router,
            &token,
            &id,
            "postpone",
            "cancelled",
            json!({"due_on":"2024-02-05"})
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn owner_and_due_date_are_revalidated_after_observed_lock_wait() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    for revoke in [false, true] {
        let mut lock = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
            .bind(&owner)
            .execute(&mut *lock)
            .await
            .unwrap();
        if revoke {
            sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
                .bind(&owner)
                .execute(&mut *lock)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE recurring_occurrences SET due_on='2024-03-05' WHERE id=$1::uuid")
                .bind(&id)
                .execute(&mut *lock)
                .await
                .unwrap();
        }
        let (r, t, i) = (router.clone(), token.clone(), id.clone());
        let handle = tokio::spawn(async move {
            action(
                &r,
                &t,
                &i,
                "postpone",
                "waiting",
                json!({"due_on":"2024-02-05"}),
            )
            .await
        });
        wait_for_lock(&pool, "SELECT email_verified_at").await;
        lock.commit().await.unwrap();
        let before = snapshot(&pool).await;
        error(
            handle.await.unwrap(),
            if revoke {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            },
            if revoke {
                "email_not_verified"
            } else {
                "invalid_request"
            },
        );
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn ordered_skip_and_postpone_queues_preserve_terminal_state_and_replay() {
    let pool = common::pool().await;
    for first_skip in [true, false] {
        let (owner, token) = seed(&pool, true, None).await;
        let id = occurrence(&pool, &owner).await;
        let router = app(Some(pool.clone()));
        let mut gate = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
            .bind(&owner)
            .execute(&mut *gate)
            .await
            .unwrap();
        let (r, t, i) = (router.clone(), token.clone(), id.clone());
        let a = tokio::spawn(async move {
            action(
                &r,
                &t,
                &i,
                if first_skip { "skip" } else { "postpone" },
                "queue-a",
                if first_skip {
                    json!({})
                } else {
                    json!({"due_on":"2024-04-01"})
                },
            )
            .await
        });
        wait_for_lock(&pool, "SELECT email_verified_at").await;
        let (r, t, i) = (router.clone(), token.clone(), id.clone());
        let z = tokio::spawn(async move {
            action(
                &r,
                &t,
                &i,
                if first_skip { "postpone" } else { "skip" },
                "queue-z",
                if first_skip {
                    json!({"due_on":"2024-05-01"})
                } else {
                    json!({})
                },
            )
            .await
        });
        for _ in 0..100 {
            let n:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'SELECT email_verified_at%'").fetch_one(&pool).await.unwrap();
            if n >= 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'SELECT email_verified_at%'").fetch_one(&pool).await.unwrap();
        assert_eq!(n, 2);
        gate.commit().await.unwrap();
        let x = a.await.unwrap();
        let y = z.await.unwrap();
        assert_eq!(x.0, StatusCode::OK);
        if first_skip {
            assert_eq!(y.0, StatusCode::CONFLICT);
            assert_eq!(y.1["code"], "occurrence_state_conflict");
        } else {
            assert_eq!(y.0, StatusCode::OK);
            assert_eq!(y.1["due_on"], "2024-04-01");
        }
        let before = snapshot(&pool).await;
        assert_eq!(
            action(
                &router,
                &token,
                &id,
                if first_skip { "skip" } else { "postpone" },
                "queue-a",
                if first_skip {
                    json!({})
                } else {
                    json!({"due_on":"2024-04-01"})
                }
            )
            .await,
            x
        );
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL harness only"]
async fn competing_forward_requests_use_locked_current_due_date() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true, None).await;
    let id = occurrence(&pool, &owner).await;
    let router = app(Some(pool.clone()));
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
        .bind(&owner)
        .execute(&mut *gate)
        .await
        .unwrap();
    let (r, t, i) = (router.clone(), token.clone(), id.clone());
    let first = tokio::spawn(async move {
        action(
            &r,
            &t,
            &i,
            "postpone",
            "forward-a",
            json!({"due_on":"2024-06-01"}),
        )
        .await
    });
    wait_for_lock(&pool, "SELECT email_verified_at").await;
    let (r, t, i) = (router.clone(), token.clone(), id.clone());
    let second = tokio::spawn(async move {
        action(
            &r,
            &t,
            &i,
            "postpone",
            "forward-b",
            json!({"due_on":"2024-05-01"}),
        )
        .await
    });
    let mut both = false;
    for _ in 0..100 {
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'SELECT email_verified_at%'").fetch_one(&pool).await.unwrap();
        if n == 2 {
            both = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(both);
    gate.commit().await.unwrap();
    assert_eq!(first.await.unwrap().0, StatusCode::OK);
    error(
        second.await.unwrap(),
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    let due: String = sqlx::query_scalar(
        "SELECT to_char(due_on,'YYYY-MM-DD') FROM recurring_occurrences WHERE id=$1::uuid",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(due, "2024-06-01");
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM recurring_occurrence_action_keys WHERE user_id=$1::uuid",
    )
    .bind(&owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let stable = snapshot(&pool).await;
    error(
        action(
            &router,
            &token,
            &id,
            "postpone",
            "forward-b",
            json!({"due_on":"2024-05-01"}),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    assert_eq!(snapshot(&pool).await, stable);
}
