//! Read-only shared category catalog acceptance.
mod common;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::{registration_http::app, session_tokens::generate};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn send(
    pool: Option<PgPool>,
    token: Option<&str>,
    method: &str,
    path: &str,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = app(pool)
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    if path.starts_with("/v1/categories") && status != StatusCode::METHOD_NOT_ALLOWED {
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    let id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if value.get("code").is_some() {
        assert_eq!(value["request_id"], id);
        assert_eq!(value["details"], Value::Null);
    }
    (status, value)
}

#[tokio::test]
async fn category_reads_require_identity_and_preserve_existing_routes() {
    let reply = send(None, None, "GET", "/v1/categories").await;
    assert_eq!(reply.0, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.1["code"], "invalid_access_token");
    assert_eq!(
        send(None, None, "GET", "/v1/health/live").await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(None, None, "GET", "/categories").await.0,
        StatusCode::NOT_FOUND
    );
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

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn catalog_is_shared_read_only_and_available_before_verification() {
    let pool = common::pool().await;
    let (_, pending) = seed(&pool, false).await;
    let (verified_user, verified) = seed(&pool, true).await;
    let before = snapshot(&pool).await;
    let reply = send(Some(pool.clone()), Some(&pending), "GET", "/v1/categories").await;
    assert_eq!(reply.0, StatusCode::OK);
    let entries = [
        ("salary", "income", "Salary"),
        ("interest", "income", "Interest"),
        ("other_income", "income", "Other income"),
        ("housing", "expense", "Housing"),
        ("groceries", "expense", "Groceries"),
        ("transport", "expense", "Transport"),
        ("utilities", "expense", "Utilities"),
        ("health", "expense", "Health"),
        ("leisure", "expense", "Leisure"),
        ("other_expense", "expense", "Other expense"),
    ];
    let expected: Vec<Value> = entries.iter().enumerate().map(|(i, (code, kind, name))|
        json!({"id": format!("10000000-0000-4000-8000-{:012}", i+1), "code": code, "kind": kind, "name": name})
    ).collect();
    assert_eq!(reply.1, json!({"categories": expected}));
    assert_eq!(
        send(Some(pool.clone()), Some(&verified), "GET", "/v1/categories")
            .await
            .1,
        reply.1
    );
    for method in ["POST", "PUT", "PATCH", "DELETE"] {
        assert_eq!(
            send(Some(pool.clone()), Some(&pending), method, "/v1/categories")
                .await
                .0,
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
    for query in ["kind=income", "user_id=ignored", "limit=1"] {
        let reply = send(
            Some(pool.clone()),
            Some(&pending),
            "GET",
            &format!("/v1/categories?{query}"),
        )
        .await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST);
        assert_eq!(reply.1["code"], "invalid_request");
    }
    let response = app(Some(pool.clone()))
        .oneshot(
            Request::get("/v1/categories")
                .header(header::AUTHORIZATION, format!("Bearer {pending}"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(snapshot(&pool).await, before);
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE categories IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let unavailable = send(Some(pool.clone()), Some(&pending), "GET", "/v1/categories").await;
    assert_eq!(unavailable.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(unavailable.1["code"], "category_listing_unavailable");
    assert_eq!(
        unavailable.1["message"],
        "Category listing is temporarily unavailable."
    );
    lock.rollback().await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(
        send(Some(pool.clone()), Some(&pending), "GET", "/v1/categories")
            .await
            .1,
        reply.1
    );
    sqlx::query("UPDATE user_sessions SET revoked_at=statement_timestamp() WHERE user_id=$1::uuid")
        .bind(&verified_user)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    assert_eq!(
        send(Some(pool.clone()), Some(&verified), "GET", "/v1/categories")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("UPDATE session_access_tokens SET created_at=statement_timestamp()-interval '1 hour', expires_at=statement_timestamp()-interval '1 second' WHERE user_id<>$1::uuid")
        .bind(&verified_user).execute(&pool).await.unwrap();
    assert_eq!(
        send(Some(pool.clone()), Some(&pending), "GET", "/v1/categories")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

async fn snapshot(pool: &PgPool) -> String {
    let data: String = sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM categories c), (SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u), (SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s), (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM session_access_tokens t), (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM transactions t), (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM account_movements m))::text")
        .fetch_one(pool).await.unwrap();
    format!("{}:{data}", common::snapshot(pool).await)
}
