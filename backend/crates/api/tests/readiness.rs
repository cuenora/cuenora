mod common;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use cuenora_api::{config::pool_from_url, registration_http};
use std::time::{Duration, Instant};
use tower::ServiceExt;

async fn get(app: Router, path: &str) -> (StatusCode, Vec<u8>) {
    let response = app
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    if path.ends_with("ready") {
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(response.headers().contains_key("x-request-id"));
    }
    (
        status,
        to_bytes(response.into_body(), 1024).await.unwrap().to_vec(),
    )
}

#[tokio::test]
async fn missing_database_is_not_ready_but_liveness_is_unchanged() {
    let app = registration_http::app(None);
    assert_eq!(
        get(app.clone(), "/v1/health/ready").await,
        (StatusCode::SERVICE_UNAVAILABLE, b"Unavailable".to_vec())
    );
    assert_eq!(
        get(app, "/v1/health/live").await,
        (StatusCode::OK, b"OK".to_vec())
    );
}

#[tokio::test]
async fn readiness_preserves_typed_method_errors() {
    let response = registration_http::app(None)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(value["code"], "method_not_allowed");
}

#[tokio::test]
async fn unreachable_database_has_bounded_generic_failure() {
    let pool = pool_from_url(Some(
        "postgres://synthetic:do-not-disclose@127.0.0.1:1/missing",
    ))
    .unwrap()
    .unwrap();
    let app = registration_http::app(Some(pool.clone()));
    let start = Instant::now();
    assert_eq!(
        get(app.clone(), "/v1/health/ready").await,
        (StatusCode::SERVICE_UNAVAILABLE, b"Unavailable".to_vec())
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(
        get(app, "/v1/health/live").await,
        (StatusCode::OK, b"OK".to_vec())
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "isolated PostgreSQL harness only"]
async fn migrated_database_is_ready_without_changing_records() {
    let pool = common::pool().await;
    let before = common::snapshot(&pool).await;
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM transactions),(SELECT count(*) FROM account_movements),(SELECT count(*) FROM transaction_audit)").fetch_one(&pool).await.unwrap();
    let app = registration_http::app(Some(pool.clone()));
    for _ in 0..2 {
        assert_eq!(
            get(app.clone(), "/v1/health/ready").await,
            (StatusCode::OK, b"OK".to_vec())
        );
    }
    assert_eq!(
        get(app, "/v1/health/live").await,
        (StatusCode::OK, b"OK".to_vec())
    );
    assert_eq!(common::snapshot(&pool).await, before);
    let after: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM transactions),(SELECT count(*) FROM account_movements),(SELECT count(*) FROM transaction_audit)").fetch_one(&pool).await.unwrap();
    assert_eq!(counts, after);
    pool.close().await;
}

