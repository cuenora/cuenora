//! Owned account metadata transitions through public HTTP.
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

async fn request(
    router: Router,
    token: Option<&str>,
    method: &str,
    uri: &str,
    body: &str,
    media: Option<&str>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(media) = media {
        req = req.header(header::CONTENT_TYPE, media);
    }
    req = req.header("idempotency-key", "management-create");
    let response = router
        .oneshot(req.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    if !headers.contains_key(header::CACHE_CONTROL) {
        assert_eq!(
            status,
            StatusCode::OK,
            "authorized management PATCH must be available"
        );
    }
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    let data: Value = serde_json::from_slice(&bytes).unwrap();
    if data.get("code").is_some() {
        assert_eq!(
            data["request_id"],
            headers["x-request-id"].to_str().unwrap()
        );
        assert_eq!(data["details"], Value::Null);
    }
    (status, headers, data)
}

async fn patch(
    router: Router,
    token: &str,
    id: &str,
    body: Value,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    request(
        router,
        Some(token),
        "PATCH",
        &format!("/v1/accounts/{id}"),
        &body.to_string(),
        Some("application/json"),
    )
    .await
}

async fn create(router: Router, token: &str) -> Value {
    let reply = request(
        router,
        Some(token),
        "POST",
        "/v1/accounts",
        r#"{"name":"Original","account_type":"normal","currency":"SEK"}"#,
        Some("application/json"),
    )
    .await;
    assert_eq!(reply.0, StatusCode::CREATED);
    reply.2
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

async fn snapshot(pool: &PgPool) -> String {
    let remaining: String = sqlx::query_scalar(
        "SELECT jsonb_build_array(
            (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s),
            (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM session_access_tokens t),
            (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM session_refresh_tokens t),
            (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM accounts a),
            (SELECT coalesce(jsonb_agg(to_jsonb(k) ORDER BY user_id), '[]') FROM account_creation_keys k),
            (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM transactions t),
            (SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id), '[]') FROM account_movements m),
            (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM transaction_audit a)
        )::text",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    format!("{}:{remaining}", common::snapshot(pool).await)
}

fn assert_error(
    reply: &(StatusCode, axum::http::HeaderMap, Value),
    status: StatusCode,
    code: &str,
    message: &str,
) {
    assert_eq!(reply.0, status);
    assert_eq!(reply.2["code"], code);
    assert_eq!(reply.2["message"], message);
}

async fn post(pool: &PgPool, owner: &str, id: &str, units: i64) -> String {
    let event: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,account_id,currency,amount_minor,occurred_on) VALUES($1::uuid,'opening_balance',$2::uuid,'SEK',$3,'2025-01-01') RETURNING id::text")
        .bind(owner).bind(id).bind(units).fetch_one(pool).await.unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,currency,signed_amount_minor,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,'SEK',$4,'2025-01-01')")
        .bind(owner).bind(&event).bind(id).bind(units).execute(pool).await.unwrap();
    event
}

