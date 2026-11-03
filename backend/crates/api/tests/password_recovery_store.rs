//! Password recovery persistence checks; PostgreSQL cases require the disposable harness.
mod common;

use cuenora_api::{
    login_store::{issue_initial_session, lookup_credentials},
    password_recovery_store,
    registration_policy::{Email, hash_password},
    registration_store::persist_registration,
    session_tokens::generate,
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::Barrier;

struct Account {
    id: String,
}

async fn seed(pool: &PgPool, email: &str) -> Account {
    let email = Email::parse(email).unwrap();
    let hash = hash_password("a synthetic recovery password 2048").unwrap();
    persist_registration(pool, &email, &hash).await.unwrap();
    let credential = lookup_credentials(pool, &email).await.unwrap().unwrap();
    let tokens = generate().unwrap();
    assert!(
        issue_initial_session(pool, &credential, &tokens)
            .await
            .unwrap()
    );
    let id =
        sqlx::query_scalar("SELECT user_id::text FROM session_access_tokens WHERE token_hash=$1")
            .bind(tokens.access.digest().as_slice())
            .fetch_one(pool)
            .await
            .unwrap();
    Account { id }
}

async fn snapshot(pool: &PgPool, user_id: &str) -> String {
    sqlx::query_scalar(
        "SELECT md5(jsonb_build_array(
            (SELECT to_jsonb(u) FROM users u WHERE id=$1::uuid),
            (SELECT to_jsonb(c) FROM password_credentials c WHERE user_id=$1::uuid),
            (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM password_recovery_challenges r WHERE user_id=$1::uuid),
            (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s WHERE user_id=$1::uuid),
            (SELECT coalesce(jsonb_agg(to_jsonb(n) ORDER BY id), '[]') FROM password_reset_notifications n WHERE user_id=$1::uuid)
        )::text)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn wait_for_recovery_lock(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_stat_activity
                 WHERE pid<>pg_backend_pid() AND wait_event_type='Lock'
                   AND query LIKE '%FROM password_credentials c JOIN users u%'",
            )
            .fetch_one(pool)
            .await
            .unwrap();
            if waiting > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("reset did not wait for the credential lock");
}

