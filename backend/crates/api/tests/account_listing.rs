//! Owner-scoped account listing acceptance through the public HTTP route.
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
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
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

struct AccountFixture {
    id: String,
    name: String,
    account_type: String,
    currency: String,
    archived: bool,
    created_at: String,
}

fn fixture_id(group: u32, index: u32) -> String {
    format!("{group:08x}-0000-0000-0000-{index:012x}")
}

async fn add_account(pool: &PgPool, owner: &str, fixture: &AccountFixture) {
    sqlx::query("INSERT INTO accounts (id,user_id,name,account_type,currency,archived_at,created_at) VALUES ($1::uuid,$2::uuid,$3,$4,$5,CASE WHEN $6 THEN statement_timestamp() END,$7::timestamptz)")
        .bind(&fixture.id)
        .bind(owner)
        .bind(&fixture.name)
        .bind(&fixture.account_type)
        .bind(&fixture.currency)
        .bind(fixture.archived)
        .bind(&fixture.created_at)
        .execute(pool)
        .await
        .unwrap();
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

fn account_ids(reply: &Value) -> Vec<String> {
    reply["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|account| account["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn verified_owner_can_list_empty_accounts_without_a_profile_zone() {
    let pool = common::pool().await;
    let (user, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let before_missing_bearer = snapshot(&pool).await;
    assert_error(
        &send(router.clone(), None, "/v1/accounts", "").await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
        "The access token is invalid.",
    );
    assert_eq!(snapshot(&pool).await, before_missing_bearer);
    let before = snapshot(&pool).await;
    let reply = send(router.clone(), Some(&token), "/v1/accounts", "").await;
    assert_eq!(reply.0, StatusCode::OK);
    assert_eq!(reply.2, json!({"accounts": [], "next_cursor": null}));
    assert_eq!(reply.2.as_object().unwrap().len(), 2);
    assert_eq!(snapshot(&pool).await, before);

    let (_, pending) = seed(&pool, false).await;
    let before_pending_read = snapshot(&pool).await;
    assert_error(
        &send(router.clone(), Some(&pending), "/v1/accounts", "").await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
        "Email verification is required.",
    );
    assert_eq!(snapshot(&pool).await, before_pending_read);
    let expires_at = sqlx::query("UPDATE session_access_tokens SET created_at=statement_timestamp()-interval '10 minutes', expires_at=statement_timestamp()-interval '1 second' WHERE user_id=$1::uuid")
        .bind(&user)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(expires_at.rows_affected(), 1);
    let before_expired_read = snapshot(&pool).await;
    assert_error(
        &send(router.clone(), Some(&token), "/v1/accounts", "").await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
        "The access token is invalid.",
    );
    assert_eq!(snapshot(&pool).await, before_expired_read);

    let (revoked_user, revoked_token) = seed(&pool, true).await;
    sqlx::query("UPDATE user_sessions SET revoked_at=statement_timestamp() WHERE user_id=$1::uuid")
        .bind(&revoked_user)
        .execute(&pool)
        .await
        .unwrap();
    let before_revoked_read = snapshot(&pool).await;
    assert_error(
        &send(router, Some(&revoked_token), "/v1/accounts", "").await,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
        "The access token is invalid.",
    );
    assert_eq!(snapshot(&pool).await, before_revoked_read);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn listing_preserves_metadata_adds_zero_balances_and_isolates_each_owner() {
    let pool = common::pool().await;
    let (owner, owner_token) = seed(&pool, true).await;
    let (other, other_token) = seed(&pool, true).await;
    let fixtures = [
        AccountFixture {
            id: fixture_id(0x2000, 1),
            name: "  Legacy  ".into(),
            account_type: "normal".into(),
            currency: "SEK".into(),
            archived: false,
            created_at: "2025-01-01T00:00:00Z".into(),
        },
        AccountFixture {
            id: fixture_id(0x2000, 2),
            name: "Duplicate".into(),
            account_type: "savings".into(),
            currency: "EUR".into(),
            archived: true,
            created_at: "2025-01-02T00:00:00Z".into(),
        },
        AccountFixture {
            id: fixture_id(0x2000, 3),
            name: "Duplicate".into(),
            account_type: "normal".into(),
            currency: "USD".into(),
            archived: false,
            created_at: "2025-01-03T00:00:00Z".into(),
        },
    ];
    for fixture in &fixtures {
        add_account(&pool, &owner, fixture).await;
    }
    let foreign_id = fixture_id(0x2001, 1);
    add_account(
        &pool,
        &other,
        &AccountFixture {
            id: foreign_id.clone(),
            name: "Private".into(),
            account_type: "normal".into(),
            currency: "SEK".into(),
            archived: false,
            created_at: "2025-01-01T00:00:00Z".into(),
        },
    )
    .await;

    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    let listed = send(router.clone(), Some(&owner_token), "/v1/accounts", "").await;
    assert_eq!(listed.0, StatusCode::OK);
    assert_eq!(listed.2.as_object().unwrap().len(), 2);
    assert_eq!(listed.2["next_cursor"], Value::Null);
    let accounts = listed.2["accounts"].as_array().unwrap();
    assert_eq!(accounts.len(), 3);
    assert_eq!(
        account_ids(&listed.2),
        fixtures
            .iter()
            .map(|fixture| fixture.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        accounts[0],
        json!({
            "id": fixtures[0].id,
            "name": "  Legacy  ",
            "account_type": "normal",
            "currency": "SEK",
            "status": "active",
            "balance": "0.00"
        })
    );
    assert_eq!(accounts[1]["name"], "Duplicate");
    assert_eq!(accounts[1]["status"], "archived");
    assert_eq!(accounts[1]["account_type"], "savings");
    assert_eq!(accounts[2]["name"], "Duplicate");
    assert_eq!(accounts[2]["currency"], "USD");
    for account in accounts {
        assert_eq!(account.as_object().unwrap().len(), 6);
    }

    let other_list = send(router, Some(&other_token), "/v1/accounts", "").await;
    assert_eq!(other_list.0, StatusCode::OK);
    assert_eq!(account_ids(&other_list.2), vec![foreign_id]);
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn cursor_pages_are_stable_for_equal_creation_times_and_default_to_fifty() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let expected: Vec<String> = (1..=105).map(|n| fixture_id(0x3000, n)).collect();
    for (offset, id) in expected.iter().enumerate() {
        add_account(
            &pool,
            &owner,
            &AccountFixture {
                id: id.clone(),
                name: "Same timestamp".into(),
                account_type: if offset % 2 == 0 { "normal" } else { "savings" }.into(),
                currency: ["SEK", "EUR", "USD"][offset % 3].into(),
                archived: offset % 4 == 0,
                created_at: "2025-02-03T04:05:06Z".into(),
            },
        )
        .await;
    }
    let router = app(Some(pool.clone()));
    let before = snapshot(&pool).await;
    let mut cursor: Option<String> = None;
    let mut observed = Vec::new();
    for (start, end) in [(0, 50), (50, 100), (100, 105)] {
        let uri = cursor.as_ref().map_or_else(
            || "/v1/accounts".to_owned(),
            |cursor| format!("/v1/accounts?cursor={cursor}"),
        );
        let reply = send(router.clone(), Some(&token), &uri, "").await;
        assert_eq!(reply.0, StatusCode::OK);
        let ids = account_ids(&reply.2);
        assert_eq!(ids, expected[start..end]);
        observed.extend(ids);
        cursor = reply.2["next_cursor"].as_str().map(str::to_owned);
        if end == expected.len() {
            assert_eq!(cursor, None);
        } else {
            assert_eq!(cursor.as_deref(), Some(expected[end - 1].as_str()));
        }
    }
    assert_eq!(observed, expected);

    let first_hundred = send(router.clone(), Some(&token), "/v1/accounts?limit=100", "").await;
    assert_eq!(account_ids(&first_hundred.2), expected[..100]);
    assert_eq!(first_hundred.2["next_cursor"], expected[99]);
    let final_five = send(
        router,
        Some(&token),
        &format!("/v1/accounts?limit=100&cursor={}", expected[99]),
        "",
    )
    .await;
    assert_eq!(account_ids(&final_five.2), expected[100..]);
    assert_eq!(final_five.2["next_cursor"], Value::Null);
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn invalid_query_cursors_and_get_bodies_are_rejected_without_writes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (other, _) = seed(&pool, true).await;
    let own_id = fixture_id(0x4000, 1);
    let foreign_id = fixture_id(0x4001, 1);
    add_account(
        &pool,
        &owner,
        &AccountFixture {
            id: own_id,
            name: "Own".into(),
            account_type: "normal".into(),
            currency: "SEK".into(),
            archived: false,
            created_at: "2025-03-01T00:00:00Z".into(),
        },
    )
    .await;
    add_account(
        &pool,
        &other,
        &AccountFixture {
            id: foreign_id.clone(),
            name: "Foreign".into(),
            account_type: "normal".into(),
            currency: "SEK".into(),
            archived: false,
            created_at: "2025-03-01T00:00:00Z".into(),
        },
    )
    .await;
    let router = app(Some(pool.clone()));
    let invalid = [
        "/v1/accounts?limit=0",
        "/v1/accounts?limit=101",
        "/v1/accounts?limit=abc",
        "/v1/accounts?limit=1&limit=2",
        "/v1/accounts?owner_id=somebody-else",
        "/v1/accounts?cursor=not-a-uuid",
        "/v1/accounts?cursor=00000000-0000-0000-0000-000000004099",
    ];
    for uri in invalid {
        let before = snapshot(&pool).await;
        let reply = send(router.clone(), Some(&token), uri, "").await;
        assert_error(
            &reply,
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is invalid.",
        );
        assert_eq!(snapshot(&pool).await, before, "{uri}");
    }
    let foreign_uri = format!("/v1/accounts?cursor={foreign_id}");
    let before = snapshot(&pool).await;
    assert_error(
        &send(router.clone(), Some(&token), &foreign_uri, "").await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "The request is invalid.",
    );
    assert_eq!(snapshot(&pool).await, before);

    for body in ["{}", &format!(" {}", "x".repeat(8192))] {
        let before = snapshot(&pool).await;
        let reply = send(router.clone(), Some(&token), "/v1/accounts", body).await;
        if body.len() > 8192 {
            assert_error(
                &reply,
                StatusCode::PAYLOAD_TOO_LARGE,
                "invalid_request",
                "The request is invalid.",
            );
        } else {
            assert_error(
                &reply,
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request is invalid.",
            );
        }
        assert_eq!(snapshot(&pool).await, before);
    }
    let long_query = format!("/v1/accounts?limit=1&{}", "x".repeat(1024));
    let before = snapshot(&pool).await;
    assert_error(
        &send(router, Some(&token), &long_query, "").await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "The request is invalid.",
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn listing_storage_failure_is_safe_and_never_looks_like_an_empty_page() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let before = snapshot(&pool).await;
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE accounts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let reply = send(app(Some(pool.clone())), Some(&token), "/v1/accounts", "").await;
    assert_error(
        &reply,
        StatusCode::SERVICE_UNAVAILABLE,
        "account_listing_unavailable",
        "Account listing is temporarily unavailable.",
    );
    lock.rollback().await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn listing_uses_one_repeatable_read_snapshot_and_post_ignores_listing_query() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let old = fixture_id(0x9900, 1);
    add_account(
        &pool,
        &owner,
        &AccountFixture {
            id: old.clone(),
            name: "old".into(),
            account_type: "normal".into(),
            currency: "SEK".into(),
            archived: false,
            created_at: "2025-01-01T00:00:00Z".into(),
        },
    )
    .await;
    let event = post_movement(&pool, &owner, &old, "SEK", 10).await;
    let router = app(Some(pool.clone()));

    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE accounts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let token_copy = token.clone();
    let router_copy = router.clone();
    let read =
        tokio::spawn(async move { send(router_copy, Some(&token_copy), "/v1/accounts", "").await });
    let mut waiting = false;
    for _ in 0..100 {
        waiting = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1 FROM pg_stat_activity
                WHERE datname = current_database()
                  AND wait_event_type = 'Lock'
                  AND query LIKE 'SELECT a.id::text,%'
            )",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(waiting, "listing did not reach its locked page query");

    sqlx::query("UPDATE users SET email_verified_at = NULL WHERE id = $1::uuid")
        .bind(&owner)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let inserted = fixture_id(0x9900, 2);
    sqlx::query("INSERT INTO accounts (id,user_id,name,account_type,currency,created_at) VALUES ($1::uuid,$2::uuid,'new','savings','EUR','2024-01-01T00:00:00Z')")
        .bind(&inserted)
        .bind(&owner)
        .execute(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,currency,signed_amount_minor,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,'SEK',20,'2025-01-01')")
        .bind(&owner).bind(event).bind(&old).execute(&mut *blocker).await.unwrap();
    blocker.commit().await.unwrap();

    let after_concurrent_changes = snapshot(&pool).await;
    let read = read.await.unwrap();
    assert_eq!(read.0, StatusCode::OK);
    assert_eq!(account_ids(&read.2), vec![old.clone()]);
    assert_eq!(read.2["accounts"][0]["balance"], "0.10");
    assert_error(
        &send(router.clone(), Some(&token), "/v1/accounts", "").await,
        StatusCode::FORBIDDEN,
        "email_not_verified",
        "Email verification is required.",
    );
    assert_eq!(snapshot(&pool).await, after_concurrent_changes);

    sqlx::query("UPDATE users SET email_verified_at = statement_timestamp() WHERE id = $1::uuid")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    let before_cursor_checks = snapshot(&pool).await;
    let first = send(router.clone(), Some(&token), "/v1/accounts?limit=1", "").await;
    assert_eq!(account_ids(&first.2), vec![inserted.clone()]);
    assert_eq!(first.2["next_cursor"], inserted);
    let last = send(
        router.clone(),
        Some(&token),
        &format!("/v1/accounts?limit=1&cursor={inserted}"),
        "",
    )
    .await;
    assert_eq!(account_ids(&last.2), vec![old.clone()]);
    assert_eq!(last.2["accounts"][0]["balance"], "0.30");
    assert_eq!(last.2["next_cursor"], Value::Null);

    for query in [
        format!("cursor={old}&cursor={old}"),
        "cursor=&limit=1".into(),
        "limit=65536".into(),
    ] {
        assert_error(
            &send(
                router.clone(),
                Some(&token),
                &format!("/v1/accounts?{query}"),
                "",
            )
            .await,
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is invalid.",
        );
    }
    let foreign = send(
        router.clone(),
        Some(&token),
        &format!("/v1/accounts?cursor={}", fixture_id(0x9901, 1)),
        "",
    )
    .await;
    let malformed = send(router.clone(), Some(&token), "/v1/accounts?cursor=bad", "").await;
    assert_eq!(foreign.0, malformed.0);
    let mut foreign_body = foreign.2;
    let mut malformed_body = malformed.2;
    foreign_body["request_id"] = Value::Null;
    malformed_body["request_id"] = Value::Null;
    assert_eq!(foreign_body, malformed_body);
    assert_eq!(snapshot(&pool).await, before_cursor_checks);

    let uri = format!(
        "/v1/accounts?limit=0&limit=101&owner_id={}",
        "x".repeat(1100)
    );
    let response = app(Some(pool))
        .oneshot(
            Request::post(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("idempotency-key", "listing-query-post-compatibility")
                .body(Body::from(
                    r#"{"name":"Unchanged POST","account_type":"normal","currency":"SEK"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "listing query validation changed existing POST behavior"
    );
}

async fn post_movement(
    pool: &PgPool,
    owner: &str,
    account: &str,
    currency: &str,
    minor: i64,
) -> String {
    let event: String = sqlx::query_scalar("INSERT INTO transactions(user_id,kind,account_id,currency,amount_minor,occurred_on) VALUES($1::uuid,'opening_balance',$2::uuid,$3,$4,'2025-01-01') RETURNING id::text")
        .bind(owner).bind(account).bind(currency).bind(minor).fetch_one(pool).await.unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,currency,signed_amount_minor,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5,'2025-01-01')")
        .bind(owner).bind(&event).bind(account).bind(currency).bind(minor).execute(pool).await.unwrap();
    event
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn balances_sum_signed_postings_compensations_and_keep_currencies_and_owners_separate() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (other, other_token) = seed(&pool, true).await;
    let mut ids = Vec::new();
    for (index, currency) in ["SEK", "EUR", "USD", "SEK"].iter().enumerate() {
        let id = fixture_id(0xa000, index as u32);
        add_account(
            &pool,
            &owner,
            &AccountFixture {
                id: id.clone(),
                name: "Balance".into(),
                account_type: if index == 1 { "savings" } else { "normal" }.into(),
                currency: (*currency).into(),
                archived: index == 1,
                created_at: "2025-01-01T00:00:00Z".into(),
            },
        )
        .await;
        ids.push(id);
    }
    // The first opening remains effective even when its source event is voided;
    // only compensating postings cancel it. Accountless events have no effect.
    let event = post_movement(&pool, &owner, &ids[0], "SEK", 12345).await;
    sqlx::query("UPDATE transactions SET state='voided' WHERE id=$1::uuid")
        .bind(event)
        .execute(&pool)
        .await
        .unwrap();
    post_movement(&pool, &owner, &ids[0], "SEK", -45).await;
    let compensated = post_movement(&pool, &owner, &ids[0], "SEK", 700).await;
    sqlx::query("UPDATE transactions SET state='voided' WHERE id=$1::uuid")
        .bind(&compensated)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,currency,signed_amount_minor,occurred_on) VALUES($1::uuid,$2::uuid,$3::uuid,'SEK',-700,'2025-01-01')")
        .bind(&owner).bind(compensated).bind(&ids[0]).execute(&pool).await.unwrap();
    post_movement(&pool, &owner, &ids[1], "EUR", -101).await;
    post_movement(&pool, &owner, &ids[2], "USD", 1).await;
    sqlx::query("INSERT INTO transactions(user_id,kind,currency,amount_minor,occurred_on) VALUES($1::uuid,'income','SEK',999999,'2025-01-01')").bind(&owner).execute(&pool).await.unwrap();
    let foreign = fixture_id(0xa001, 1);
    add_account(
        &pool,
        &other,
        &AccountFixture {
            id: foreign.clone(),
            name: "Private".into(),
            account_type: "normal".into(),
            currency: "SEK".into(),
            archived: false,
            created_at: "2025-01-01T00:00:00Z".into(),
        },
    )
    .await;
    post_movement(&pool, &other, &foreign, "SEK", 999999).await;
    let before = snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let reply = send(router.clone(), Some(&token), "/v1/accounts", "").await;
    assert_eq!(reply.0, StatusCode::OK);
    for (index, expected) in ["123.00", "-1.01", "0.01", "0.00"].iter().enumerate() {
        assert_eq!(reply.2["accounts"][index]["id"], ids[index]);
        assert_eq!(reply.2["accounts"][index]["balance"], *expected);
    }
    assert_eq!(reply.2["accounts"][1]["status"], "archived");
    let foreign_reply = send(router.clone(), Some(&other_token), "/v1/accounts", "").await;
    assert_eq!(foreign_reply.2["accounts"][0]["balance"], "9999.99");
    assert_eq!(account_ids(&foreign_reply.2), vec![foreign.clone()]);
    let rejected = send(
        router.clone(),
        Some(&token),
        &format!("/v1/accounts?cursor={foreign}"),
        "",
    )
    .await;
    assert_eq!(rejected.0, StatusCode::BAD_REQUEST);
    let supplied_balance = send(router, Some(&token), "/v1/accounts?balance=100", "").await;
    assert_eq!(supplied_balance.0, StatusCode::BAD_REQUEST);
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn balances_use_exact_final_sums_and_reject_unrepresentable_pages_without_writes() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let mut ids = Vec::new();
    for index in 0..4 {
        let id = fixture_id(0xb000, index);
        add_account(
            &pool,
            &owner,
            &AccountFixture {
                id: id.clone(),
                name: "Bound".into(),
                account_type: "normal".into(),
                currency: "SEK".into(),
                archived: false,
                created_at: "2025-01-01T00:00:00Z".into(),
            },
        )
        .await;
        ids.push(id);
    }
    // Intermediate i64 sums can overflow even when the final exact sum is valid.
    for minor in [i64::MAX, 1, -1] {
        post_movement(&pool, &owner, &ids[0], "SEK", minor).await;
    }
    post_movement(&pool, &owner, &ids[1], "SEK", -i64::MAX).await;
    for minor in [-i64::MAX, -1] {
        post_movement(&pool, &owner, &ids[2], "SEK", minor).await;
    }
    for minor in [i64::MAX, 1] {
        post_movement(&pool, &owner, &ids[3], "SEK", minor).await;
    }
    let before = snapshot(&pool).await;
    let router = app(Some(pool.clone()));
    let first = send(router.clone(), Some(&token), "/v1/accounts?limit=2", "").await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(first.2["accounts"][0]["balance"], "92233720368547758.07");
    assert_eq!(first.2["accounts"][1]["balance"], "-92233720368547758.07");
    assert_eq!(first.2["next_cursor"], ids[1]);
    for uri in [
        "/v1/accounts".to_owned(),
        format!("/v1/accounts?cursor={}&limit=1", ids[1]),
        format!("/v1/accounts?cursor={}&limit=1", ids[2]),
    ] {
        let reply = send(router.clone(), Some(&token), &uri, "").await;
        assert_error(
            &reply,
            StatusCode::SERVICE_UNAVAILABLE,
            "account_listing_unavailable",
            "Account listing is temporarily unavailable.",
        );
        assert!(reply.2.get("accounts").is_none());
    }
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn public_opening_creation_replay_and_listing_balance() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    let req = Request::patch("/v1/me")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(r#"{"time_zone":"Etc/UTC"}"#))
        .unwrap();
    assert_eq!(
        router.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );
    let body = r#"{"name":"Public opening","account_type":"savings","currency":"EUR","opening_balance":{"amount":"-123.45","occurred_on":"2024-02-29"}}"#;
    let mut original = None;
    for _ in 0..2 {
        let req = Request::post("/v1/accounts?limit=0&balance=999")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("idempotency-key", "independent-balance")
            .body(Body::from(body))
            .unwrap();
        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let data: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
        assert_eq!(data.as_object().unwrap().len(), 5);
        if let Some(ref original) = original {
            assert_eq!(&data, original);
        } else {
            original = Some(data);
        }
    }
    let before = snapshot(&pool).await;
    let result = send(router.clone(), Some(&token), "/v1/accounts", "").await;
    assert_eq!(result.0, StatusCode::OK);
    let mut listed = result.2["accounts"][0].clone();
    assert_eq!(listed.as_object().unwrap().len(), 6);
    assert_eq!(
        listed.as_object_mut().unwrap().remove("balance"),
        Some(json!("-123.45"))
    );
    assert_eq!(Some(listed), original);
    assert_error(
        &send(router, Some(&token), "/v1/accounts?balance=999", "").await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "The request is invalid.",
    );
    assert_eq!(snapshot(&pool).await, before);
}