async fn timestamps(pool: &PgPool, id: &str) -> String {
    sqlx::query_scalar("SELECT jsonb_build_array(created_at,updated_at,archived_at)::text FROM accounts WHERE id=$1::uuid").bind(id).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn management_without_bearer_rejects_before_storage() {
    let reply = request(
        app(None),
        None,
        "PATCH",
        "/v1/accounts/00000000-0000-0000-0000-000000000001",
        r#"{"name":"New"}"#,
        Some("application/json"),
    )
    .await;
    assert_error(
        &reply,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
        "The access token is invalid.",
    );
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn owned_rename_archive_reactivate_and_retries_preserve_financial_state() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let initial = create(router.clone(), &token).await;
    let id = initial["id"].as_str().unwrap();
    let before = timestamps(&pool, id).await;
    let renamed = patch(router.clone(), &token, id, json!({"name":"  My  雪  "})).await;
    assert_eq!(renamed.0, StatusCode::OK);
    assert_eq!(renamed.2["name"], "My  雪");
    assert_eq!(renamed.2.as_object().unwrap().len(), 5);
    assert_eq!(renamed.2["currency"], "SEK");
    assert_ne!(timestamps(&pool, id).await, before);
    for state in ["archived", "active", "archived"] {
        let changed = patch(router.clone(), &token, id, json!({"status":state})).await;
        assert_eq!(changed.0, StatusCode::OK);
        assert_eq!(changed.2["status"], state);
        let stable = snapshot(&pool).await;
        assert_eq!(
            patch(
                router.clone(),
                &token,
                id,
                json!({"status":state,"name":" My  雪 "})
            )
            .await
            .2,
            changed.2
        );
        assert_eq!(snapshot(&pool).await, stable);
    }
    // Metadata names remain editable while archived; this does not post money.
    assert_eq!(
        patch(router.clone(), &token, id, json!({"name":"Archived label"}))
            .await
            .0,
        StatusCode::OK
    );
    let listed = request(
        router.clone(),
        Some(&token),
        "GET",
        "/v1/accounts",
        "",
        None,
    )
    .await;
    assert_eq!(listed.2["accounts"][0]["status"], "archived");
    assert_eq!(listed.2["accounts"][0]["balance"], "0.00");
    let prior = snapshot(&pool).await;
    assert_eq!(create(router, &token).await, initial);
    assert_eq!(snapshot(&pool).await, prior);
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE user_id=$1::uuid),(SELECT count(*) FROM account_movements WHERE user_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=$1::uuid)").bind(owner).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn metadata_input_and_authority_rejections_leave_all_state_unchanged() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (_, other) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let account = create(router.clone(), &token).await;
    let id = account["id"].as_str().unwrap();
    let missing = "00000000-0000-0000-0000-000000000001";
    let before = snapshot(&pool).await;
    for target in [id, missing] {
        assert_error(
            &patch(router.clone(), &other, target, json!({"name":"Forbidden"})).await,
            StatusCode::NOT_FOUND,
            "not_found",
            "The resource was not found.",
        );
    }
    for body in [
        "{}".to_owned(),
        "null".into(),
        r#"{"name":null}"#.into(),
        r#"{"status":null}"#.into(),
        r#"{"name":"x","name":"y"}"#.into(),
        r#"{"status":"active","status":"archived"}"#.into(),
        r#"{"user_id":"forged"}"#.into(),
        r#"{"account_type":"savings"}"#.into(),
        r#"{"currency":"USD"}"#.into(),
        r#"{"balance":"0.00"}"#.into(),
        r#"{"status":"deleted"}"#.into(),
        json!({"name":"\t \n"}).to_string(),
        json!({"name":"a\nb"}).to_string(),
        json!({"name":"x".repeat(256)}).to_string(),
    ] {
        assert_error(
            &request(
                router.clone(),
                Some(&token),
                "PATCH",
                &format!("/v1/accounts/{id}"),
                &body,
                Some("application/json"),
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is invalid.",
        );
    }
    for uri in [
        format!("/v1/accounts/{id}?status=active"),
        "/v1/accounts/bad".into(),
    ] {
        assert_eq!(
            request(
                router.clone(),
                Some(&token),
                "PATCH",
                &uri,
                r#"{"status":"active"}"#,
                Some("application/json")
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
            "PATCH",
            &format!("/v1/accounts/{id}"),
            r#"{"name":"x"}"#,
            None
        )
        .await
        .0,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        request(
            router.clone(),
            Some(&token),
            "PATCH",
            &format!("/v1/accounts/{id}"),
            &json!({"name":"x".repeat(8192)}).to_string(),
            Some("application/json")
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(snapshot(&pool).await, before);
    // A valid multibyte name is bounded by Unicode scalars, not bytes.
    let unicode = "雪".repeat(255);
    assert_eq!(
        patch(router.clone(), &token, id, json!({"name":unicode}))
            .await
            .2["name"],
        unicode
    );
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let pending = snapshot(&pool).await;
    assert_error(
        &patch(router.clone(), &token, id, json!({"status":"archived"})).await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
        "Email verification is required.",
    );
    assert_eq!(snapshot(&pool).await, pending);
    sqlx::query("UPDATE session_access_tokens SET expires_at=statement_timestamp()-interval '1 second',created_at=statement_timestamp()-interval '10 minutes' WHERE user_id=$1::uuid").bind(owner).execute(&pool).await.unwrap();
    let expired = snapshot(&pool).await;
    assert_eq!(
        patch(router, &token, id, json!({"status":"active"}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(snapshot(&pool).await, expired);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn archive_requires_exact_zero_and_never_applies_a_partial_rename() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let account = create(router.clone(), &token).await;
    let id = account["id"].as_str().unwrap();
    // Exercise positive, negative and beyond-i64 exact totals without narrowing.
    for units in [1, -2, i64::MAX, i64::MAX] {
        post(&pool, &owner, id, units).await;
        let before = snapshot(&pool).await;
        assert_error(
            &patch(
                router.clone(),
                &token,
                id,
                json!({"name":"Must not apply","status":"archived"}),
            )
            .await,
            StatusCode::CONFLICT,
            "account_balance_not_zero",
            "Archive requires a zero account balance.",
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for units in [-i64::MAX, -i64::MAX, 1] {
        post(&pool, &owner, id, units).await;
    }
    let financial_before: String=sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM transactions t),(SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM account_movements m),(SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id) FROM account_creation_keys k))::text").fetch_one(&pool).await.unwrap();
    let archived = patch(
        router.clone(),
        &token,
        id,
        json!({"name":"Zero","status":"archived"}),
    )
    .await;
    assert_eq!(archived.0, StatusCode::OK);
    assert_eq!(archived.2["name"], "Zero");
    assert_eq!(archived.2["status"], "archived");
    let financial_after: String=sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM transactions t),(SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM account_movements m),(SELECT jsonb_agg(to_jsonb(k) ORDER BY user_id) FROM account_creation_keys k))::text").fetch_one(&pool).await.unwrap();
    assert_eq!(financial_after, financial_before);
    let listed = request(router, Some(&token), "GET", "/v1/accounts", "", None).await;
    assert_eq!(listed.2["accounts"][0]["balance"], "0.00");
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn posting_committed_before_archive_lock_is_included_in_zero_check() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let account = create(router.clone(), &token).await;
    let id = account["id"].as_str().unwrap().to_owned();
    let event = post(&pool, &owner, &id, 0).await;
    let mut pending = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,currency,signed_amount_minor,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,'SEK',10,'2025-01-01')")
        .bind(&owner).bind(event).bind(&id).execute(&mut *pending).await.unwrap();
    let (r, t, i) = (router.clone(), token.clone(), id.clone());
    let task = tokio::spawn(async move { patch(r, &t, &i, json!({"status":"archived"})).await });
    let mut waiting = false;
    for _ in 0..100 {
        waiting=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%FOR UPDATE%')").fetch_one(&pool).await.unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(waiting, "archive did not wait for pending posting");
    pending.commit().await.unwrap();
    let before = snapshot(&pool).await;
    assert_eq!(task.await.unwrap().0, StatusCode::CONFLICT);
    assert_eq!(snapshot(&pool).await, before);
    let listed = request(router, Some(&token), "GET", "/v1/accounts", "", None).await;
    assert_eq!(listed.2["accounts"][0]["status"], "active");
    assert_eq!(listed.2["accounts"][0]["balance"], "0.10");
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn concurrent_metadata_fields_and_duplicate_archive_do_not_lose_state() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let account = create(router.clone(), &token).await;
    let id = account["id"].as_str().unwrap();
    let (rename, archive) = tokio::join!(
        patch(router.clone(), &token, id, json!({"name":"Concurrent"})),
        patch(router.clone(), &token, id, json!({"status":"archived"}))
    );
    assert_eq!(rename.0, StatusCode::OK);
    assert_eq!(archive.0, StatusCode::OK);
    let before = snapshot(&pool).await;
    let (a, b) = tokio::join!(
        patch(router.clone(), &token, id, json!({"status":"archived"})),
        patch(router.clone(), &token, id, json!({"status":"archived"}))
    );
    assert_eq!(a.2, b.2);
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(snapshot(&pool).await, before);
    let listed = request(router, Some(&token), "GET", "/v1/accounts", "", None).await;
    assert_eq!(listed.2["accounts"][0]["name"], "Concurrent");
    assert_eq!(listed.2["accounts"][0]["status"], "archived");
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn update_failures_timeout_and_rollback_without_false_success() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let account = create(router.clone(), &token).await;
    let id = account["id"].as_str().unwrap();
    let before = snapshot(&pool).await;
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE accounts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    assert_error(
        &patch(router.clone(), &token, id, json!({"name":"Blocked"})).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "account_update_unavailable",
        "Account update is temporarily unavailable.",
    );
    lock.rollback().await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("CREATE FUNCTION fail_account_update() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic private failure'; END $$").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_account_update AFTER UPDATE ON accounts FOR EACH ROW EXECUTE FUNCTION fail_account_update()").execute(&pool).await.unwrap();
    assert_error(
        &patch(
            router,
            &token,
            id,
            json!({"name":"Must roll back","status":"archived"}),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
        "account_update_unavailable",
        "Account update is temporarily unavailable.",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn legacy_nonzero_restore_and_original_opening_replay() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    assert_eq!(
        request(
            router.clone(),
            Some(&token),
            "PATCH",
            "/v1/me",
            r#"{"time_zone":"Etc/UTC"}"#,
            Some("application/json")
        )
        .await
        .0,
        StatusCode::OK
    );
    let original_body = r#"{"name":"Original","account_type":"normal","currency":"SEK","opening_balance":{"amount":"5.00","occurred_on":"2024-01-01"}}"#;
    let initial = request(
        router.clone(),
        Some(&token),
        "POST",
        "/v1/accounts",
        original_body,
        Some("application/json"),
    )
    .await;
    assert_eq!(initial.0, StatusCode::CREATED);
    let id = initial.2["id"].as_str().unwrap();
    sqlx::query("UPDATE accounts SET archived_at=statement_timestamp() WHERE id=$1::uuid")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    assert_eq!(
        patch(router.clone(), &token, id, json!({"status":"archived"}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(
        patch(
            router.clone(),
            &token,
            id,
            json!({"name":"Archived rename"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let before = snapshot(&pool).await;
    assert_eq!(
        patch(
            router.clone(),
            &token,
            id,
            json!({"name":" Archived rename ","status":"archived"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(
        patch(router.clone(), &token, id, json!({"status":"active"}))
            .await
            .0,
        StatusCode::OK
    );
    let listed = request(
        router.clone(),
        Some(&token),
        "GET",
        "/v1/accounts",
        "",
        None,
    )
    .await;
    assert_eq!(listed.2["accounts"][0]["balance"], "5.00");
    assert_eq!(listed.2["accounts"][0]["status"], "active");
    let before = snapshot(&pool).await;
    assert_eq!(
        patch(
            router.clone(),
            &token,
            id,
            json!({"name":"Must not apply","status":"archived"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(snapshot(&pool).await, before);
    post(&pool, &owner, id, -500).await;
    sqlx::query("UPDATE transactions SET state='voided' WHERE user_id=$1::uuid")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        patch(router.clone(), &token, id, json!({"status":"archived"}))
            .await
            .0,
        StatusCode::OK
    );
    let before = snapshot(&pool).await;
    let replay = request(
        router.clone(),
        Some(&token),
        "POST",
        "/v1/accounts",
        original_body,
        Some("application/json"),
    )
    .await;
    assert_eq!(replay.0, StatusCode::CREATED);
    assert_eq!(replay.2, initial.2);
    assert_eq!(snapshot(&pool).await, before);
    let listed = request(router, Some(&token), "GET", "/v1/accounts", "", None).await;
    assert_eq!(listed.2["accounts"][0]["name"], "Archived rename");
    assert_eq!(listed.2["accounts"][0]["status"], "archived");
    assert_eq!(listed.2["accounts"][0]["balance"], "0.00");
    assert_eq!(snapshot(&pool).await, before);
}
