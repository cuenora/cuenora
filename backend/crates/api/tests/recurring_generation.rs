//! Callable application boundary; never exposed as a production HTTP route.
mod common;
use axum::{Extension, Router, body::Body, http::Request, middleware, routing::get};
use chrono::{DateTime, Utc};
use cuenora_api::{
    auth_identity::{AuthenticatedIdentity, require_identity},
    recurring_rules::generate_due,
    session_tokens,
};
use sqlx::PgPool;
use tower::ServiceExt;

fn instant(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
async fn identity(pool: &PgPool, verified: bool) -> AuthenticatedIdentity {
    let tokens = session_tokens::generate().unwrap();
    let email = format!("{}@example.test", &tokens.access.expose_secret()[4..24]);
    let owner: String = sqlx::query_scalar("INSERT INTO users(email_original,email_address,email_verified_at) VALUES($1,$1,CASE WHEN $2 THEN statement_timestamp() END) RETURNING id::text").bind(email).bind(verified).fetch_one(pool).await.unwrap();
    let session: String = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id) VALUES($1::uuid) RETURNING id::text",
    )
    .bind(&owner)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO session_access_tokens(session_id,user_id,token_hash,expires_at) VALUES($1::uuid,$2::uuid,$3,statement_timestamp()+interval '15 minutes')").bind(session).bind(owner).bind(tokens.access.digest().as_slice()).execute(pool).await.unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
    let router = Router::new()
        .route(
            "/identity",
            get(
                move |Extension(id): Extension<AuthenticatedIdentity>| async move {
                    tx.lock().unwrap().take().unwrap().send(id).ok();
                },
            ),
        )
        .route_layer(middleware::from_fn_with_state(
            Some(pool.clone()),
            require_identity,
        ));
    assert_eq!(
        router
            .oneshot(
                Request::builder()
                    .uri("/identity")
                    .header(
                        "authorization",
                        format!("Bearer {}", tokens.access.expose_secret())
                    )
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        200
    );
    rx.await.unwrap()
}
async fn rule(pool: &PgPool, owner: &AuthenticatedIdentity, zone: &str) -> String {
    sqlx::query_scalar("INSERT INTO recurring_rules(user_id,amount_minor,currency,cadence,starts_on,time_zone,description) VALUES($1::uuid,10000,'SEK','monthly','2024-01-31',$2,'Original') RETURNING id::text").bind(owner.user_id()).bind(zone).fetch_one(pool).await.unwrap()
}
async fn slots(pool: &PgPool, id: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT to_char(scheduled_on,'YYYY-MM-DD') FROM recurring_occurrences WHERE rule_id=$1::uuid ORDER BY scheduled_on").bind(id).fetch_all(pool).await.unwrap()
}
async fn snapshot(pool: &PgPool, id: &str) -> String {
    sqlx::query_scalar("SELECT jsonb_build_array((SELECT to_jsonb(r) FROM recurring_rules r WHERE id=$1::uuid),(SELECT coalesce(jsonb_agg(to_jsonb(o) ORDER BY scheduled_on),'[]') FROM recurring_occurrences o WHERE rule_id=$1::uuid))::text").bind(id).fetch_one(pool).await.unwrap()
}
async fn financial(pool: &PgPool) -> String {
    let identity_snapshot = common::snapshot(pool).await;
    let rows: String = sqlx::query_scalar("SELECT jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM transactions t),(SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id),'[]') FROM account_movements m),(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM transaction_audit a))::text").fetch_one(pool).await.unwrap();
    format!("{identity_snapshot}:{rows}")
}
async fn run(pool: &PgPool, owner: &AuthenticatedIdentity, id: &str, at: &str) -> u64 {
    generate_due(pool, owner, id, || instant(at)).await.unwrap()
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn retries_and_concurrent_runs_preserve_one_snapshot_without_posting() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    let id = rule(&pool, &owner, "Europe/Stockholm").await;
    let before = financial(&pool).await;
    let (a, b) = tokio::join!(
        generate_due(&pool, &owner, &id, || instant("2024-03-31T12:00:00Z")),
        generate_due(&pool, &owner, &id, || instant("2024-03-31T12:00:00Z"))
    );
    assert_eq!(a.unwrap() + b.unwrap(), 3);
    assert_eq!(
        slots(&pool, &id).await,
        ["2024-01-31", "2024-02-29", "2024-03-31"]
    );
    let saved = snapshot(&pool, &id).await;
    assert_eq!(run(&pool, &owner, &id, "2024-03-31T12:00:00Z").await, 0);
    assert_eq!(run(&pool, &owner, &id, "2024-02-01T12:00:00Z").await, 0);
    assert_eq!(snapshot(&pool, &id).await, saved);
    let planned: bool = sqlx::query_scalar("SELECT bool_and(state='planned' AND transaction_id IS NULL AND due_on=scheduled_on AND amount_minor=10000 AND currency='SEK' AND description='Original') FROM recurring_occurrences WHERE rule_id=$1::uuid").bind(&id).fetch_one(&pool).await.unwrap();
    assert!(planned);
    assert_eq!(financial(&pool).await, before);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn rule_zone_today_is_inclusive_and_future_slots_are_not_generated() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    for (zone, expected) in [
        ("Europe/Stockholm", 2),
        ("America/Los_Angeles", 1),
        ("Pacific/Kiritimati", 2),
    ] {
        let id = rule(&pool, &owner, zone).await;
        assert_eq!(
            run(&pool, &owner, &id, "2024-02-29T00:30:00Z").await,
            expected
        );
        assert_eq!(slots(&pool, &id).await.len() as u64, expected);
    }
    let future = rule(&pool, &owner, "Etc/UTC").await;
    assert_eq!(run(&pool, &owner, &future, "2024-01-30T12:00:00Z").await, 0);
    assert!(slots(&pool, &future).await.is_empty());
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn pauses_exclusions_end_and_edit_coverage_preserve_calendar_boundaries() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    let id = rule(&pool, &owner, "Etc/UTC").await;
    sqlx::query(
        "UPDATE recurring_rules SET state='paused',paused_on='2024-02-01' WHERE id=$1::uuid",
    )
    .bind(&id)
    .execute(&pool)
    .await
    .unwrap();
    let paused = snapshot(&pool, &id).await;
    assert_eq!(run(&pool, &owner, &id, "2024-05-31T12:00:00Z").await, 0);
    assert_eq!(snapshot(&pool, &id).await, paused);
    sqlx::query("UPDATE recurring_rules SET state='active',paused_on=NULL,excluded_dates='{[2024-02-01,2024-04-01)}',ends_on='2024-05-31' WHERE id=$1::uuid").bind(&id).execute(&pool).await.unwrap();
    assert_eq!(run(&pool, &owner, &id, "2024-06-30T12:00:00Z").await, 3);
    assert_eq!(
        slots(&pool, &id).await,
        ["2024-01-31", "2024-04-30", "2024-05-31"]
    );
    let resumed = rule(&pool, &owner, "Etc/UTC").await;
    sqlx::query("UPDATE recurring_rules SET excluded_dates='{[2024-02-01,2024-02-29)}',generated_through='2024-01-31' WHERE id=$1::uuid").bind(&resumed).execute(&pool).await.unwrap();
    assert_eq!(
        run(&pool, &owner, &resumed, "2024-02-29T12:00:00Z").await,
        1
    );
    let edited = rule(&pool, &owner, "Etc/UTC").await;
    sqlx::query("UPDATE recurring_rules SET starts_on='2024-01-15',generated_through='2024-03-15' WHERE id=$1::uuid").bind(&edited).execute(&pool).await.unwrap();
    assert_eq!(run(&pool, &owner, &edited, "2024-04-15T12:00:00Z").await, 1);
    assert_eq!(slots(&pool, &edited).await, ["2024-04-15"]);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn existing_slots_and_archived_references_survive_term_changes() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    let id = rule(&pool, &owner, "Etc/UTC").await;
    let account: String = sqlx::query_scalar("INSERT INTO accounts(user_id,name,account_type,currency,archived_at) VALUES($1::uuid,'Archived planning','normal','SEK',statement_timestamp()) RETURNING id::text").bind(owner.user_id()).fetch_one(&pool).await.unwrap();
    sqlx::query("UPDATE recurring_rules SET account_id=$1::uuid WHERE id=$2::uuid")
        .bind(&account)
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(run(&pool, &owner, &id, "2024-01-31T12:00:00Z").await, 1);
    sqlx::query("UPDATE recurring_occurrences SET due_on='2024-02-05' WHERE rule_id=$1::uuid")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    let original: String = sqlx::query_scalar(
        "SELECT to_jsonb(o)::text FROM recurring_occurrences o WHERE rule_id=$1::uuid",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE recurring_rules SET amount_minor=12000,description='Changed',generated_through=NULL WHERE id=$1::uuid").bind(&id).execute(&pool).await.unwrap();
    assert_eq!(run(&pool, &owner, &id, "2024-02-29T12:00:00Z").await, 1);
    let retained: String = sqlx::query_scalar("SELECT to_jsonb(o)::text FROM recurring_occurrences o WHERE rule_id=$1::uuid AND scheduled_on='2024-01-31'").bind(&id).fetch_one(&pool).await.unwrap();
    assert_eq!(original, retained);
    let changed: bool = sqlx::query_scalar("SELECT amount_minor=12000 AND description='Changed' AND account_id=$2::uuid FROM recurring_occurrences WHERE rule_id=$1::uuid AND scheduled_on='2024-02-29'").bind(&id).bind(account).fetch_one(&pool).await.unwrap();
    assert!(changed);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn foreign_unverified_and_invalid_clock_reject_without_mutation() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    let foreign = identity(&pool, true).await;
    let pending = identity(&pool, false).await;
    let id = rule(&pool, &owner, "Pacific/Kiritimati").await;
    let saved = snapshot(&pool, &id).await;
    for who in [&foreign, &pending] {
        assert!(
            generate_due(&pool, who, &id, || instant("2024-03-31T12:00:00Z"))
                .await
                .is_err()
        );
        assert_eq!(snapshot(&pool, &id).await, saved);
    }
    assert!(
        generate_due(&pool, &owner, &id, || instant("9999-12-31T23:30:00Z"))
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, &id).await, saved);
}
#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn late_insert_failure_rolls_back_all_slots_and_progress() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    let id = rule(&pool, &owner, "Etc/UTC").await;
    let saved = snapshot(&pool, &id).await;
    sqlx::raw_sql(&format!("CREATE FUNCTION fail_generation_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.rule_id='{id}'::uuid AND NEW.scheduled_on='2024-02-29' THEN RAISE EXCEPTION 'synthetic generation failure'; END IF; RETURN NEW; END; $$; CREATE TRIGGER fail_generation_test BEFORE INSERT ON recurring_occurrences FOR EACH ROW EXECUTE FUNCTION fail_generation_test();")).execute(&pool).await.unwrap();
    assert!(
        generate_due(&pool, &owner, &id, || instant("2024-03-31T12:00:00Z"))
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, &id).await, saved);
    sqlx::raw_sql("DROP TRIGGER fail_generation_test ON recurring_occurrences; DROP FUNCTION fail_generation_test();").execute(&pool).await.unwrap();
    assert_eq!(run(&pool, &owner, &id, "2024-03-31T12:00:00Z").await, 3);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn every_cadence_keeps_its_anchor_and_extended_ends_recover_backlog() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    for (cadence, expected) in [
        ("weekly", 53),
        ("monthly", 13),
        ("quarterly", 5),
        ("annual", 2),
    ] {
        let id = rule(&pool, &owner, "Etc/UTC").await;
        sqlx::query("UPDATE recurring_rules SET cadence=$1 WHERE id=$2::uuid")
            .bind(cadence)
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            run(&pool, &owner, &id, "2025-01-31T12:00:00Z").await,
            expected
        );
        assert_eq!(slots(&pool, &id).await.first().unwrap(), "2024-01-31");
    }
    let id = rule(&pool, &owner, "Etc/UTC").await;
    sqlx::query("UPDATE recurring_rules SET ends_on='2024-02-15' WHERE id=$1::uuid")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(run(&pool, &owner, &id, "2024-03-31T12:00:00Z").await, 1);
    sqlx::query("UPDATE recurring_rules SET ends_on='2024-03-31' WHERE id=$1::uuid")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(run(&pool, &owner, &id, "2024-03-31T12:00:00Z").await, 2);
    assert_eq!(
        slots(&pool, &id).await,
        ["2024-01-31", "2024-02-29", "2024-03-31"]
    );
    sqlx::query("UPDATE recurring_occurrences SET state='skipped' WHERE rule_id=$1::uuid")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE recurring_rules SET generated_through=NULL WHERE id=$1::uuid")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    let saved: Vec<String> = sqlx::query_scalar("SELECT to_jsonb(o)::text FROM recurring_occurrences o WHERE rule_id=$1::uuid ORDER BY scheduled_on")
        .bind(&id).fetch_all(&pool).await.unwrap();
    assert_eq!(run(&pool, &owner, &id, "2024-03-31T12:00:00Z").await, 0);
    let retained: Vec<String> = sqlx::query_scalar("SELECT to_jsonb(o)::text FROM recurring_occurrences o WHERE rule_id=$1::uuid ORDER BY scheduled_on")
        .bind(&id).fetch_all(&pool).await.unwrap();
    assert_eq!(retained, saved);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_clock_waits_for_each_lock_and_snapshots_serialized_edit() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    for owner_lock in [true, false] {
        let id = rule(&pool, &owner, "Etc/UTC").await;
        let mut holder = pool.begin().await.unwrap();
        if owner_lock {
            sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
                .bind(owner.user_id())
                .execute(&mut *holder)
                .await
                .unwrap();
        } else {
            sqlx::query("SELECT id FROM recurring_rules WHERE id=$1::uuid FOR UPDATE")
                .bind(&id)
                .execute(&mut *holder)
                .await
                .unwrap();
        }
        let called = Arc::new(AtomicBool::new(false));
        let marker = called.clone();
        let work = generate_due(&pool, &owner, &id, move || {
            marker.store(true, Ordering::SeqCst);
            instant("2024-02-29T12:00:00Z")
        });
        tokio::pin!(work);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut work)
                .await
                .is_err()
        );
        assert!(!called.load(Ordering::SeqCst));
        sqlx::query("UPDATE recurring_rules SET description='Serialized edit' WHERE id=$1::uuid")
            .bind(&id)
            .execute(&mut *holder)
            .await
            .unwrap();
        holder.commit().await.unwrap();
        assert_eq!(work.await.unwrap(), 2);
        assert!(called.load(Ordering::SeqCst));
        let edited: bool = sqlx::query_scalar("SELECT bool_and(description='Serialized edit') FROM recurring_occurrences WHERE rule_id=$1::uuid").bind(&id).fetch_one(&pool).await.unwrap();
        assert!(edited);
    }
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_waiting_generation_observes_committed_pause_without_mutation() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    let id = rule(&pool, &owner, "Etc/UTC").await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1::uuid FOR UPDATE")
        .bind(owner.user_id())
        .execute(&mut *holder)
        .await
        .unwrap();
    let work = generate_due(&pool, &owner, &id, || {
        panic!("paused generation must not read the clock")
    });
    tokio::pin!(work);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut work)
            .await
            .is_err()
    );
    sqlx::query(
        "UPDATE recurring_rules SET state='paused',paused_on='2024-02-01' WHERE id=$1::uuid",
    )
    .bind(&id)
    .execute(&mut *holder)
    .await
    .unwrap();
    holder.commit().await.unwrap();
    let before = snapshot(&pool, &id).await;
    let money = financial(&pool).await;
    assert_eq!(work.await.unwrap(), 0);
    assert_eq!(snapshot(&pool, &id).await, before);
    assert_eq!(financial(&pool).await, money);
}

#[tokio::test]
#[ignore = "disposable PostgreSQL infrastructure harness only"]
async fn independent_cancelled_wait_releases_owner_and_preserves_rows() {
    let pool = common::pool().await;
    let owner = identity(&pool, true).await;
    let id = rule(&pool, &owner, "Etc/UTC").await;
    let before = snapshot(&pool, &id).await;
    let money = financial(&pool).await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM recurring_rules WHERE id=$1::uuid FOR UPDATE")
        .bind(&id)
        .execute(&mut *holder)
        .await
        .unwrap();
    {
        let work = generate_due(&pool, &owner, &id, || {
            panic!("blocked generation must not read the clock")
        });
        tokio::pin!(work);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut work)
                .await
                .is_err()
        );
    }
    holder.rollback().await.unwrap();
    assert_eq!(snapshot(&pool, &id).await, before);
    assert_eq!(financial(&pool).await, money);
    assert_eq!(run(&pool, &owner, &id, "2024-02-29T12:00:00Z").await, 2);
}