#[tokio::test]
#[ignore = "isolated PostgreSQL harness only"]
async fn missing_late_relation_or_column_is_not_ready() {
    let pool = common::pool().await;
    let app = registration_http::app(Some(pool.clone()));
    for (remove, restore) in [
        (
            "ALTER TABLE password_reset_notifications RENAME TO health_missing_notifications",
            "ALTER TABLE health_missing_notifications RENAME TO password_reset_notifications",
        ),
        (
            "ALTER TABLE transactions RENAME COLUMN quoted_rate TO health_missing_quote",
            "ALTER TABLE transactions RENAME COLUMN health_missing_quote TO quoted_rate",
        ),
    ] {
        sqlx::query(remove).execute(&pool).await.unwrap();
        let response = get(app.clone(), "/v1/health/ready").await;
        sqlx::query(restore).execute(&pool).await.unwrap();
        assert_eq!(
            response,
            (StatusCode::SERVICE_UNAVAILABLE, b"Unavailable".to_vec())
        );
        assert_eq!(
            get(app.clone(), "/v1/health/ready").await,
            (StatusCode::OK, b"OK".to_vec())
        );
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "isolated PostgreSQL harness only"]
async fn locked_schema_times_out_without_blocking_liveness_or_queuing_probes() {
    let pool = common::pool().await;
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE users IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let app = registration_http::app(Some(pool.clone()));
    let pending = tokio::spawn(get(app.clone(), "/v1/health/ready"));
    let mut waiting = false;
    for _ in 0..50 {
        waiting = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'SELECT 1 FROM public.users%')").fetch_one(&pool).await.unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if waiting {
        let start = Instant::now();
        assert_eq!(
            get(app.clone(), "/v1/health/ready").await,
            (StatusCode::SERVICE_UNAVAILABLE, b"Unavailable".to_vec())
        );
        assert!(start.elapsed() < Duration::from_millis(500));
        assert_eq!(
            get(app.clone(), "/v1/health/live").await,
            (StatusCode::OK, b"OK".to_vec())
        );
    }
    let result = pending.await.unwrap();
    lock.rollback().await.unwrap();
    assert!(
        waiting,
        "readiness never attempted the blocked schema query"
    );
    assert_eq!(
        result,
        (StatusCode::SERVICE_UNAVAILABLE, b"Unavailable".to_vec())
    );
    assert_eq!(
        get(app, "/v1/health/ready").await,
        (StatusCode::OK, b"OK".to_vec())
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "isolated PostgreSQL harness only"]
async fn empty_database_is_not_ready() {
    let pool = common::pool().await;
    sqlx::query("CREATE DATABASE cuenora_health_empty")
        .execute(&pool)
        .await
        .unwrap();
    let empty = sqlx::postgres::PgPoolOptions::new()
        .connect_with(
            (*pool.connect_options())
                .clone()
                .database("cuenora_health_empty"),
        )
        .await
        .unwrap();
    let app = registration_http::app(Some(empty.clone()));
    let ready = get(app.clone(), "/v1/health/ready").await;
    let live = get(app, "/v1/health/live").await;
    empty.close().await;
    sqlx::query("DROP DATABASE cuenora_health_empty")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        ready,
        (StatusCode::SERVICE_UNAVAILABLE, b"Unavailable".to_vec())
    );
    assert_eq!(live, (StatusCode::OK, b"OK".to_vec()));
    pool.close().await;
}

#[tokio::test]
#[ignore = "isolated PostgreSQL harness only"]
async fn read_only_role_detects_missing_select_permission() {
    let admin = common::pool().await;
    sqlx::query("CREATE ROLE health_verifier LOGIN PASSWORD 'synthetic-health-only'")
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO health_verifier")
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON ALL TABLES IN SCHEMA public TO health_verifier")
        .execute(&admin)
        .await
        .unwrap();
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(
            (*admin.connect_options())
                .clone()
                .username("health_verifier")
                .password("synthetic-health-only"),
        )
        .await
        .unwrap();
    let app = registration_http::app(Some(reader.clone()));
    assert_eq!(
        get(app.clone(), "/v1/health/ready").await,
        (StatusCode::OK, b"OK".to_vec())
    );
    sqlx::query("REVOKE SELECT ON users FROM health_verifier")
        .execute(&admin)
        .await
        .unwrap();
    assert_eq!(
        get(app.clone(), "/v1/health/ready").await,
        (StatusCode::SERVICE_UNAVAILABLE, b"Unavailable".to_vec())
    );
    assert_eq!(
        get(app, "/v1/health/live").await,
        (StatusCode::OK, b"OK".to_vec())
    );
    reader.close().await;
    sqlx::query("DROP OWNED BY health_verifier")
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query("DROP ROLE health_verifier")
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "isolated PostgreSQL harness only"]
async fn cancelled_probe_discards_pending_connection() {
    let admin = common::pool().await;
    let probe_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*admin.connect_options()).clone())
        .await
        .unwrap();
    let old_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&probe_pool)
        .await
        .unwrap();
    let mut lock = admin.begin().await.unwrap();
    sqlx::query("LOCK TABLE users IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let app = registration_http::app(Some(probe_pool.clone()));
    let pending = tokio::spawn(get(app.clone(), "/v1/health/ready"));
    let mut waiting = false;
    for _ in 0..50 {
        waiting = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')",
        )
        .bind(old_pid)
        .fetch_one(&admin)
        .await
        .unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(waiting);
    pending.abort();
    let _ = pending.await;
    let new_pid: i32 = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&probe_pool),
    )
    .await
    .unwrap()
    .unwrap();
    assert_ne!(old_pid, new_pid);
    lock.rollback().await.unwrap();
    assert_eq!(
        get(app, "/v1/health/ready").await,
        (StatusCode::OK, b"OK".to_vec())
    );
    probe_pool.close().await;
    admin.close().await;
}
