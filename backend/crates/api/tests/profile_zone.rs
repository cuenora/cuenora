//! Profile time-zone confirmation through the public HTTP route.
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

#[tokio::test]
async fn missing_bearer_is_rejected_by_profile_zone_route() {
    let response = app(None)
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/me")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"time_zone":"Europe/Stockholm"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16 * 1024).await.unwrap()).unwrap();
    assert_eq!(body["code"], "invalid_access_token");
}

async fn send(
    router: Router,
    token: Option<&str>,
    body: &str,
    content_type: Option<&str>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::patch("/v1/me");
    if let Some(value) = content_type {
        request = request.header(header::CONTENT_TYPE, value);
    }
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
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16 * 1024).await.unwrap()).unwrap();
    if body.get("code").is_some() {
        assert_eq!(
            body["request_id"].as_str(),
            headers["x-request-id"].to_str().ok()
        );
    }
    (status, headers, body)
}

async fn seed(pool: &PgPool) -> (String, String) {
    let tokens = generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let user: String = sqlx::query_scalar(
        "INSERT INTO users (email_original, email_address) VALUES ($1, $1) RETURNING id::text",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions (user_id) VALUES ($1::uuid) RETURNING id::text",
    )
    .bind(&user)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens (session_id,user_id,token_hash,expires_at) VALUES ($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')")
        .bind(session)
        .bind(&user)
        .bind(tokens.access.digest().as_slice())
        .execute(pool)
        .await
        .unwrap();
    (user, tokens.access.expose_secret().to_owned())
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
    assert_eq!(reply.2["details"], Value::Null);
}

async fn users_snapshot(pool: &PgPool) -> String {
    common::snapshot(pool).await
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn pending_user_confirms_and_updates_only_their_own_zone() {
    let pool = common::pool().await;
    let (owner, owner_token) = seed(&pool).await;
    let (other, other_token) = seed(&pool).await;
    let router = app(Some(pool.clone()));

    let confirmed = send(
        router.clone(),
        Some(&owner_token),
        r#"{"time_zone":"Europe/Stockholm"}"#,
        Some("application/json"),
    )
    .await;
    assert_eq!(confirmed.0, StatusCode::OK);
    assert_eq!(confirmed.2, json!({"time_zone":"Europe/Stockholm"}));
    assert_eq!(confirmed.2.as_object().unwrap().len(), 1);
    let first_updated_at: String =
        sqlx::query_scalar("SELECT updated_at::text FROM users WHERE id = $1::uuid")
            .bind(&owner)
            .fetch_one(&pool)
            .await
            .unwrap();

    let retry = send(
        router.clone(),
        Some(&owner_token),
        r#"{"time_zone":"Europe/Stockholm"}"#,
        Some("application/json"),
    )
    .await;
    assert_eq!(retry.0, StatusCode::OK);
    let retry_updated_at: String =
        sqlx::query_scalar("SELECT updated_at::text FROM users WHERE id = $1::uuid")
            .bind(&owner)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retry_updated_at, first_updated_at);

    let changed = send(
        router.clone(),
        Some(&owner_token),
        r#"{"time_zone":"US/Eastern"}"#,
        Some("application/json"),
    )
    .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert_eq!(changed.2["time_zone"], "US/Eastern");
    let zones: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT (SELECT profile_time_zone FROM users WHERE id=$1::uuid), (SELECT profile_time_zone FROM users WHERE id=$2::uuid)",
    )
    .bind(&owner)
    .bind(&other)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(zones, (Some("US/Eastern".into()), None));
    assert_eq!(
        send(
            router,
            Some(&other_token),
            r#"{"time_zone":"Etc/UTC"}"#,
            Some("application/json")
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn malformed_or_unsupported_requests_fail_safely_without_user_mutation() {
    let pool = common::pool().await;
    let (_, token) = seed(&pool).await;
    let router = app(Some(pool.clone()));
    for (body, status, code, message) in [
        (
            "{".to_owned(),
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is invalid.",
        ),
        (
            r#"{"time_zone":2}"#.to_owned(),
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is invalid.",
        ),
        (
            r#"{"time_zone":"Not/AZone"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
            "unsupported_time_zone",
            "The time zone is unsupported.",
        ),
        (
            r#"{"time_zone":"+01:00"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
            "unsupported_time_zone",
            "The time zone is unsupported.",
        ),
        (
            r#"{"time_zone":"europe/Stockholm"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
            "unsupported_time_zone",
            "The time zone is unsupported.",
        ),
        (
            r#"{"time_zone":" Europe/Stockholm"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
            "unsupported_time_zone",
            "The time zone is unsupported.",
        ),
        (
            r#"{"time_zone":"Europe/Stockholm","user_id":"forged"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is invalid.",
        ),
        (
            r#"{"time_zone":"Etc/UTC","time_zone":"Europe/Stockholm"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is invalid.",
        ),
    ] {
        let before = users_snapshot(&pool).await;
        assert_error(
            &send(
                router.clone(),
                Some(&token),
                &body,
                Some("application/json"),
            )
            .await,
            status,
            code,
            message,
        );
        assert_eq!(users_snapshot(&pool).await, before);
    }
    for content_type in [None, Some("text/plain")] {
        let before = users_snapshot(&pool).await;
        let reply = send(
            router.clone(),
            Some(&token),
            r#"{"time_zone":"Etc/UTC"}"#,
            content_type,
        )
        .await;
        assert_eq!(reply.0, StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(reply.2["code"], "invalid_request");
        assert_eq!(users_snapshot(&pool).await, before);
    }
    let before = users_snapshot(&pool).await;
    let reply = send(
        router,
        Some(&token),
        &format!("{{\"time_zone\":\"{}\"}}", "a".repeat(8192)),
        Some("application/json"),
    )
    .await;
    assert_eq!(reply.0, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(reply.2["code"], "invalid_request");
    assert_eq!(users_snapshot(&pool).await, before);
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn revoked_sessions_cannot_update_profile() {
    let pool = common::pool().await;
    let (user, token) = seed(&pool).await;
    let router = app(Some(pool.clone()));
    sqlx::query("UPDATE user_sessions SET revoked_at=statement_timestamp() WHERE user_id=$1::uuid")
        .bind(&user)
        .execute(&pool)
        .await
        .unwrap();
    let after_revoke = users_snapshot(&pool).await;
    let reply = send(
        router.clone(),
        Some(&token),
        r#"{"time_zone":"Etc/UTC"}"#,
        Some("application/json"),
    )
    .await;
    assert_error(
        &reply,
        StatusCode::UNAUTHORIZED,
        "invalid_access_token",
        "The access token is invalid.",
    );
    assert_eq!(users_snapshot(&pool).await, after_revoke);
}
