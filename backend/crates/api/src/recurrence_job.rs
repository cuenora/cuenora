//! One-shot planning job; no scheduler, bearer identity fabrication or posting.
use crate::recurring_rules::generate_due_on_connection;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

// Stable database-scoped namespace "CUEN", job 709. Do not change across replicas.
const LOCK_NAMESPACE: i32 = 0x4355_454e;
const LOCK_JOB: i32 = 709;
const PAGE_SIZE: i64 = 100;

/// Aggregate counters only; never contains user/rule IDs or financial terms.
#[derive(Debug, Default)]
pub struct Report {
    pub already_running: bool,
    pub processed: u64,
    pub failed: u64,
    pub generated: u64,
}
/// Storage/admission failure, deliberately without database diagnostics.
#[derive(Debug)]
pub struct Unavailable;
fn unavailable(_: sqlx::Error) -> Unavailable {
    Unavailable
}

/// Run one bounded discovery pass. `clock` is a trusted server clock, evaluated
/// for each rule after its locks. Rule failures are isolated and counted; callers
/// must treat `failed > 0` as failure, not a completely successful pass.
pub async fn run(pool: &PgPool, clock: impl Fn() -> DateTime<Utc>) -> Result<Report, Unavailable> {
    let mut connection = pool.acquire().await.map_err(unavailable)?;
    // Arm before any lock attempt: even cancellation with an unknown lock result
    // must close the session, never return a session advisory lock to the pool.
    connection.close_on_drop();
    sqlx::raw_sql("SET statement_timeout = '5s'; SET lock_timeout = '2s'")
        .execute(&mut *connection)
        .await
        .map_err(unavailable)?;
    let admitted: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1,$2)")
        .bind(LOCK_NAMESPACE)
        .bind(LOCK_JOB)
        .fetch_one(&mut *connection)
        .await
        .map_err(unavailable)?;
    if !admitted {
        connection.close().await.map_err(unavailable)?;
        return Ok(Report {
            already_running: true,
            ..Report::default()
        });
    }
    // High watermark bounds this run even if new rules arrive continuously.
    let upper: Option<String> = sqlx::query_scalar(
        "SELECT r.id::text FROM recurring_rules r JOIN users u ON u.id=r.user_id
         WHERE r.state='active' AND u.email_verified_at IS NOT NULL ORDER BY r.id DESC LIMIT 1",
    )
    .fetch_optional(&mut *connection)
    .await
    .map_err(unavailable)?;
    let mut report = Report::default();
    let mut after: Option<String> = None;
    if let Some(upper) = upper {
        loop {
            let page: Vec<(String,String)> = sqlx::query_as(
                "SELECT r.id::text,r.user_id::text FROM recurring_rules r JOIN users u ON u.id=r.user_id
                 WHERE r.state='active' AND u.email_verified_at IS NOT NULL
                   AND ($1::uuid IS NULL OR r.id>$1::uuid) AND r.id<=$2::uuid
                 ORDER BY r.id LIMIT $3",
            ).bind(&after).bind(&upper).bind(PAGE_SIZE)
                .fetch_all(&mut *connection).await.map_err(unavailable)?;
            if page.is_empty() {
                break;
            }
            for (id, owner) in page {
                report.processed += 1;
                // This is the SAME session as the advisory lock. Connection loss
                // cannot leave a detached generator writing under lost exclusion.
                match generate_due_on_connection(&mut connection, &owner, &id, &clock).await {
                    Ok(count) => report.generated += count,
                    Err(_) => report.failed += 1,
                }
                after = Some(id);
            }
        }
    }
    let released: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1,$2)")
        .bind(LOCK_NAMESPACE)
        .bind(LOCK_JOB)
        .fetch_one(&mut *connection)
        .await
        .map_err(unavailable)?;
    if !released {
        return Err(Unavailable);
    }
    connection.close().await.map_err(unavailable)?;
    Ok(report)
}
