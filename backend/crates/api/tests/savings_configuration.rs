//! Public-interface savings configuration regression tests.
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
    app: Router,
    token: Option<&str>,
    method: &str,
    uri: &str,
    input: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    if input.is_some() {
        req = req.header("content-type", "application/json");
    }
    let response = app
        .oneshot(
            req.body(Body::from(input.map(|v| v.to_string()).unwrap_or_default()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    if body.get("code").is_some() {
        assert_eq!(body["request_id"], id);
        assert!(body["details"].is_null());
    }
    (status, body)
}
async fn seed(pool: &PgPool, verified: bool, kind: &str) -> (String, String, String) {
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
    let account: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency) VALUES($1::uuid,'Synthetic savings',$2,'SEK') RETURNING id::text").bind(&user).bind(kind).fetch_one(pool).await.unwrap();
    (user, tokens.access.expose_secret().to_owned(), account)
}
fn terms(date: &str) -> Value {
    json!({"effective_on":date,"payout_frequency":"monthly","capitalize":false,"rate_basis":"nominal_annual","day_count":"actual_calendar_year","balance_timing":"daily_close","rounding":"period_end_half_even"})
}
async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM savings_terms t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM savings_rate_periods t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM account_movements t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transaction_audit t),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM accounts t))::text").fetch_one(pool).await.unwrap()
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn append_query_and_retry_independent_histories_without_posting() {
    let pool = common::pool().await;
    let _ = common::snapshot(&pool).await;
    let (_, token, account) = seed(&pool, true, "savings").await;
    let app = app(Some(pool.clone()));
    let uri = format!("/v1/accounts/{account}/savings-terms");
    let rates = format!("/v1/accounts/{account}/savings-rates");
    assert_eq!(
        send(app.clone(), Some(&token), "GET", &uri, None).await,
        (StatusCode::OK, json!({"versions":[],"next_cursor":null}))
    );
    let input = terms("2024-02-29");
    let created = send(app.clone(), Some(&token), "POST", &uri, Some(input.clone())).await;
    assert_eq!(created.0, StatusCode::CREATED);
    assert_eq!(created.1["effective_on"], "2024-02-29");
    let before = snapshot(&pool).await;
    assert_eq!(
        send(app.clone(), Some(&token), "POST", &uri, Some(input)).await,
        created
    );
    let mut changed = terms("2024-02-29");
    changed["capitalize"] = json!(true);
    assert_eq!(
        send(app.clone(), Some(&token), "POST", &uri, Some(changed))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(snapshot(&pool).await, before);
    for (index, cadence) in ["daily", "quarterly", "annual"].iter().enumerate() {
        let mut input = terms(&format!("9999-12-{}", 28 + index));
        input["payout_frequency"] = json!(cadence);
        input["capitalize"] = json!(true);
        assert_eq!(
            send(app.clone(), Some(&token), "POST", &uri, Some(input))
                .await
                .0,
            StatusCode::CREATED
        );
    }
    for (date, rate) in [("0001-01-01", "0"), ("2024-03-01", "3.123456")] {
        let input = json!({"effective_on":date,"nominal_annual_percent":rate});
        let created = send(
            app.clone(),
            Some(&token),
            "POST",
            &rates,
            Some(input.clone()),
        )
        .await;
        assert_eq!(created.0, StatusCode::CREATED);
        assert_eq!(created.1["nominal_annual_percent"], rate);
        assert_eq!(
            send(app.clone(), Some(&token), "POST", &rates, Some(input)).await,
            created
        );
    }
    let original = send(app.clone(), Some(&token), "GET", &rates, None).await;
    assert_eq!(original.0, StatusCode::OK);
    assert_eq!(original.1["versions"].as_array().unwrap().len(), 2);
    let replay = send(
        app.clone(),
        Some(&token),
        "POST",
        &rates,
        Some(json!({"effective_on":"0001-01-01","nominal_annual_percent":"0.000000"})),
    )
    .await;
    assert_eq!(replay.0, StatusCode::CREATED);
    assert_eq!(replay.1, original.1["versions"][0]);
    let before = snapshot(&pool).await;
    let conflict = send(
        app.clone(),
        Some(&token),
        "POST",
        &rates,
        Some(json!({"effective_on":"0001-01-01","nominal_annual_percent":"1"})),
    )
    .await;
    assert_eq!(conflict.0, StatusCode::CONFLICT);
    assert_eq!(conflict.1["code"], "savings_configuration_conflict");
    assert_eq!(snapshot(&pool).await, before);
    // Existing account-list behavior is preserved by the additional route merge.
    assert_eq!(
        send(app.clone(), Some(&token), "GET", "/v1/accounts", None)
            .await
            .0,
        StatusCode::OK
    );
    let page = send(
        app.clone(),
        Some(&token),
        "GET",
        &format!("{uri}?limit=1"),
        None,
    )
    .await;
    assert_eq!(page.0, StatusCode::OK);
    assert_eq!(page.1["versions"].as_array().unwrap().len(), 1);
    assert_eq!(page.1["next_cursor"], "2024-02-29");
    let rest = send(
        app.clone(),
        Some(&token),
        "GET",
        &format!("{uri}?cursor=2024-02-29"),
        None,
    )
    .await;
    assert_eq!(rest.1["versions"].as_array().unwrap().len(), 3);
    assert!(rest.1["next_cursor"].is_null());
    let before = snapshot(&pool).await;
    for method in ["PATCH", "DELETE"] {
        assert_eq!(
            send(
                app.clone(),
                Some(&token),
                method,
                &uri,
                Some(terms("2024-02-29"))
            )
            .await
            .0,
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
    assert_eq!(snapshot(&pool).await, before);
    let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM transactions WHERE account_id=$1::uuid),(SELECT count(*) FROM account_movements WHERE account_id=$1::uuid),(SELECT count(*) FROM transaction_audit WHERE user_id=(SELECT user_id FROM accounts WHERE id=$1::uuid))").bind(account).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn reject_invalid_input_and_enforce_verified_savings_ownership() {
    let pool = common::pool().await;
    let (_, token, account) = seed(&pool, true, "savings").await;
    let (_, foreign, _) = seed(&pool, true, "savings").await;
    let (_, pending, pending_account) = seed(&pool, false, "savings").await;
    let (_, normal, normal_account) = seed(&pool, true, "normal").await;
    let app = app(Some(pool.clone()));
    for stream in ["savings-terms", "savings-rates"] {
        let uri = format!("/v1/accounts/{account}/{stream}");
        let input = if stream == "savings-terms" {
            terms("2024-01-01")
        } else {
            json!({"effective_on":"2024-01-01","nominal_annual_percent":"2"})
        };
        let before = snapshot(&pool).await;
        for method in ["GET", "POST"] {
            let body = (method == "POST").then(|| input.clone());
            for (bearer, target, status) in [
                (None, account.as_str(), StatusCode::UNAUTHORIZED),
                (
                    Some(foreign.as_str()),
                    account.as_str(),
                    StatusCode::NOT_FOUND,
                ),
                (
                    Some(pending.as_str()),
                    pending_account.as_str(),
                    StatusCode::FORBIDDEN,
                ),
                (
                    Some(normal.as_str()),
                    normal_account.as_str(),
                    StatusCode::NOT_FOUND,
                ),
            ] {
                assert_eq!(
                    send(
                        app.clone(),
                        bearer,
                        method,
                        &format!("/v1/accounts/{target}/{stream}"),
                        body.clone()
                    )
                    .await
                    .0,
                    status
                );
            }
        }
        for query in [
            "limit=0",
            "limit=101",
            "limit=x",
            "cursor=2024-02-30",
            "unknown=1",
            "limit=1&limit=2",
        ] {
            assert_eq!(
                send(
                    app.clone(),
                    Some(&token),
                    "GET",
                    &format!("{uri}?{query}"),
                    None
                )
                .await
                .0,
                StatusCode::BAD_REQUEST
            );
        }
        for date in ["2024-02-30", "0000-01-01", "10000-01-01", "2024-1-01"] {
            let mut invalid = input.clone();
            invalid["effective_on"] = json!(date);
            assert_eq!(
                send(app.clone(), Some(&token), "POST", &uri, Some(invalid))
                    .await
                    .0,
                StatusCode::BAD_REQUEST
            );
        }
        let mut invalid = input.clone();
        invalid["user_id"] = json!("not-authority");
        assert_eq!(
            send(app.clone(), Some(&token), "POST", &uri, Some(invalid))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    let rates = format!("/v1/accounts/{account}/savings-rates");
    let before = snapshot(&pool).await;
    for rate in [
        json!("-1"),
        json!("NaN"),
        json!("Infinity"),
        json!("3.1234560"),
        json!("1e2"),
        json!(" 2"),
        json!(""),
        json!(2),
        Value::Null,
    ] {
        assert_eq!(
            send(
                app.clone(),
                Some(&token),
                "POST",
                &rates,
                Some(json!({"effective_on":"2024-01-01","nominal_annual_percent":rate}))
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let uri = format!("/v1/accounts/{account}/savings-terms");
    for (key, value) in [
        ("payout_frequency", json!("weekly")),
        ("capitalize", Value::Null),
        ("rate_basis", json!("effective_annual")),
        ("day_count", json!("360")),
        ("rounding", json!("daily")),
        ("balance_timing", json!("opening")),
    ] {
        let mut input = terms("2024-01-01");
        input[key] = value;
        assert_eq!(
            send(app.clone(), Some(&token), "POST", &uri, Some(input))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    for field in [
        "effective_on",
        "capitalize",
        "rate_basis",
        "day_count",
        "balance_timing",
        "rounding",
        "payout_frequency",
    ] {
        let mut input = terms("2024-01-01");
        input.as_object_mut().unwrap().remove(field);
        assert_eq!(
            send(app.clone(), Some(&token), "POST", &uri, Some(input))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        send(
            app.clone(),
            Some(&token),
            "POST",
            &format!("{uri}?unknown=1"),
            Some(terms("2024-01-01"))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(app.clone(), Some(&token), "GET", &uri, Some(json!({})))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(snapshot(&pool).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn concurrent_retries_and_failed_insert_are_atomic() {
    let pool = common::pool().await;
    let (user, token, account) = seed(&pool, true, "savings").await;
    let app = app(Some(pool.clone()));
    let uri = format!("/v1/accounts/{account}/savings-terms");
    let (a, b) = tokio::join!(
        send(
            app.clone(),
            Some(&token),
            "POST",
            &uri,
            Some(terms("2025-01-01"))
        ),
        send(
            app.clone(),
            Some(&token),
            "POST",
            &uri,
            Some(terms("2025-01-01"))
        )
    );
    assert_eq!(a.0, StatusCode::CREATED);
    assert_eq!(a, b);
    // Regression: different concurrent rate writes must elect exactly one value.
    let rates_uri = format!("/v1/accounts/{account}/savings-rates");
    let (left, right) = tokio::join!(
        send(
            app.clone(),
            Some(&token),
            "POST",
            &rates_uri,
            Some(json!({"effective_on":"2031-01-01","nominal_annual_percent":"0.000001"}))
        ),
        send(
            app.clone(),
            Some(&token),
            "POST",
            &rates_uri,
            Some(json!({"effective_on":"2031-01-01","nominal_annual_percent":"100.000000"}))
        )
    );
    assert!(
        (left.0 == StatusCode::CREATED && right.0 == StatusCode::CONFLICT)
            || (right.0 == StatusCode::CREATED && left.0 == StatusCode::CONFLICT)
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM savings_rate_periods WHERE account_id=$1::uuid AND effective_on=DATE '2031-01-01'").bind(&account).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    let winner = if left.0 == StatusCode::CREATED {
        &left.1
    } else {
        &right.1
    };
    let before_retry = snapshot(&pool).await;
    let loser_rate = if left.0 == StatusCode::CREATED {
        "100.000000"
    } else {
        "0.000001"
    };
    assert_eq!(
        send(
            app.clone(),
            Some(&token),
            "POST",
            &rates_uri,
            Some(json!({"effective_on":"2031-01-01","nominal_annual_percent":loser_rate}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(snapshot(&pool).await, before_retry);
    let history = send(app.clone(), Some(&token), "GET", &rates_uri, None).await;
    assert_eq!(&history.1["versions"][0], winner);
    // Revoked/expired sessions cannot use either stream, and rejected requests are read-only.
    for expired in [false, true] {
        let (other, bearer, owned) = seed(&pool, true, "savings").await;
        if expired {
            sqlx::query("UPDATE session_access_tokens SET created_at=statement_timestamp()-interval '2 minutes', expires_at=statement_timestamp()-interval '1 second' WHERE user_id=$1::uuid").bind(&other).execute(&pool).await.unwrap();
        } else {
            sqlx::query(
                "UPDATE user_sessions SET revoked_at=statement_timestamp() WHERE user_id=$1::uuid",
            )
            .bind(&other)
            .execute(&pool)
            .await
            .unwrap();
        }
        let before_denied = snapshot(&pool).await;
        for stream in ["savings-terms", "savings-rates"] {
            let target = format!("/v1/accounts/{owned}/{stream}");
            for method in ["GET", "POST"] {
                let payload = (method == "POST").then(|| {
                    if stream == "savings-terms" {
                        terms("2030-01-01")
                    } else {
                        json!({"effective_on":"2030-01-01","nominal_annual_percent":"2"})
                    }
                });
                assert_eq!(
                    send(app.clone(), Some(&bearer), method, &target, payload)
                        .await
                        .0,
                    StatusCode::UNAUTHORIZED
                );
            }
        }
        assert_eq!(snapshot(&pool).await, before_denied);
    }
    let before = snapshot(&pool).await;
    sqlx::query("ALTER TABLE savings_rate_periods ADD CONSTRAINT savings_test_failure CHECK(effective_on <> DATE '2025-01-01')").execute(&pool).await.unwrap();
    let rates = format!("/v1/accounts/{account}/savings-rates");
    let reply = send(
        app.clone(),
        Some(&token),
        "POST",
        &rates,
        Some(json!({"effective_on":"2025-01-01","nominal_annual_percent":"2"})),
    )
    .await;
    sqlx::query("ALTER TABLE savings_rate_periods DROP CONSTRAINT savings_test_failure")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(reply.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=$1::uuid")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        send(
            app.clone(),
            Some(&token),
            "POST",
            &uri,
            Some(terms("2025-01-01"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}
