mod common;

use cuenora_api::{
    login_store::{issue_initial_session, lookup_credentials},
    registration_policy::{Email, hash_password},
    registration_store::persist_registration,
    session_store::{SessionStoreUnavailable, revoke, rotate},
    session_tokens::{InitialTokens, generate},
};
use sqlx::{PgPool, Row};
use std::time::Duration;

async fn seed(pool: &PgPool, email: &str) -> InitialTokens {
    let email = Email::parse(email).unwrap();
    let hash = hash_password("synthetic refresh password 42").unwrap();
    persist_registration(pool, &email, &hash).await.unwrap();
    let credential = lookup_credentials(pool, &email).await.unwrap().unwrap();
    let tokens = generate().unwrap();
    assert!(
        issue_initial_session(pool, &credential, &tokens)
            .await
            .unwrap()
    );
    tokens
}

async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM users u), (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY user_id), '[]') FROM password_credentials c), (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s), (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM session_access_tokens a), (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM session_refresh_tokens r))::text)")
        .fetch_one(pool).await.unwrap()
}

async fn family(pool: &PgPool, tokens: &InitialTokens) -> String {
    sqlx::query_scalar("SELECT session_id::text FROM session_refresh_tokens WHERE token_hash=$1")
        .bind(tokens.refresh.digest().as_slice())
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn revoked(pool: &PgPool, id: &str) -> bool {
    sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM user_sessions WHERE id=$1::uuid")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn counts(pool: &PgPool, id: &str) -> (i64, i64) {
    let row = sqlx::query("SELECT (SELECT count(*) FROM session_access_tokens WHERE session_id=$1::uuid) AS a, (SELECT count(*) FROM session_refresh_tokens WHERE session_id=$1::uuid) AS r")
        .bind(id).fetch_one(pool).await.unwrap();
    (row.get("a"), row.get("r"))
}

async fn wait_for_family_locks(pool: &PgPool, count: i64) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE '%refresh_family_lock%'")
                .fetch_one(pool).await.unwrap();
            if waiting >= count { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("refresh operations did not reach the family lock");
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn rotations_slide_thirty_days_without_family_age_limit_and_store_only_digests() {
    let pool = common::pool().await;
    let mut tokens = seed(&pool, "sliding@example.test").await;
    let id = family(&pool, &tokens).await;
    sqlx::query("UPDATE user_sessions SET created_at=clock_timestamp()-interval '90 days' WHERE id=$1::uuid")
        .bind(&id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE session_refresh_tokens SET created_at=clock_timestamp()-interval '29 days', expires_at=clock_timestamp()+interval '1 day' WHERE session_id=$1::uuid")
        .bind(&id).execute(&pool).await.unwrap();
    let identity = common::snapshot(&pool).await;
    let original_access = tokens.access.digest();
    for generation in 2..=3 {
        let next = rotate(&pool, tokens.refresh.expose_secret())
            .await
            .unwrap()
            .expect("valid refresh must rotate");
        assert!(next.access.expose_secret() != tokens.access.expose_secret());
        assert!(next.refresh.expose_secret() != tokens.refresh.expose_secret());
        assert_eq!(family(&pool, &next).await, id);
        assert_eq!(counts(&pool, &id).await, (generation, generation));
        let valid: bool = sqlx::query_scalar("SELECT r.used_at = n.created_at AND r.replaced_by=n.id AND n.used_at IS NULL AND n.expires_at-n.created_at=interval '30 days' AND n.created_at>=clock_timestamp()-interval '1 minute' AND a.created_at=n.created_at AND a.expires_at-a.created_at=interval '15 minutes' FROM session_refresh_tokens r JOIN session_refresh_tokens n ON n.id=r.replaced_by JOIN session_access_tokens a ON a.session_id=n.session_id AND a.token_hash=$3 WHERE r.token_hash=$1 AND n.token_hash=$2")
            .bind(tokens.refresh.digest().as_slice()).bind(next.refresh.digest().as_slice()).bind(next.access.digest().as_slice())
            .fetch_one(&pool).await.unwrap();
        assert!(valid);
        assert_eq!(common::snapshot(&pool).await, identity);
        let stored: String = sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(a)) FROM session_access_tokens a), (SELECT jsonb_agg(to_jsonb(r)) FROM session_refresh_tokens r))::text")
            .fetch_one(&pool).await.unwrap();
        for secret in [
            tokens.access.expose_secret(),
            tokens.refresh.expose_secret(),
            next.access.expose_secret(),
            next.refresh.expose_secret(),
        ] {
            assert!(!stored.contains(secret));
        }
        tokens = next;
    }
    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM session_access_tokens WHERE token_hash=$1")
            .bind(original_access.as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retained, 1);
    assert!(!revoked(&pool, &id).await);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn replay_and_explicit_revocation_preserve_other_sessions_and_identity() {
    let pool = common::pool().await;
    let original = seed(&pool, "replay@example.test").await;
    let same_user = seed(&pool, "replay@example.test").await;
    let other_user = seed(&pool, "other-replay@example.test").await;
    let id = family(&pool, &original).await;
    let same_id = family(&pool, &same_user).await;
    let other_id = family(&pool, &other_user).await;
    let identity = common::snapshot(&pool).await;
    let successor = rotate(&pool, original.refresh.expose_secret())
        .await
        .unwrap()
        .unwrap();
    // Even an old, expired consumed generation still detects reuse.
    sqlx::query("UPDATE session_refresh_tokens SET created_at=clock_timestamp()-interval '31 days', expires_at=clock_timestamp()-interval '1 day' WHERE token_hash=$1")
        .bind(original.refresh.digest().as_slice()).execute(&pool).await.unwrap();
    assert!(
        rotate(&pool, original.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert!(revoked(&pool, &id).await);
    assert!(!revoked(&pool, &same_id).await);
    assert!(!revoked(&pool, &other_id).await);
    let before = snapshot(&pool).await;
    assert!(
        rotate(&pool, successor.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        rotate(&pool, original.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        revoke(&pool, successor.refresh.expose_secret())
            .await
            .unwrap()
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(counts(&pool, &id).await, (2, 2));
    assert!(
        rotate(&pool, same_user.refresh.expose_secret())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        revoke(&pool, other_user.refresh.expose_secret())
            .await
            .unwrap()
    );
    let before = snapshot(&pool).await;
    assert!(
        rotate(&pool, other_user.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(common::snapshot(&pool).await, identity);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn malformed_unknown_expired_tokens_and_database_failures_do_not_write() {
    let pool = common::pool().await;
    let tokens = seed(&pool, "invalid-refresh@example.test").await;
    let before = snapshot(&pool).await;
    let unknown = generate().unwrap();
    for value in [
        "",
        "cr1_",
        "cr1_0123' OR 1=1",
        &format!("cr1_{}", "A".repeat(64)),
        tokens.access.expose_secret(),
        unknown.refresh.expose_secret(),
    ] {
        assert!(rotate(&pool, value).await.unwrap().is_none());
        assert!(!revoke(&pool, value).await.unwrap());
        assert_eq!(snapshot(&pool).await, before);
    }
    for (table, event) in [
        ("session_refresh_tokens", "INSERT"),
        ("session_access_tokens", "INSERT"),
        ("session_refresh_tokens", "UPDATE"),
        ("user_sessions", "UPDATE"),
    ] {
        sqlx::raw_sql(&format!("CREATE FUNCTION reject_refresh_write() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END $$; CREATE TRIGGER reject_refresh_write BEFORE {event} ON {table} FOR EACH ROW EXECUTE FUNCTION reject_refresh_write();"))
            .execute(&pool).await.unwrap();
        let result = if table == "user_sessions" {
            revoke(&pool, tokens.refresh.expose_secret()).await.err()
        } else {
            rotate(&pool, tokens.refresh.expose_secret()).await.err()
        };
        let after = snapshot(&pool).await;
        sqlx::raw_sql(&format!(
            "DROP TRIGGER reject_refresh_write ON {table}; DROP FUNCTION reject_refresh_write();"
        ))
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(result, Some(SessionStoreUnavailable));
        assert_eq!(after, before);
    }
    sqlx::query("UPDATE session_refresh_tokens SET created_at=clock_timestamp()-interval '31 days', expires_at=clock_timestamp() WHERE token_hash=$1")
        .bind(tokens.refresh.digest().as_slice()).execute(&pool).await.unwrap();
    let expired = snapshot(&pool).await;
    assert!(
        rotate(&pool, tokens.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(&pool).await, expired);
    assert_eq!(
        format!("{:?}", SessionStoreUnavailable),
        "SessionStoreUnavailable"
    );
    pool.close().await;
    assert_eq!(
        rotate(&pool, tokens.refresh.expose_secret()).await.err(),
        Some(SessionStoreUnavailable)
    );
    assert_eq!(
        revoke(&pool, tokens.refresh.expose_secret()).await.err(),
        Some(SessionStoreUnavailable)
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn simultaneous_rotation_has_one_winner_and_replay_revokes_its_successor() {
    let pool = common::pool().await;
    let tokens = seed(&pool, "concurrent-refresh@example.test").await;
    let id = family(&pool, &tokens).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM user_sessions WHERE id=$1::uuid FOR UPDATE")
        .bind(&id)
        .execute(&mut *gate)
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let pool = pool.clone();
        let refresh = tokens.refresh.expose_secret().to_owned();
        tasks.push(tokio::spawn(async move {
            rotate(&pool, &refresh).await.unwrap()
        }));
    }
    wait_for_family_locks(&pool, 2).await;
    gate.commit().await.unwrap();
    let a = tasks.remove(0).await.unwrap();
    let b = tasks.remove(0).await.unwrap();
    assert_eq!(usize::from(a.is_some()) + usize::from(b.is_some()), 1);
    let winner = a.or(b).unwrap();
    assert!(revoked(&pool, &id).await);
    assert_eq!(counts(&pool, &id).await, (2, 2));
    let before = snapshot(&pool).await;
    assert!(
        rotate(&pool, winner.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(&pool).await, before);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn revocation_expiry_and_cancellation_serialize_against_rotation() {
    let pool = common::pool().await;
    let tokens = seed(&pool, "revoke-race@example.test").await;
    let id = family(&pool, &tokens).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM user_sessions WHERE id=$1::uuid FOR UPDATE")
        .bind(&id)
        .execute(&mut *gate)
        .await
        .unwrap();
    let rotate_pool = pool.clone();
    let rotate_secret = tokens.refresh.expose_secret().to_owned();
    let rotate_task =
        tokio::spawn(async move { rotate(&rotate_pool, &rotate_secret).await.unwrap() });
    let revoke_pool = pool.clone();
    let revoke_secret = tokens.refresh.expose_secret().to_owned();
    let revoke_task =
        tokio::spawn(async move { revoke(&revoke_pool, &revoke_secret).await.unwrap() });
    wait_for_family_locks(&pool, 2).await;
    gate.commit().await.unwrap();
    let result = rotate_task.await.unwrap();
    assert!(revoke_task.await.unwrap());
    assert!(revoked(&pool, &id).await);
    let expected = if result.is_some() { 2 } else { 1 };
    assert_eq!(counts(&pool, &id).await, (expected, expected));
    let before = snapshot(&pool).await;
    if let Some(next) = result {
        assert!(
            rotate(&pool, next.refresh.expose_secret())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(snapshot(&pool).await, before);
    }
    for action in ["revoke", "expire", "cancel"] {
        let tokens = seed(&pool, &format!("{action}-refresh@example.test")).await;
        let id = family(&pool, &tokens).await;
        let mut gate = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM user_sessions WHERE id=$1::uuid FOR UPDATE")
            .bind(&id)
            .execute(&mut *gate)
            .await
            .unwrap();
        let task_pool = pool.clone();
        let refresh = tokens.refresh.expose_secret().to_owned();
        let task = tokio::spawn(async move { rotate(&task_pool, &refresh).await });
        wait_for_family_locks(&pool, 1).await;
        if action == "cancel" {
            task.abort();
            assert!(matches!(task.await, Err(error) if error.is_cancelled()));
            gate.commit().await.unwrap();
            assert!(
                rotate(&pool, tokens.refresh.expose_secret())
                    .await
                    .unwrap()
                    .is_some()
            );
        } else {
            if action == "revoke" {
                sqlx::query(
                    "UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1::uuid",
                )
                .bind(&id)
                .execute(&mut *gate)
                .await
                .unwrap();
            } else {
                sqlx::query("UPDATE session_refresh_tokens SET created_at=clock_timestamp()-interval '31 days', expires_at=clock_timestamp() WHERE session_id=$1::uuid")
                    .bind(&id).execute(&mut *gate).await.unwrap();
            }
            gate.commit().await.unwrap();
            let before = snapshot(&pool).await;
            assert!(task.await.unwrap().unwrap().is_none());
            assert_eq!(snapshot(&pool).await, before);
        }
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn failed_replay_revocation_rolls_back_and_can_be_retried() {
    let pool = common::pool().await;
    let original = seed(&pool, "replay-failure@example.test").await;
    let successor = rotate(&pool, original.refresh.expose_secret())
        .await
        .unwrap()
        .unwrap();
    let id = family(&pool, &original).await;
    let before = snapshot(&pool).await;
    sqlx::raw_sql("CREATE FUNCTION reject_replay_revoke() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END $$; CREATE TRIGGER reject_replay_revoke BEFORE UPDATE ON user_sessions FOR EACH ROW EXECUTE FUNCTION reject_replay_revoke();")
        .execute(&pool).await.unwrap();
    let result = rotate(&pool, original.refresh.expose_secret()).await.err();
    let after = snapshot(&pool).await;
    sqlx::raw_sql(
        "DROP TRIGGER reject_replay_revoke ON user_sessions; DROP FUNCTION reject_replay_revoke();",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(result, Some(SessionStoreUnavailable));
    assert_eq!(after, before);
    assert!(!revoked(&pool, &id).await);
    assert!(
        rotate(&pool, original.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert!(revoked(&pool, &id).await);
    let revoked_state = snapshot(&pool).await;
    assert!(
        rotate(&pool, successor.refresh.expose_secret())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(&pool).await, revoked_state);
    pool.close().await;
}
