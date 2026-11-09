mod common;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

#[tokio::test]
async fn listing_routes_are_mounted_behind_authentication() {
    let app = cuenora_api::registration_http::app(None).layer(axum::Extension(
        axum::extract::ConnectInfo("192.0.2.1:1".parse::<std::net::SocketAddr>().unwrap()),
    ));
    for path in ["/v1/recurring-rules", "/v1/recurring-occurrences"] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
    }
}

async fn send(router: &Router, token: Option<&str>, uri: &str) -> (StatusCode, Value) {
    let mut request = Request::get(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn seed(pool: &PgPool, verified: bool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let user: String = sqlx::query_scalar(
        "INSERT INTO users(email_original,email_address,email_verified_at)
         VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END) RETURNING id::text",
    )
    .bind(email)
    .bind(verified)
    .fetch_one(pool)
    .await
    .unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&user)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at)
         VALUES($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')",
    )
    .bind(session)
    .bind(&user)
    .bind(tokens.access.digest().as_slice())
    .execute(pool)
    .await
    .unwrap();
    (user, tokens.access.expose_secret().to_owned())
}

async fn snapshot(pool: &PgPool) -> String {
    let rows: String = sqlx::query_scalar(
        "SELECT md5(jsonb_build_array(
            (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),
            (SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id),'[]') FROM account_movements m),
            (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM transaction_audit a),
            (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id),'[]') FROM recurring_rules r),
            (SELECT coalesce(jsonb_agg(to_jsonb(o) ORDER BY id),'[]') FROM recurring_occurrences o))::text)",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    format!("{}:{rows}", common::snapshot(pool).await)
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn owner_scoped_pages_filter_order_and_reject_foreign_or_invalid_cursors() {
    let pool = common::pool().await;
    let (owner, token) = seed(&pool, true).await;
    let (other_owner, other_token) = seed(&pool, true).await;
    let router = app(Some(pool.clone()));
    for (id, user, description, state) in [
        (
            "10000000-0000-4000-8000-000000000001",
            owner.as_str(),
            "First",
            "active",
        ),
        (
            "10000000-0000-4000-8000-000000000002",
            owner.as_str(),
            "Second",
            "paused",
        ),
        (
            "20000000-0000-4000-8000-000000000001",
            other_owner.as_str(),
            "Private",
            "active",
        ),
    ] {
        sqlx::query(
            "INSERT INTO recurring_rules(id,user_id,amount_minor,currency,description,cadence,
             starts_on,time_zone,state,paused_on)
             VALUES($1::uuid,$2::uuid,12345,'SEK',$3,'monthly','2000-01-31','Europe/Stockholm',
                    $4,CASE WHEN $4='paused' THEN '2001-01-01'::date END)",
        )
        .bind(id)
        .bind(user)
        .bind(description)
        .bind(state)
        .execute(&pool)
        .await
        .unwrap();
    }
    for (id, rule_id, user, due, state, transaction_id) in [
        (
            "30000000-0000-4000-8000-000000000001",
            "10000000-0000-4000-8000-000000000001",
            owner.as_str(),
            "2000-01-31",
            "planned",
            None::<&str>,
        ),
        (
            "30000000-0000-4000-8000-000000000002",
            "10000000-0000-4000-8000-000000000002",
            owner.as_str(),
            "2000-01-31",
            "planned",
            None::<&str>,
        ),
        (
            "40000000-0000-4000-8000-000000000001",
            "20000000-0000-4000-8000-000000000001",
            other_owner.as_str(),
            "2000-01-31",
            "planned",
            None::<&str>,
        ),
    ] {
        sqlx::query(
            "INSERT INTO recurring_occurrences(id,rule_id,user_id,scheduled_on,due_on,state,
             amount_minor,currency,transaction_id)
             VALUES($1::uuid,$2::uuid,$3::uuid,'2000-01-31',$4::date,$5,12345,'SEK',$6::uuid)",
        )
        .bind(id)
        .bind(rule_id)
        .bind(user)
        .bind(due)
        .bind(state)
        .bind(transaction_id)
        .execute(&pool)
        .await
        .unwrap();
    }
    let before = snapshot(&pool).await;

    let (status, rules) = send(&router, Some(&token), "/v1/recurring-rules?limit=1").await;
    assert_eq!(status, StatusCode::OK, "{rules}");
    assert_eq!(rules["rules"].as_array().unwrap().len(), 1);
    assert_eq!(
        rules["rules"][0]["id"],
        "10000000-0000-4000-8000-000000000001"
    );
    assert_eq!(rules["rules"][0]["amount"], "123.45");
    assert_eq!(rules["rules"][0]["description"], "First");
    assert_eq!(rules["rules"][0].as_object().unwrap().len(), 13);
    let cursor = rules["next_cursor"].as_str().unwrap();
    let (status, second_rules) = send(
        &router,
        Some(&token),
        &format!("/v1/recurring-rules?cursor={cursor}&state=paused"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{second_rules}");
    assert_eq!(second_rules["rules"].as_array().unwrap().len(), 1);
    assert_eq!(
        second_rules["rules"][0]["id"],
        "10000000-0000-4000-8000-000000000002"
    );
    assert!(second_rules["next_cursor"].is_null());

    let (status, occurrences) = send(
        &router,
        Some(&token),
        "/v1/recurring-occurrences?limit=1&state=planned",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{occurrences}");
    assert_eq!(occurrences["occurrences"].as_array().unwrap().len(), 1);
    assert_eq!(occurrences["occurrences"][0]["amount"], "123.45");
    assert!(occurrences["occurrences"][0]["transaction_id"].is_null());
    assert_eq!(occurrences["occurrences"][0].as_object().unwrap().len(), 11);
    let cursor = occurrences["next_cursor"].as_str().unwrap();
    assert!(cursor.starts_with("2000-01-31~"));
    let (status, next) = send(
        &router,
        Some(&token),
        &format!(
            "/v1/recurring-occurrences?cursor={cursor}&rule_id=10000000-0000-4000-8000-000000000001"
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{next}");
    assert!(next["occurrences"].as_array().unwrap().is_empty());

    let foreign_rule_cursor = "20000000-0000-4000-8000-000000000001";
    assert_eq!(
        send(
            &router,
            Some(&token),
            &format!("/v1/recurring-rules?cursor={foreign_rule_cursor}")
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let foreign_occurrence_cursor = "2000-01-31~40000000-0000-4000-8000-000000000001";
    assert_eq!(
        send(
            &router,
            Some(&token),
            &format!("/v1/recurring-occurrences?cursor={foreign_occurrence_cursor}")
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    for uri in [
        "/v1/recurring-rules?limit=0",
        "/v1/recurring-rules?state=ended",
        "/v1/recurring-rules?limit=1&limit=2",
        "/v1/recurring-occurrences?state=active",
        "/v1/recurring-occurrences?rule_id=nope",
        "/v1/recurring-occurrences?limit=1&limit=2",
    ] {
        assert_eq!(
            send(&router, Some(&token), uri).await.0,
            StatusCode::BAD_REQUEST,
            "{uri}"
        );
    }
    let large_query = format!("/v1/recurring-rules?state={}", "x".repeat(1024));
    assert_eq!(
        send(&router, Some(&token), &large_query).await.0,
        StatusCode::BAD_REQUEST
    );
    let body_response = router
        .clone()
        .oneshot(
            Request::get("/v1/recurring-rules")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from("not-empty"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(body_response.status(), StatusCode::BAD_REQUEST);
    let (status, private) = send(&router, Some(&other_token), "/v1/recurring-rules").await;
    assert_eq!(status, StatusCode::OK, "{private}");
    assert_eq!(private["rules"].as_array().unwrap().len(), 1);
    assert_eq!(private["rules"][0]["description"], "Private");
    assert_eq!(snapshot(&pool).await, before);
}