#[test]
fn recovery_tokens_are_purpose_bound_lowercase_hex() {
    let valid = format!("pr1_{}", "a".repeat(64));
    assert!(password_recovery_store::valid_token(&valid));
    for invalid in [
        format!("ev1_{}", "a".repeat(64)),
        format!("pr1_{}", "A".repeat(64)),
        format!("pr1_{}", "g".repeat(64)),
        format!("pr1_{}", "a".repeat(63)),
        format!("pr1_{}x", "a".repeat(64)),
    ] {
        assert!(!password_recovery_store::valid_token(&invalid));
    }
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn issue_is_canonical_digest_only_and_suppresses_unknown_cooldown_and_caps() {
    let pool = common::pool().await;
    let account = seed(&pool, "Recovery.User@example.test").await;
    assert!(!common::snapshot(&pool).await.is_empty());
    let canonical = Email::parse("recovery.user@EXAMPLE.test").unwrap();
    let issued = password_recovery_store::issue(&pool, &canonical)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(issued.email, "Recovery.User@example.test");
    assert!(issued.token.starts_with("pr1_"));
    assert!(issued.idempotency_key.starts_with("password-recovery-"));
    let digest: Vec<u8> = sqlx::query_scalar(
        "SELECT token_hash FROM password_recovery_challenges WHERE user_id=$1::uuid",
    )
    .bind(&account.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(digest, Sha256::digest(issued.token.as_bytes()).as_slice());
    let before = snapshot(&pool, &account.id).await;
    assert!(
        password_recovery_store::issue(&pool, &canonical)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(&pool, &account.id).await, before);

    let unknown = Email::parse("unknown.recovery@example.test").unwrap();
    assert!(
        password_recovery_store::issue(&pool, &unknown)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(&pool, &account.id).await, before);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn five_live_or_hourly_challenges_suppress_new_delivery() {
    let pool = common::pool().await;
    let _account = seed(&pool, "recovery-cap@example.test").await;
    let email = Email::parse("recovery-cap@example.test").unwrap();
    for _ in 0..5 {
        let challenge = password_recovery_store::issue(&pool, &email)
            .await
            .unwrap()
            .unwrap();
        let digest = Sha256::digest(challenge.token.as_bytes()).to_vec();
        // Retain every challenge as live while allowing the next issuance past cooldown.
        sqlx::query("UPDATE password_recovery_challenges SET created_at=clock_timestamp()-interval '2 minutes' WHERE token_hash=$1")
            .bind(digest).execute(&pool).await.unwrap();
    }
    assert!(
        password_recovery_store::issue(&pool, &email)
            .await
            .unwrap()
            .is_none()
    );
    let _hourly_account = seed(&pool, "recovery-hourly-cap@example.test").await;
    let hourly_email = Email::parse("recovery-hourly-cap@example.test").unwrap();
    for _ in 0..5 {
        let challenge = password_recovery_store::issue(&pool, &hourly_email)
            .await
            .unwrap()
            .unwrap();
        let digest = Sha256::digest(challenge.token.as_bytes()).to_vec();
        sqlx::query("UPDATE password_recovery_challenges SET created_at=clock_timestamp()-interval '2 minutes',consumed_at=clock_timestamp() WHERE token_hash=$1")
            .bind(digest).execute(&pool).await.unwrap();
    }
    assert!(
        password_recovery_store::issue(&pool, &hourly_email)
            .await
            .unwrap()
            .is_none()
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn reset_atomically_changes_hash_revokes_sessions_preserves_verification_and_queues_safe_notice()
 {
    let pool = common::pool().await;
    let account = seed(&pool, "Recovery.Verified@example.test").await;
    sqlx::query("UPDATE users SET email_verified_at=clock_timestamp() WHERE id=$1::uuid")
        .bind(&account.id)
        .execute(&pool)
        .await
        .unwrap();
    let email = Email::parse("recovery.verified@example.test").unwrap();
    let challenge = password_recovery_store::issue(&pool, &email)
        .await
        .unwrap()
        .unwrap();
    // Older live links remain independent and are invalidated only by successful reset.
    let sibling = format!("pr1_{}", "b".repeat(64));
    let sibling_digest = Sha256::digest(sibling.as_bytes()).to_vec();
    sqlx::query(
        "INSERT INTO password_recovery_challenges(user_id,email,credential_revision,token_hash,created_at,expires_at)
         SELECT $1::uuid,email_key,credential_revision,$2,clock_timestamp(),clock_timestamp()+interval '30 minutes'
         FROM users JOIN password_credentials ON users.id=password_credentials.user_id WHERE users.id=$1::uuid",
    )
    .bind(&account.id)
    .bind(sibling_digest)
    .execute(&pool)
    .await
    .unwrap();
    let old_hash: String =
        sqlx::query_scalar("SELECT password_hash FROM password_credentials WHERE user_id=$1::uuid")
            .bind(&account.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let old_revision: i64 = sqlx::query_scalar(
        "SELECT credential_revision FROM password_credentials WHERE user_id=$1::uuid",
    )
    .bind(&account.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let new_hash = hash_password("a different synthetic recovery password 2048").unwrap();
    password_recovery_store::confirm(&pool, &challenge.token, &new_hash)
        .await
        .unwrap();

    let row = sqlx::query(
        "SELECT c.password_hash,c.credential_revision,u.email_verified_at IS NOT NULL AS verified
         FROM password_credentials c JOIN users u ON u.id=c.user_id WHERE c.user_id=$1::uuid",
    )
    .bind(&account.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_ne!(row.get::<String, _>("password_hash"), old_hash);
    assert_eq!(row.get::<i64, _>("credential_revision"), old_revision + 1);
    assert!(row.get::<bool, _>("verified"));
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_sessions WHERE user_id=$1::uuid AND revoked_at IS NULL)",
    )
    .bind(&account.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!active);
    let statuses: Vec<(bool, bool)> = sqlx::query_as(
        "SELECT consumed_at IS NOT NULL,invalidated_at IS NOT NULL FROM password_recovery_challenges WHERE user_id=$1::uuid ORDER BY created_at,id",
    ).bind(&account.id).fetch_all(&pool).await.unwrap();
    assert_eq!(statuses.len(), 2);
    assert!(statuses.iter().any(|(consumed, _)| *consumed));
    assert!(statuses.iter().any(|(_, invalidated)| *invalidated));
    // Earlier cases intentionally leave committed outbox events in this shared
    // disposable database. Drain those before asserting this account's event.
    let notification = loop {
        let event = password_recovery_store::claim_notification(&pool)
            .await
            .unwrap()
            .unwrap();
        if event.email == "Recovery.Verified@example.test" {
            break event;
        }
        password_recovery_store::finish_notification(&pool, &event.id, event.attempt, true)
            .await
            .unwrap();
    };
    assert_eq!(notification.email, "Recovery.Verified@example.test");
    assert!(notification.idempotency_key.starts_with("password-reset-"));
    assert!(!notification.email.contains(&challenge.token));
    assert!(!notification.idempotency_key.contains(&challenge.token));
    sqlx::query("UPDATE password_reset_notifications SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1::uuid")
        .bind(&notification.id).execute(&pool).await.unwrap();
    let renewed = password_recovery_store::claim_notification(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(notification.id, renewed.id);
    assert_eq!(notification.idempotency_key, renewed.idempotency_key);
    let before_stale_finish = snapshot(&pool, &account.id).await;
    assert_eq!(
        password_recovery_store::finish_notification(
            &pool,
            &notification.id,
            notification.attempt,
            true
        )
        .await,
        Err(password_recovery_store::StoreError::InvalidChallenge)
    );
    assert_eq!(snapshot(&pool, &account.id).await, before_stale_finish);
    password_recovery_store::finish_notification(&pool, &renewed.id, renewed.attempt, false)
        .await
        .unwrap();
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM password_reset_notifications WHERE user_id=$1::uuid AND delivered_at IS NULL AND attempts=2",
    ).bind(&account.id).fetch_one(&pool).await.unwrap();
    assert_eq!(pending, 1);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn rejected_reset_and_replay_leave_account_snapshot_unchanged() {
    let pool = common::pool().await;
    let account = seed(&pool, "recovery-reject@example.test").await;
    let email = Email::parse("recovery-reject@example.test").unwrap();
    let challenge = password_recovery_store::issue(&pool, &email)
        .await
        .unwrap()
        .unwrap();
    let new_hash = hash_password("yet another synthetic recovery password 2048").unwrap();
    let before = snapshot(&pool, &account.id).await;
    assert!(
        password_recovery_store::confirm(&pool, "pr1_invalid", &new_hash)
            .await
            .is_err()
    );
    let unknown = format!("pr1_{}", "f".repeat(64));
    assert!(
        password_recovery_store::confirm(&pool, &unknown, &new_hash)
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, &account.id).await, before);
    password_recovery_store::confirm(&pool, &challenge.token, &new_hash)
        .await
        .unwrap();
    let after = snapshot(&pool, &account.id).await;
    assert!(
        password_recovery_store::confirm(&pool, &challenge.token, &new_hash)
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, &account.id).await, after);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn pending_account_reset_keeps_verification_pending_and_failed_outbox_insert_rolls_back() {
    let pool = common::pool().await;
    let account = seed(&pool, "recovery-pending@example.test").await;
    let email = Email::parse("recovery-pending@example.test").unwrap();
    let challenge = password_recovery_store::issue(&pool, &email)
        .await
        .unwrap()
        .unwrap();
    let new_hash = hash_password("pending account recovery password 2048").unwrap();
    let constraint = format!(
        "ALTER TABLE password_reset_notifications ADD CONSTRAINT recovery_test_fail_check CHECK (user_id <> '{}'::uuid)",
        account.id
    );
    sqlx::query(&constraint).execute(&pool).await.unwrap();
    let before = snapshot(&pool, &account.id).await;
    assert!(
        password_recovery_store::confirm(&pool, &challenge.token, &new_hash)
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, &account.id).await, before);
    sqlx::query(
        "ALTER TABLE password_reset_notifications DROP CONSTRAINT recovery_test_fail_check",
    )
    .execute(&pool)
    .await
    .unwrap();
    password_recovery_store::confirm(&pool, &challenge.token, &new_hash)
        .await
        .unwrap();
    let verified: bool =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id=$1::uuid")
            .bind(&account.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!verified);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn reset_rechecks_stale_email_after_wait_and_cancellation_rolls_back() {
    let pool = common::pool().await;
    let account = seed(&pool, "recovery-wait@example.test").await;
    let email = Email::parse("recovery-wait@example.test").unwrap();
    let challenge = password_recovery_store::issue(&pool, &email)
        .await
        .unwrap()
        .unwrap();
    let new_hash = hash_password("wait synthetic recovery password 2048").unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT user_id FROM password_credentials WHERE user_id=$1::uuid FOR UPDATE")
        .bind(&account.id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET email_original='recovery-moved@example.test',email_address='recovery-moved@example.test' WHERE id=$1::uuid")
        .bind(&account.id).execute(&mut *blocker).await.unwrap();
    let pool_wait = pool.clone();
    let token_wait = challenge.token.clone();
    let hash_wait = new_hash.clone();
    let waiting = tokio::spawn(async move {
        password_recovery_store::confirm(&pool_wait, &token_wait, &hash_wait).await
    });
    wait_for_recovery_lock(&pool).await;
    blocker.commit().await.unwrap();
    assert!(waiting.await.unwrap().is_err());

    // Use another fixture: the first account remains inside its intentional
    // one-minute resend cooldown even after an email change.
    let account = seed(&pool, "recovery-cancel@example.test").await;
    let next = Email::parse("recovery-cancel@example.test").unwrap();
    let new_challenge = password_recovery_store::issue(&pool, &next)
        .await
        .unwrap()
        .unwrap();
    let before = snapshot(&pool, &account.id).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT user_id FROM password_credentials WHERE user_id=$1::uuid FOR UPDATE")
        .bind(&account.id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pool_cancel = pool.clone();
    let token_cancel = new_challenge.token.clone();
    let hash_cancel = new_hash;
    let cancelled = tokio::spawn(async move {
        password_recovery_store::confirm(&pool_cancel, &token_cancel, &hash_cancel).await
    });
    wait_for_recovery_lock(&pool).await;
    cancelled.abort();
    let _ = cancelled.await;
    blocker.commit().await.unwrap();
    assert_eq!(snapshot(&pool, &account.id).await, before);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn concurrent_reset_of_one_challenge_has_one_winner() {
    let pool = common::pool().await;
    let account = seed(&pool, "recovery-race@example.test").await;
    let email = Email::parse("recovery-race@example.test").unwrap();
    let challenge = password_recovery_store::issue(&pool, &email)
        .await
        .unwrap()
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let pool = pool.clone();
        let token = challenge.token.clone();
        let barrier = barrier.clone();
        let hash = hash_password("race synthetic recovery password 2048").unwrap();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            password_recovery_store::confirm(&pool, &token, &hash).await
        }));
    }
    barrier.wait().await;
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap());
    }
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let notifications: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM password_reset_notifications WHERE user_id=$1::uuid",
    )
    .bind(&account.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(notifications, 1);
    pool.close().await;
}
