//! One-shot job contract; database scenarios require disposable PostgreSQL.
mod common;
use chrono::{DateTime, Utc};
use cuenora_api::recurrence_job::run;
use sqlx::PgPool;
use std::{
    process::{Command, Output},
    time::Duration,
};

fn now() -> DateTime<Utc> {
    "2024-03-31T12:00:00Z".parse().unwrap()
}
fn command(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cuenora-recurrence-job"))
        .args(args)
        .env_remove("CUENORA_DATABASE_URL")
        .output()
        .unwrap()
}
#[test]
fn cli_help_and_rejections_are_safe_without_database_access() {
    for flag in ["--help", "-h"] {
        let out = command(&[flag]);
        assert!(out.status.success());
        assert!(
            String::from_utf8(out.stdout)
                .unwrap()
                .contains("CUENORA_DATABASE_URL")
        );
        assert!(out.stderr.is_empty());
    }
    for args in [vec![], vec!["--unknown"], vec!["--help", "unexpected"]] {
        let out = command(&args);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    let out = Command::new(env!("CARGO_BIN_EXE_cuenora-recurrence-job"))
        .env("CUENORA_DATABASE_URL", "synthetic-private-secret")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        !String::from_utf8(out.stderr)
            .unwrap()
            .contains("synthetic-private-secret")
    );
}
async fn setup() -> PgPool {
    let pool = common::pool().await;
    // The harness serializes groups; keep each all-owner scan independent.
    sqlx::query(
        "UPDATE recurring_rules SET state='paused',paused_on=starts_on WHERE state='active'",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}
async fn seed(pool: &PgPool, verified: bool, suffix: &str) -> String {
    let email = format!("job-{suffix}@example.test");
    let owner: String = sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at) VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END) RETURNING id::text").bind(email).bind(verified).fetch_one(pool).await.unwrap();
    sqlx::query_scalar("INSERT INTO recurring_rules(user_id,amount_minor,currency,cadence,starts_on,ends_on,time_zone) VALUES($1::uuid,10000,'SEK','monthly','2024-01-31','2024-03-31','Etc/UTC') RETURNING id::text").bind(owner).fetch_one(pool).await.unwrap()
}
async fn financial(pool: &PgPool) -> String {
    let users = common::snapshot(pool).await;
    let rows: String = sqlx::query_scalar("SELECT jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),(SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id),'[]') FROM account_movements m),(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM transaction_audit a))::text").fetch_one(pool).await.unwrap();
    format!("{users}:{rows}")
}
async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id),'[]') FROM recurring_rules r),(SELECT coalesce(jsonb_agg(to_jsonb(o) ORDER BY id),'[]') FROM recurring_occurrences o))::text").fetch_one(pool).await.unwrap()
}
async fn lock(pool: &PgPool) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
    let mut guard = pool.acquire().await.unwrap();
    guard.close_on_drop();
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1,709)")
        .bind(0x4355_454e_i32)
        .fetch_one(&mut *guard)
        .await
        .unwrap();
    assert!(acquired);
    guard
}
fn database_url() -> String {
    format!(
        "postgres://cuenora:{}@127.0.0.1:{}/cuenora",
        std::env::var("CUENORA_TEST_PGPASSWORD").unwrap(),
        std::env::var("CUENORA_TEST_PGPORT").unwrap()
    )
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn job_discovers_verified_active_owners_and_retries_without_posting() {
    let pool = setup().await;
    seed(&pool, true, "first").await;
    seed(&pool, true, "second").await;
    seed(&pool, false, "pending").await;
    let paused = seed(&pool, true, "paused").await;
    sqlx::query(
        "UPDATE recurring_rules SET state='paused',paused_on='2024-02-01' WHERE id=$1::uuid",
    )
    .bind(paused)
    .execute(&pool)
    .await
    .unwrap();
    let before = financial(&pool).await;
    let result = run(&pool, now).await.unwrap();
    assert!(!result.already_running);
    assert_eq!(
        (result.processed, result.failed, result.generated),
        (2, 0, 6)
    );
    let saved = snapshot(&pool).await;
    assert_eq!(run(&pool, now).await.unwrap().generated, 0);
    assert_eq!(snapshot(&pool).await, saved);
    assert_eq!(financial(&pool).await, before);
    let out = Command::new(env!("CARGO_BIN_EXE_cuenora-recurrence-job"))
        .env("CUENORA_DATABASE_URL", database_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "status=completed rules_processed=2 occurrences_created=0 rules_failed=0\n"
    );
    assert!(out.stderr.is_empty());
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn overlapping_process_is_a_successful_noop_and_lock_is_reusable() {
    let pool = setup().await;
    seed(&pool, true, "overlap").await;
    let guard = lock(&pool).await;
    let before = snapshot(&pool).await;
    let result = run(&pool, now).await.unwrap();
    assert!(result.already_running);
    assert_eq!(
        (result.processed, result.failed, result.generated),
        (0, 0, 0)
    );
    let out = Command::new(env!("CARGO_BIN_EXE_cuenora-recurrence-job"))
        .env("CUENORA_DATABASE_URL", database_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, b"status=already_running\n");
    assert_eq!(snapshot(&pool).await, before);
    guard.close().await.unwrap();
    assert_eq!(run(&pool, now).await.unwrap().generated, 3);
    let _reacquired = lock(&pool).await;
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn failed_rule_rolls_back_and_other_rules_continue_with_failure_status() {
    let pool = setup().await;
    let bad = seed(&pool, true, "failure").await;
    let good = seed(&pool, true, "healthy").await;
    sqlx::raw_sql(&format!("CREATE FUNCTION fail_job_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.rule_id='{bad}'::uuid AND NEW.scheduled_on='2024-02-29' THEN RAISE EXCEPTION 'synthetic-private-diagnostic'; END IF; RETURN NEW; END; $$; CREATE TRIGGER fail_job_test BEFORE INSERT ON recurring_occurrences FOR EACH ROW EXECUTE FUNCTION fail_job_test();")).execute(&pool).await.unwrap();
    let before = financial(&pool).await;
    let result = run(&pool, now).await.unwrap();
    assert_eq!(
        (result.processed, result.failed, result.generated),
        (2, 1, 3)
    );
    let missing: bool = sqlx::query_scalar("SELECT generated_through IS NULL AND NOT EXISTS(SELECT 1 FROM recurring_occurrences WHERE rule_id=$1::uuid) FROM recurring_rules WHERE id=$1::uuid").bind(&bad).fetch_one(&pool).await.unwrap();
    assert!(missing);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM recurring_occurrences WHERE rule_id=$1::uuid")
            .bind(good)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 3);
    assert_eq!(financial(&pool).await, before);
    let out = Command::new(env!("CARGO_BIN_EXE_cuenora-recurrence-job"))
        .env("CUENORA_DATABASE_URL", database_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        out.stdout,
        b"status=completed rules_processed=2 occurrences_created=0 rules_failed=1\n"
    );
    assert!(out.stderr.is_empty());
    sqlx::raw_sql(
        "DROP TRIGGER fail_job_test ON recurring_occurrences; DROP FUNCTION fail_job_test();",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(run(&pool, now).await.unwrap().generated, 3);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn cancelled_job_discards_lock_session_and_preserves_waiting_rule() {
    let pool = setup().await;
    let id = seed(&pool, true, "cancelled").await;
    let before = snapshot(&pool).await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=(SELECT user_id FROM recurring_rules WHERE id=$1::uuid) FOR UPDATE").bind(id).execute(&mut *holder).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(200), run(&pool, now))
            .await
            .is_err()
    );
    holder.rollback().await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let report = run(&pool, now).await.unwrap();
            if !report.already_running {
                break report;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(result.generated, 3);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn keyset_scan_covers_multiple_pages_and_does_not_repeat_failed_rules() {
    let pool = setup().await;
    let id = seed(&pool, true, "pages").await;
    sqlx::query("INSERT INTO recurring_rules(user_id,amount_minor,currency,cadence,starts_on,ends_on,time_zone) SELECT user_id,10000,'SEK','monthly','2024-03-31','2024-03-31','Etc/UTC' FROM recurring_rules CROSS JOIN generate_series(1,105) WHERE id=$1::uuid").bind(id).execute(&pool).await.unwrap();
    let report = run(&pool, now).await.unwrap();
    assert_eq!(
        (report.processed, report.failed, report.generated),
        (106, 0, 108)
    );
}

async fn waiting_job_pid(pool: &PgPool) -> i32 {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar("SELECT a.pid FROM pg_locks l JOIN pg_stat_activity a ON a.pid=l.pid WHERE l.locktype='advisory' AND l.classid=1129661774::oid AND l.objid=709::oid AND a.wait_event_type='Lock' LIMIT 1").fetch_optional(pool).await.unwrap();
            if let Some(pid)=pid { return pid; }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }).await.expect("job never reached locked owner/rule")
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_backend_loss_cannot_continue_writing() {
    tokio::task::LocalSet::new().run_until(async {
    let pool=setup().await;
    let id=seed(&pool,true,"independent-kill").await;
    let before=snapshot(&pool).await;
    let money=financial(&pool).await;
    let mut holder=pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=(SELECT user_id FROM recurring_rules WHERE id=$1::uuid) FOR UPDATE").bind(&id).execute(&mut *holder).await.unwrap();
    let p=pool.clone();
    let job=tokio::task::spawn_local(async move { run(&p,now).await });
    let pid=waiting_job_pid(&pool).await;
    let killed:bool=sqlx::query_scalar("SELECT pg_terminate_backend($1)").bind(pid).fetch_one(&pool).await.unwrap();
    assert!(killed);
    holder.rollback().await.unwrap();
    assert!(job.await.unwrap().is_err());
    assert_eq!(snapshot(&pool).await,before);
    assert_eq!(financial(&pool).await,money);
    assert_eq!(run(&pool,now).await.unwrap().generated,3);

 }).await;
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_owner_revalidation_rejects_and_next_rule_recovers() {
    tokio::task::LocalSet::new().run_until(async {
    let pool=setup().await;
    let id=seed(&pool,true,"independent-unverify").await;
    let before=snapshot(&pool).await;
    let mut holder=pool.begin().await.unwrap();
    sqlx::query("UPDATE users SET email_verified_at=NULL WHERE id=(SELECT user_id FROM recurring_rules WHERE id=$1::uuid)").bind(&id).execute(&mut *holder).await.unwrap();
    let p=pool.clone(); let job=tokio::task::spawn_local(async move {run(&p,now).await});
    waiting_job_pid(&pool).await;
    holder.commit().await.unwrap();
    let r=job.await.unwrap().unwrap();
    assert_eq!((r.processed,r.failed,r.generated),(1,1,0));
    assert_eq!(snapshot(&pool).await,before);
    seed(&pool,true,"independent-recovery").await;
    assert_eq!(run(&pool,now).await.unwrap().generated,3);

 }).await;
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_pause_after_discovery_is_respected() {
    tokio::task::LocalSet::new().run_until(async {
    let pool=setup().await; let id=seed(&pool,true,"independent-pause").await;
    let money=financial(&pool).await;
    let mut holder=pool.begin().await.unwrap();
    sqlx::query("UPDATE recurring_rules SET state='paused',paused_on='2024-02-01' WHERE id=$1::uuid").bind(&id).execute(&mut *holder).await.unwrap();
    let p=pool.clone(); let job=tokio::task::spawn_local(async move {run(&p,now).await});
    waiting_job_pid(&pool).await; holder.commit().await.unwrap();
    let saved=snapshot(&pool).await; let r=job.await.unwrap().unwrap();
    assert_eq!((r.processed,r.failed,r.generated),(1,0,0));
    assert_eq!(snapshot(&pool).await,saved);
    assert_eq!(financial(&pool).await,money);

 }).await;
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_cancelled_session_is_not_reused() {
    tokio::task::LocalSet::new().run_until(async {
    let pool=setup().await; let id=seed(&pool,true,"independent-cancel").await;
    let before=snapshot(&pool).await;
    let mut holder=pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=(SELECT user_id FROM recurring_rules WHERE id=$1::uuid) FOR UPDATE").bind(&id).execute(&mut *holder).await.unwrap();
    let p=pool.clone(); let job=tokio::task::spawn_local(async move {run(&p,now).await});
    let pid=waiting_job_pid(&pool).await; job.abort(); assert!(job.await.unwrap_err().is_cancelled());
    holder.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(3),async {
        loop { let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1)").bind(pid).fetch_one(&pool).await.unwrap(); if !exists {break;} tokio::time::sleep(Duration::from_millis(10)).await; }
    }).await.unwrap();
    assert_eq!(snapshot(&pool).await,before);
    let mut c=pool.acquire().await.unwrap();
    let new_pid:i32=sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *c).await.unwrap(); assert_ne!(pid,new_pid); drop(c);
    assert_eq!(run(&pool,now).await.unwrap().generated,3);
    let saved=snapshot(&pool).await;
    unknown_lock_result_cancellation(&pool).await;
    assert_eq!(snapshot(&pool).await,saved);

 }).await;
}

// Withhold the lock query's execution response, not its prepare response.
async fn unknown_lock_result_cancellation(pool: &PgPool) {
    use std::{
        io::{Read, Write},
        net::{Shutdown, TcpListener, TcpStream},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let blocked = Arc::new(AtomicBool::new(false));
    let gate = blocked.clone();
    let upstream_port = std::env::var("CUENORA_TEST_PGPORT").unwrap();
    std::thread::spawn(move || {
        let (mut client, _) = listener.accept().unwrap();
        let mut server = TcpStream::connect(format!("127.0.0.1:{upstream_port}")).unwrap();
        let mut len = [0u8; 4];
        client.read_exact(&mut len).unwrap();
        let n = u32::from_be_bytes(len) as usize;
        let mut payload = vec![0; n - 4];
        client.read_exact(&mut payload).unwrap();
        server.write_all(&len).unwrap();
        server.write_all(&payload).unwrap();
        let mut incoming = server.try_clone().unwrap();
        let mut outgoing = client.try_clone().unwrap();
        let g = gate.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = incoming.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if !g.load(Ordering::SeqCst) && outgoing.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            let _ = outgoing.shutdown(Shutdown::Both);
        });
        let mut lock_prepared = false;
        loop {
            let mut header = [0u8; 5];
            if client.read_exact(&mut header).is_err() {
                break;
            }
            let n = u32::from_be_bytes(header[1..].try_into().unwrap()) as usize;
            let mut body = vec![0; n - 4];
            if client.read_exact(&mut body).is_err() {
                break;
            }
            if header[0] == b'P' && body.windows(21).any(|x| x == b"pg_try_advisory_lock(") {
                lock_prepared = true;
            }
            if header[0] == b'B' && lock_prepared {
                gate.store(true, Ordering::SeqCst);
            }
            if server
                .write_all(&header)
                .and_then(|_| server.write_all(&body))
                .is_err()
            {
                break;
            }
        }
        let _ = server.shutdown(Shutdown::Both);
    });
    let url = database_url().replace(
        &format!(":{}/", std::env::var("CUENORA_TEST_PGPORT").unwrap()),
        &format!(":{port}/"),
    ) + "?sslmode=disable";
    let proxy_pool = cuenora_api::config::pool_from_url(Some(&url))
        .unwrap()
        .unwrap();
    let p = proxy_pool.clone();
    let job = tokio::task::spawn_local(async move { run(&p, now).await });
    tokio::time::timeout(Duration::from_secs(3),async {
        loop {
            let held:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=1129661774::oid AND objid=709::oid)").fetch_one(pool).await.unwrap();
            if held && blocked.load(Ordering::SeqCst) {break;}
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }).await.expect("never withheld acquired advisory result");
    job.abort();
    assert!(job.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(3),async {
        loop {
            let held:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=1129661774::oid AND objid=709::oid)").fetch_one(pool).await.unwrap();
            if !held {break;}
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }).await.expect("unknown-result cancellation leaked advisory lock");
    proxy_pool.close().await;
}

#[test]
fn independent_cli_unavailable_database_is_generic() {
    let out = Command::new(env!("CARGO_BIN_EXE_cuenora-recurrence-job"))
        .env(
            "CUENORA_DATABASE_URL",
            "postgres://synthetic:never-print-this@127.0.0.1:1/cuenora",
        )
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr, b"Recurrence job unavailable; retry safely.\n");
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_late_failure_does_not_poison_following_same_session_rule() {
    let pool = setup().await;
    let mut ids = [
        seed(&pool, true, "independent-first").await,
        seed(&pool, true, "independent-second").await,
    ];
    ids.sort();
    let bad = &ids[0];
    let good = &ids[1];
    sqlx::raw_sql(&format!("CREATE FUNCTION independent_fail_job() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.rule_id='{bad}'::uuid AND NEW.scheduled_on='2024-02-29' THEN RAISE EXCEPTION 'synthetic'; END IF; RETURN NEW; END; $$; CREATE TRIGGER independent_fail_job BEFORE INSERT ON recurring_occurrences FOR EACH ROW EXECUTE FUNCTION independent_fail_job();")).execute(&pool).await.unwrap();
    let before = financial(&pool).await;
    let r = run(&pool, now).await.unwrap();
    assert_eq!((r.processed, r.failed, r.generated), (2, 1, 3));
    let bad_clean:bool=sqlx::query_scalar("SELECT generated_through IS NULL AND NOT EXISTS(SELECT 1 FROM recurring_occurrences WHERE rule_id=$1::uuid) FROM recurring_rules WHERE id=$1::uuid").bind(bad).fetch_one(&pool).await.unwrap();
    assert!(bad_clean);
    let good_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM recurring_occurrences WHERE rule_id=$1::uuid")
            .bind(good)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(good_count, 3);
    assert_eq!(financial(&pool).await, before);
    sqlx::raw_sql("DROP TRIGGER independent_fail_job ON recurring_occurrences; DROP FUNCTION independent_fail_job();").execute(&pool).await.unwrap();
}
