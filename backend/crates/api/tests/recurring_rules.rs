//! Public lifecycle acceptance; database cases run only in disposable PostgreSQL.
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
    router: Router,
    token: Option<&str>,
    method: &str,
    path: &str,
    body: &str,
    key: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    if let Some(key) = key {
        req = req.header("idempotency-key", key);
    }
    let response = router
        .oneshot(req.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
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
async fn send(
    router: &Router,
    token: &str,
    method: &str,
    suffix: &str,
    body: Value,
) -> (StatusCode, Value) {
    request(
        router.clone(),
        Some(token),
        method,
        &format!("/v1/recurring-rules{suffix}"),
        &body.to_string(),
        Some("rule-create"),
    )
    .await
}
fn terms() -> Value {
    json!({"amount":"100.00","currency":"SEK","cadence":"monthly","starts_on":"2024-01-31","time_zone":"Europe/Stockholm"})
}
async fn seed(pool: &PgPool, verified: bool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let user: String = sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at) VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END) RETURNING id::text").bind(email).bind(verified).fetch_one(pool).await.unwrap();
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
async fn create(router: &Router, token: &str, body: Value) -> Value {
    let reply = send(router, token, "POST", "", body).await;
    assert_eq!(reply.0, StatusCode::CREATED, "{}", reply.1);
    reply.1
}
async fn account(pool: &PgPool, owner: &str, currency: &str, archived: bool) -> String {
    sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency,archived_at) VALUES($1::uuid,'Planning','normal',$2,CASE WHEN $3 THEN statement_timestamp() END) RETURNING id::text").bind(owner).bind(currency).bind(archived).fetch_one(pool).await.unwrap()
}
async fn snapshot(pool: &PgPool, financial_only: bool) -> String {
    let data: String = sqlx::query_scalar("SELECT md5(jsonb_build_array(
        (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),
        (SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id),'[]') FROM account_movements m),
        (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM transaction_audit a),
        (SELECT coalesce(jsonb_agg(to_jsonb(o) ORDER BY id),'[]') FROM recurring_occurrences o),
        CASE WHEN $1 THEN '[]'::jsonb ELSE (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id),'[]') FROM recurring_rules r) END,
        CASE WHEN $1 THEN '[]'::jsonb ELSE (SELECT coalesce(jsonb_agg(to_jsonb(k) ORDER BY user_id,key_hash),'[]') FROM recurring_rule_creation_keys k) END)::text)")
        .bind(financial_only).fetch_one(pool).await.unwrap();
    format!("{}:{data}", common::snapshot(pool).await)
}
async fn reject(
    router: &Router,
    token: &str,
    method: &str,
    suffix: &str,
    body: Value,
    pool: &PgPool,
    status: StatusCode,
) {
    let before = snapshot(pool, false).await;
    assert_eq!(send(router, token, method, suffix, body).await.0, status);
    assert_eq!(snapshot(pool, false).await, before);
}
#[tokio::test]
async fn rules_require_bearer_before_payload_or_storage() {
    for (method, path) in [
        ("POST", "/v1/recurring-rules"),
        (
            "PATCH",
            "/v1/recurring-rules/00000000-0000-0000-0000-000000000001",
        ),
        (
            "POST",
            "/v1/recurring-rules/00000000-0000-0000-0000-000000000001/pause",
        ),
        (
            "POST",
            "/v1/recurring-rules/00000000-0000-0000-0000-000000000001/resume",
        ),
    ] {
        assert_eq!(
            request(app(None), None, method, path, "{", None).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn creation_replays_original_terms_without_financial_effects() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool, true).await;
    let initial = create(&router, &token, terms()).await;
    let id = initial["id"].as_str().unwrap();
    assert_eq!(initial["state"], "active");
    assert!(initial["account_id"].is_null());
    assert_eq!(initial["amount"], "100.00");
    assert!(initial.get("user_id").is_none());
    assert_eq!(
        send(
            &router,
            &token,
            "PATCH",
            &format!("/{id}"),
            json!({"amount":"120"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(&router, &token, "POST", &format!("/{id}/pause"), json!({}))
            .await
            .0,
        StatusCode::OK
    );
    let stable = snapshot(&pool, false).await;
    let mut equivalent = terms();
    equivalent["amount"] = json!("100");
    assert_eq!(create(&router, &token, equivalent).await, initial);
    assert_eq!(snapshot(&pool, false).await, stable);
    let mut conflict = terms();
    conflict["amount"] = json!("101");
    reject(
        &router,
        &token,
        "POST",
        "",
        conflict,
        &pool,
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(snapshot(&pool, true).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn edits_preserve_occurrences_and_distinguish_absent_from_null() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let acc = account(&pool, &owner, "SEK", false).await;
    let category: String =
        sqlx::query_scalar("SELECT id::text FROM categories WHERE kind='expense' LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    let mut body = terms();
    body["account_id"] = json!(acc);
    body["category_id"] = json!(category);
    body["description"] = json!("Original");
    body["ends_on"] = json!("2030-12-31");
    let rule = create(&router, &token, body).await;
    let id = rule["id"].as_str().unwrap();
    let path = format!("/{id}");
    sqlx::query("INSERT INTO recurring_occurrences(rule_id,user_id,scheduled_on,due_on,amount_minor,currency) VALUES($1::uuid,$2::uuid,'2024-01-31','2024-01-31',10000,'SEK')").bind(id).bind(&owner).execute(&pool).await.unwrap();
    let before = snapshot(&pool, true).await;
    let reply = send(&router, &token, "PATCH", &path, json!({"amount":"120.00"})).await;
    assert_eq!(reply.0, StatusCode::OK);
    assert_eq!(reply.1["account_id"], acc);
    assert_eq!(reply.1["description"], "Original");
    assert!(reply.1["generated_through"].is_null());
    let cleared = send(
        &router,
        &token,
        "PATCH",
        &path,
        json!({"account_id":null,"category_id":null,"description":null,"ends_on":null}),
    )
    .await;
    assert_eq!(cleared.0, StatusCode::OK);
    for field in ["account_id", "category_id", "description", "ends_on"] {
        assert!(cleared.1[field].is_null());
    }
    let stable = snapshot(&pool, false).await;
    assert_eq!(
        send(&router, &token, "PATCH", &path, json!({"amount":"120"}))
            .await
            .1,
        cleared.1
    );
    assert_eq!(snapshot(&pool, false).await, stable);
    assert_eq!(snapshot(&pool, true).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn calendar_edits_advance_only_coverage_and_preserve_anchor_and_state() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let rule = create(&router, &token, terms()).await;
    let id = rule["id"].as_str().unwrap();
    let path = format!("/{id}");
    let paused = send(&router, &token, "POST", &format!("/{id}/pause"), json!({})).await;
    let today = paused.1["paused_on"].as_str().unwrap();
    let changed = send(
        &router,
        &token,
        "PATCH",
        &path,
        json!({"cadence":"quarterly"}),
    )
    .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert_eq!(changed.1["generated_through"], today);
    assert_eq!(changed.1["starts_on"], "2024-01-31");
    assert_eq!(changed.1["state"], "paused");
    sqlx::query("UPDATE recurring_rules SET generated_through='9999-12-30' WHERE id=$1::uuid")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let changed = send(
        &router,
        &token,
        "PATCH",
        &path,
        json!({"starts_on":"2020-02-29","time_zone":"Pacific/Kiritimati"}),
    )
    .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert_eq!(changed.1["generated_through"], "9999-12-30");
    assert_eq!(changed.1["starts_on"], "2020-02-29");
    let stable = snapshot(&pool, false).await;
    assert_eq!(
        send(
            &router,
            &token,
            "PATCH",
            &path,
            json!({"starts_on":"2020-02-29","time_zone":"Pacific/Kiritimati"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(snapshot(&pool, false).await, stable);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn pause_resume_unions_exclusions_and_never_touches_existing_items() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let rule = create(&router, &token, terms()).await;
    let id = rule["id"].as_str().unwrap();
    sqlx::query("INSERT INTO recurring_occurrences(rule_id,user_id,scheduled_on,due_on,amount_minor,currency) VALUES($1::uuid,$2::uuid,'2024-02-29','2024-03-05',10000,'SEK')").bind(id).bind(owner).execute(&pool).await.unwrap();
    let before = snapshot(&pool, true).await;
    for action in ["pause", "pause", "resume", "resume"] {
        let path = format!("/{id}/{action}");
        let first = send(&router, &token, "POST", &path, json!({})).await;
        assert_eq!(first.0, StatusCode::OK);
        let stable = snapshot(&pool, false).await;
        assert_eq!(
            send(&router, &token, "POST", &path, json!({})).await.1,
            first.1
        );
        assert_eq!(snapshot(&pool, false).await, stable);
    }
    sqlx::query("UPDATE recurring_rules SET state='paused',paused_on='2024-02-01',excluded_dates=datemultirange(daterange('2023-02-01','2023-04-01','[)')) WHERE id=$1::uuid").bind(id).execute(&pool).await.unwrap();
    let resumed = send(&router, &token, "POST", &format!("/{id}/resume"), json!({})).await;
    assert_eq!(resumed.0, StatusCode::OK);
    assert_eq!(resumed.1["starts_on"], "2024-01-31");
    assert!(resumed.1["paused_on"].is_null());
    let valid:bool=sqlx::query_scalar("SELECT excluded_dates @> '2023-02-28'::date AND excluded_dates @> '2024-02-29'::date AND NOT excluded_dates @> (statement_timestamp() AT TIME ZONE time_zone)::date AND generated_through IS NULL FROM recurring_rules WHERE id=$1::uuid").bind(id).fetch_one(&pool).await.unwrap();
    assert!(valid);
    assert_eq!(snapshot(&pool, true).await, before);
    sqlx::query(
        "UPDATE recurring_rules SET state='paused',paused_on='9999-12-31' WHERE id=$1::uuid",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    reject(
        &router,
        &token,
        "POST",
        &format!("/{id}/resume"),
        json!({}),
        &pool,
        StatusCode::CONFLICT,
    )
    .await;
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn ownership_verification_and_account_selection_are_atomic() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (foreign, other) = seed(&pool, true).await;
    let (_, unverified) = seed(&pool, false).await;
    let router = app(Some(pool.clone()));
    let archived = account(&pool, &owner, "SEK", true).await;
    let mismatch = account(&pool, &owner, "USD", false).await;
    let foreign = account(&pool, &foreign, "SEK", false).await;
    for (id, status) in [
        (&archived, StatusCode::CONFLICT),
        (&mismatch, StatusCode::BAD_REQUEST),
        (&foreign, StatusCode::NOT_FOUND),
    ] {
        let mut body = terms();
        body["account_id"] = json!(id);
        reject(&router, &token, "POST", "", body, &pool, status).await;
    }
    let active = account(&pool, &owner, "SEK", false).await;
    let mut body = terms();
    body["account_id"] = json!(active);
    let rule = create(&router, &token, body).await;
    let id = rule["id"].as_str().unwrap();
    let path = format!("/{id}");
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(&active)
        .execute(&pool)
        .await
        .unwrap();
    for (method, suffix, body) in [
        ("PATCH", path.clone(), json!({"description":"Retained"})),
        ("POST", format!("/{id}/pause"), json!({})),
        ("POST", format!("/{id}/resume"), json!({})),
    ] {
        let reply = send(&router, &token, method, &suffix, body.clone()).await;
        assert_eq!(reply.0, StatusCode::OK);
        assert_eq!(reply.1["account_id"], active);
        reject(
            &router,
            &other,
            method,
            &suffix,
            body.clone(),
            &pool,
            StatusCode::NOT_FOUND,
        )
        .await;
        reject(
            &router,
            &unverified,
            method,
            &suffix,
            body,
            &pool,
            StatusCode::FORBIDDEN,
        )
        .await;
    }
    reject(
        &router,
        &token,
        "PATCH",
        &path,
        json!({"account_id":archived,"amount":"333"}),
        &pool,
        StatusCode::CONFLICT,
    )
    .await;
    reject(
        &router,
        &unverified,
        "POST",
        "",
        terms(),
        &pool,
        StatusCode::FORBIDDEN,
    )
    .await;
    // Creating for another owner with the same key is independent.
    assert_eq!(create(&router, &other, terms()).await["state"], "active");
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn strict_inputs_and_boundaries_reject_without_mutation() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let rule = create(&router, &token, terms()).await;
    let id = rule["id"].as_str().unwrap();
    let path = format!("/{id}");
    for body in [
        json!({}),
        json!({"amount":null}),
        json!({"amount":"0"}),
        json!({"amount":"-1"}),
        json!({"amount":1.2}),
        json!({"amount":"0.001"}),
        json!({"currency":"GBP"}),
        json!({"cadence":"daily"}),
        json!({"starts_on":"2023-02-29"}),
        json!({"ends_on":"0001-01-01"}),
        json!({"starts_on":"10000-01-01"}),
        json!({"time_zone":"+01:00"}),
        json!({"time_zone":null}),
        json!({"account_id":"bad"}),
        json!({"category_id":"bad"}),
        json!({"description":"x".repeat(1001)}),
        json!({"description":"a\nb"}),
        json!({"state":"paused"}),
        json!({"generated_through":"2024-01-01"}),
        json!({"user_id":"forged"}),
    ] {
        reject(
            &router,
            &token,
            "PATCH",
            &path,
            body,
            &pool,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    let category: String =
        sqlx::query_scalar("SELECT id::text FROM categories WHERE kind='income' LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    reject(
        &router,
        &token,
        "PATCH",
        &path,
        json!({"category_id":category}),
        &pool,
        StatusCode::BAD_REQUEST,
    )
    .await;
    for suffix in [format!("/{id}/pause"), format!("/{id}/resume")] {
        reject(
            &router,
            &token,
            "POST",
            &suffix,
            json!({"date":"2024-01-01"}),
            &pool,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    let before = snapshot(&pool, false).await;
    for raw in [r#"{"amount":"1","amount":"2"}"#, "null", "{"] {
        assert_eq!(
            request(
                router.clone(),
                Some(&token),
                "PATCH",
                &format!("/v1/recurring-rules{path}"),
                raw,
                None
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(
            router.clone(),
            Some(&token),
            "POST",
            "/v1/recurring-rules",
            &terms().to_string(),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &router,
            &token,
            "PATCH",
            &format!("{path}?unexpected=1"),
            json!({"amount":"1"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(snapshot(&pool, false).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn concurrent_creation_and_pause_assignments_serialize() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let (a, b) = tokio::join!(
        send(&router, &token, "POST", "", terms()),
        send(&router, &token, "POST", "", terms())
    );
    assert_eq!(a.0, StatusCode::CREATED);
    assert_eq!(a, b);
    let id = a.1["id"].as_str().unwrap();
    let path = format!("/{id}/pause");
    let (a, b) = tokio::join!(
        send(&router, &token, "POST", &path, json!({})),
        send(&router, &token, "POST", &path, json!({}))
    );
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(a, b);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM recurring_rules WHERE user_id=$1::uuid")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn supported_terms_and_transport_limits_follow_the_contract() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    for (index, cadence) in ["weekly", "monthly", "quarterly", "annual"]
        .into_iter()
        .enumerate()
    {
        let mut body = terms();
        body["cadence"] = json!(cadence);
        body["currency"] = json!(["SEK", "EUR", "USD", "SEK"][index]);
        body["starts_on"] = json!("9999-12-31");
        body["ends_on"] = json!("9999-12-31");
        let reply = request(
            router.clone(),
            Some(&token),
            "POST",
            "/v1/recurring-rules",
            &body.to_string(),
            Some(&format!("supported-{index}")),
        )
        .await;
        assert_eq!(reply.0, StatusCode::CREATED);
        assert_eq!(reply.1["cadence"], cadence);
        assert_eq!(reply.1["currency"], body["currency"]);
    }
    let before = snapshot(&pool, false).await;
    let mut req = Request::builder()
        .method("POST")
        .uri("/v1/recurring-rules")
        .header("authorization", format!("Bearer {token}"));
    let missing_media = router
        .clone()
        .oneshot(req.body(Body::from(terms().to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(missing_media.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    req = Request::builder()
        .method("POST")
        .uri("/v1/recurring-rules")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");
    let large = router
        .clone()
        .oneshot(req.body(Body::from("x".repeat(8193))).unwrap())
        .await
        .unwrap();
    assert_eq!(large.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(snapshot(&pool, false).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn failed_retry_ledger_insert_rolls_back_rule_creation() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    sqlx::query("CREATE FUNCTION reject_rule_key_probe() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic probe failure'; END; $$").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_rule_key_probe BEFORE INSERT ON recurring_rule_creation_keys FOR EACH ROW EXECUTE FUNCTION reject_rule_key_probe()").execute(&pool).await.unwrap();
    let before = snapshot(&pool, false).await;
    let rejected = send(&router, &token, "POST", "", terms()).await;
    sqlx::query("DROP TRIGGER reject_rule_key_probe ON recurring_rule_creation_keys")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_rule_key_probe()")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(rejected.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(rejected.1["code"], "recurrence_unavailable");
    assert_eq!(snapshot(&pool, false).await, before);
    assert_eq!(create(&router, &token, terms()).await["state"], "active");
}
