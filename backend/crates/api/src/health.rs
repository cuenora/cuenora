//! Readiness is read-only and dependency-aware; process liveness stays independent.
use axum::{
    Extension, Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::{Acquire, PgPool};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;

// Empty SELECTs resolve required relations/compatibility columns and check read
// permissions without reading user records. This is not migration-history proof.
const SCHEMA_PROBE: &str = "SELECT 1 FROM public.users WHERE FALSE AND email_verified_at IS NULL AND profile_time_zone IS NULL
UNION ALL SELECT 1 FROM public.password_credentials WHERE FALSE AND credential_revision IS NULL
UNION ALL SELECT 1 FROM public.user_sessions WHERE FALSE
UNION ALL SELECT 1 FROM public.session_refresh_tokens WHERE FALSE
UNION ALL SELECT 1 FROM public.accounts WHERE FALSE
UNION ALL SELECT 1 FROM public.categories WHERE FALSE
UNION ALL SELECT 1 FROM public.transactions WHERE FALSE AND income_account_type IS NULL AND quoted_rate IS NULL
UNION ALL SELECT 1 FROM public.account_movements WHERE FALSE AND reverses_movement_id IS NULL
UNION ALL SELECT 1 FROM public.transaction_audit WHERE FALSE
UNION ALL SELECT 1 FROM public.session_access_tokens WHERE FALSE
UNION ALL SELECT 1 FROM public.account_creation_keys WHERE FALSE AND opening_amount_minor IS NULL AND opening_occurred_on IS NULL
UNION ALL SELECT 1 FROM public.income_creation_keys WHERE FALSE
UNION ALL SELECT 1 FROM public.expense_creation_keys WHERE FALSE
UNION ALL SELECT 1 FROM public.transaction_mutation_keys WHERE FALSE
UNION ALL SELECT 1 FROM public.transfer_creation_keys WHERE FALSE
UNION ALL SELECT 1 FROM public.recurring_rules WHERE FALSE
UNION ALL SELECT 1 FROM public.recurring_occurrences WHERE FALSE
UNION ALL SELECT 1 FROM public.recurring_rule_creation_keys WHERE FALSE
UNION ALL SELECT 1 FROM public.recurring_confirmation_keys WHERE FALSE
UNION ALL SELECT 1 FROM public.recurring_occurrence_action_keys WHERE FALSE
UNION ALL SELECT 1 FROM public.savings_terms WHERE FALSE
UNION ALL SELECT 1 FROM public.savings_rate_periods WHERE FALSE
UNION ALL SELECT 1 FROM public.email_verification_challenges WHERE FALSE
UNION ALL SELECT 1 FROM public.password_recovery_challenges WHERE FALSE
UNION ALL SELECT 1 FROM public.password_reset_notifications WHERE FALSE";

#[derive(Clone)]
struct Readiness {
    pool: Option<PgPool>,
    admission: Arc<Semaphore>,
}

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route("/v1/health/ready", get(ready))
        .method_not_allowed_fallback(
            |Extension(id): Extension<crate::request_id::RequestId>| async move {
                crate::api_error::ApiError::MethodNotAllowed.response(&id)
            },
        )
        .with_state(Readiness {
            pool,
            admission: Arc::new(Semaphore::new(1)),
        })
}

fn reply(ready: bool) -> Response {
    let (status, body) = if ready {
        (StatusCode::OK, "OK")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Unavailable")
    };
    (status, [(header::CACHE_CONTROL, "no-store")], body).into_response()
}

async fn ready(State(state): State<Readiness>) -> Response {
    let Some(pool) = state.pool else {
        return reply(false);
    };
    // Never queue probes or allow public polling to fill the database pool.
    let Ok(_permit) = state.admission.try_acquire_owned() else {
        return reply(false);
    };
    reply(matches!(
        tokio::time::timeout(Duration::from_secs(1), async {
            let mut connection = pool.acquire().await?;
            // A cancelled/timed-out probe must not return a connection with
            // an outstanding query to the application's shared pool.
            connection.close_on_drop();
            let mut transaction = connection.begin().await?;
            sqlx::query("SET TRANSACTION READ ONLY")
                .execute(&mut *transaction)
                .await?;
            sqlx::query("SET LOCAL statement_timeout = '750ms'")
                .execute(&mut *transaction)
                .await?;
            sqlx::query("SET LOCAL lock_timeout = '750ms'")
                .execute(&mut *transaction)
                .await?;
            sqlx::query(SCHEMA_PROBE).execute(&mut *transaction).await?;
            transaction.rollback().await
        })
        .await,
        Ok(Ok(_))
    ))
}

#[cfg(test)]
mod tests {
    use super::SCHEMA_PROBE;

    #[test]
    fn every_migrated_table_is_in_the_readiness_probe() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
        let mut count = 0;
        for file in std::fs::read_dir(directory).unwrap() {
            let file = file.unwrap().path();
            if file.extension().is_none_or(|extension| extension != "sql") {
                continue;
            }
            let sql = std::fs::read_to_string(file).unwrap();
            for line in sql.lines() {
                if let Some(rest) = line.strip_prefix("CREATE TABLE ") {
                    let table = rest.split_whitespace().next().unwrap();
                    assert!(
                        SCHEMA_PROBE.contains(&format!("FROM public.{table} WHERE")),
                        "unprobed table {table}"
                    );
                    count += 1;
                }
            }
        }
        assert_eq!(count, 25);
    }
}
