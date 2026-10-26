mod common;

use cuenora_api::{
    login_credentials,
    login_store::{issue_initial_session, lookup_credentials},
    registration_policy::{Email, hash_password},
    registration_store::persist_registration,
    session_tokens::{
        ACCESS_TOKEN_LIFETIME_SECONDS, InitialTokens, REFRESH_TOKEN_LIFETIME_SECONDS, generate,
    },
};
use sqlx::{PgPool, Row};
use std::time::Duration;

const PASSWORD: &str = "synthetic login password 42";

async fn seed(pool: &PgPool, address: &str, verified: bool) -> Email {
    let email = Email::parse(address).unwrap();
    let encoded = hash_password(PASSWORD).unwrap();
    persist_registration(pool, &email, &encoded).await.unwrap();
    if verified {
        sqlx::query("UPDATE users SET email_verified_at = statement_timestamp() WHERE lower(email_key) = lower($1)")
            .bind(email.address()).execute(pool).await.unwrap();
    }
    email
}

async fn records(pool: &PgPool) -> (i64, i64, i64, i64, i64) {
    let row = sqlx::query("SELECT (SELECT count(*) FROM users) AS users, (SELECT count(*) FROM password_credentials) AS credentials, (SELECT count(*) FROM user_sessions) AS sessions, (SELECT count(*) FROM session_access_tokens) AS access, (SELECT count(*) FROM session_refresh_tokens) AS refresh")
        .fetch_one(pool).await.unwrap();
    (
        row.get("users"),
        row.get("credentials"),
        row.get("sessions"),
        row.get("access"),
        row.get("refresh"),
    )
}

async fn seed_and_snapshot(pool: &PgPool) -> String {
    common::snapshot(pool).await
}

async fn full_snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM users u), (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY user_id), '[]') FROM password_credentials c), (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s), (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM session_access_tokens a), (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM session_refresh_tokens r))::text)")
        .fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn lookup_matches_generated_identity_without_mutating_unknown_or_missing_credentials() {
    let pool = common::pool().await;
    let email = seed(&pool, "Ana+tag@mañana.example", false).await;
    let before = seed_and_snapshot(&pool).await;
    let baseline = records(&pool).await;
    assert!(
        lookup_credentials(&pool, &Email::parse("nobody@example.test").unwrap())
            .await
            .unwrap()
            .is_none()
    );
    let case_variant = Email::parse("aNA+tag@XN--MAANA-PTA.EXAMPLE").unwrap();
    let snapshot = lookup_credentials(&pool, &case_variant)
        .await
        .unwrap()
        .unwrap();
    assert!(login_credentials::verify_password(PASSWORD, Some(snapshot.encoded_hash())).unwrap());
    assert!(
        lookup_credentials(&pool, &Email::parse("ana@example.test").unwrap())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        lookup_credentials(&pool, &Email::parse("ana+other@mañana.example").unwrap())
            .await
            .unwrap()
            .is_none()
    );
    // A profile without credentials is not repaired or linked by lookup.
    sqlx::query("INSERT INTO users (email_original, email_address) VALUES ('missing@example.test', 'missing@example.test')")
        .execute(&pool).await.unwrap();
    let after_insert = seed_and_snapshot(&pool).await;
    let all_before_lookup = full_snapshot(&pool).await;
    assert!(
        lookup_credentials(&pool, &Email::parse("missing@example.test").unwrap())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(records(&pool).await.2, baseline.2);
    assert_eq!(full_snapshot(&pool).await, all_before_lookup);
    assert_eq!(seed_and_snapshot(&pool).await, after_insert);
    assert_ne!(before, after_insert);
    assert_eq!(email.address(), "Ana+tag@xn--maana-pta.example");
    let dotted = seed(&pool, "Dot.Name@mañana.example", false).await;
    let before_dots = full_snapshot(&pool).await;
    assert!(lookup_credentials(&pool, &dotted).await.unwrap().is_some());
    assert!(
        lookup_credentials(&pool, &Email::parse("dotname@mañana.example").unwrap())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(full_snapshot(&pool).await, before_dots);

    // The generated identity key uses C casing, regardless of database locale.
    sqlx::query(
        "CREATE DATABASE cuenora_login_tr TEMPLATE template0 LOCALE_PROVIDER icu ICU_LOCALE 'tr'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let locale_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(
            (*pool.connect_options())
                .clone()
                .database("cuenora_login_tr"),
        )
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../../../migrations/0001_users_identity.sql"))
        .execute(&locale_pool)
        .await
        .unwrap();
    let locale_email = seed(&locale_pool, "IDA@example.test", false).await;
    let locale_lower: String = sqlx::query_scalar("SELECT lower('I')")
        .fetch_one(&locale_pool)
        .await
        .unwrap();
    assert_eq!(locale_lower, "ı");
    let stored_key: String = sqlx::query_scalar("SELECT email_key FROM users")
        .fetch_one(&locale_pool)
        .await
        .unwrap();
    assert_eq!(stored_key, "ida@example.test");
    assert!(
        lookup_credentials(&locale_pool, &locale_email)
            .await
            .unwrap()
            .is_some()
    );
    locale_pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn issuance_creates_pending_and_verified_families_with_digest_only_storage() {
    let pool = common::pool().await;
    let pending = seed(&pool, "pending@example.test", false).await;
    let verified = seed(&pool, "verified@example.test", true).await;
    let baseline = records(&pool).await;
    let identities_before = common::snapshot(&pool).await;
    let mut expected_sessions = baseline.2;
    for (email, expected_verified) in [(pending, false), (verified, true)] {
        let credential = lookup_credentials(&pool, &email).await.unwrap().unwrap();
        let tokens = generate().unwrap();
        let access_digest = tokens.access.digest();
        let refresh_digest = tokens.refresh.digest();
        assert!(
            issue_initial_session(&pool, &credential, &tokens)
                .await
                .unwrap()
        );
        expected_sessions += 1;
        assert_eq!(records(&pool).await.2, expected_sessions);
        let row = sqlx::query("SELECT s.user_id::text AS user_id, u.email_verified_at IS NOT NULL AS verified, a.token_hash AS access_hash, r.token_hash AS refresh_hash, extract(epoch FROM (a.expires_at-a.created_at))::bigint AS access_seconds, extract(epoch FROM (r.expires_at-r.created_at))::bigint AS refresh_seconds, s.created_at = a.created_at AS same_start FROM user_sessions s JOIN users u ON u.id=s.user_id JOIN session_access_tokens a ON a.session_id=s.id JOIN session_refresh_tokens r ON r.session_id=s.id WHERE lower(u.email_key)=lower($1)")
            .bind(email.address()).fetch_one(&pool).await.unwrap();
        assert_eq!(row.get::<bool, _>("verified"), expected_verified);
        assert_eq!(row.get::<Vec<u8>, _>("access_hash"), access_digest);
        assert_eq!(row.get::<Vec<u8>, _>("refresh_hash"), refresh_digest);
        assert_eq!(
            row.get::<i64, _>("access_seconds"),
            ACCESS_TOKEN_LIFETIME_SECONDS as i64
        );
        assert_eq!(
            row.get::<i64, _>("refresh_seconds"),
            REFRESH_TOKEN_LIFETIME_SECONDS as i64
        );
        assert!(row.get::<bool, _>("same_start"));
        assert_eq!(
            row.get::<String, _>("user_id"),
            sqlx::query_scalar::<_, String>(
                "SELECT id::text FROM users WHERE lower(email_key)=lower($1)"
            )
            .bind(email.address())
            .fetch_one(&pool)
            .await
            .unwrap()
        );
    }
    assert_eq!(
        records(&pool).await,
        (
            baseline.0,
            baseline.1,
            baseline.2 + 2,
            baseline.3 + 2,
            baseline.4 + 2
        )
    );
    assert_eq!(common::snapshot(&pool).await, identities_before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn stale_revisions_storage_failures_and_unavailable_pool_leave_no_partial_rows() {
    let pool = common::pool().await;
    let email = seed(&pool, "stale@example.test", false).await;
    let hash_snapshot = lookup_credentials(&pool, &email).await.unwrap().unwrap();
    let before = records(&pool).await;
    let replacement = hash_password("another synthetic password 43").unwrap();
    sqlx::query("UPDATE password_credentials SET password_hash=$1 WHERE user_id=(SELECT id FROM users WHERE email_key=lower($2))")
        .bind(replacement).bind(email.address()).execute(&pool).await.unwrap();
    let before_hash_check = full_snapshot(&pool).await;
    assert!(
        !issue_initial_session(&pool, &hash_snapshot, &generate().unwrap())
            .await
            .unwrap()
    );
    assert_eq!(full_snapshot(&pool).await, before_hash_check);
    // Exact revisions also apply before the Unix epoch.
    sqlx::query("UPDATE password_credentials SET password_changed_at='1969-12-31 23:59:59.123456+00' WHERE user_id=(SELECT id FROM users WHERE email_key=lower($1))")
        .bind(email.address()).execute(&pool).await.unwrap();
    let timestamp_snapshot = lookup_credentials(&pool, &email).await.unwrap().unwrap();
    sqlx::query("UPDATE password_credentials SET password_changed_at=password_changed_at + interval '1 microsecond' WHERE user_id=(SELECT id FROM users WHERE email_key=lower($1))")
        .bind(email.address()).execute(&pool).await.unwrap();
    let before_timestamp_check = full_snapshot(&pool).await;
    assert!(
        !issue_initial_session(&pool, &timestamp_snapshot, &generate().unwrap())
            .await
            .unwrap()
    );
    assert_eq!(records(&pool).await, before);
    assert_eq!(full_snapshot(&pool).await, before_timestamp_check);

    let current = lookup_credentials(&pool, &email).await.unwrap().unwrap();
    let generated = generate().unwrap();
    let swapped = InitialTokens {
        access: generated.refresh,
        refresh: generated.access,
    };
    let before_swapped = full_snapshot(&pool).await;
    assert!(
        issue_initial_session(&pool, &current, &swapped)
            .await
            .is_err()
    );
    assert_eq!(full_snapshot(&pool).await, before_swapped);
    let first = generate().unwrap();
    assert!(
        issue_initial_session(&pool, &current, &first)
            .await
            .unwrap()
    );
    let current = lookup_credentials(&pool, &email).await.unwrap().unwrap();
    let duplicate = generate().unwrap();
    // Force a confirmed refresh-digest uniqueness failure after session/access inserts.
    sqlx::query("UPDATE session_refresh_tokens SET token_hash=$1 WHERE token_hash=$2")
        .bind(duplicate.refresh.digest().to_vec())
        .bind(first.refresh.digest().to_vec())
        .execute(&pool)
        .await
        .unwrap();
    let before_collision = records(&pool).await;
    let before_collision_snapshot = full_snapshot(&pool).await;
    assert!(
        issue_initial_session(&pool, &current, &duplicate)
            .await
            .is_err()
    );
    assert_eq!(records(&pool).await, before_collision);
    assert_eq!(full_snapshot(&pool).await, before_collision_snapshot);

    let access_collision = InitialTokens {
        access: first.access,
        refresh: generate().unwrap().refresh,
    };
    assert!(
        issue_initial_session(&pool, &current, &access_collision)
            .await
            .is_err()
    );
    assert_eq!(full_snapshot(&pool).await, before_collision_snapshot);

    // A late storage failure must also roll back trigger-side identity changes.
    sqlx::raw_sql("CREATE FUNCTION reject_login_refresh() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN UPDATE users SET email_verified_at=statement_timestamp(); RAISE EXCEPTION 'synthetic failure'; END $$; CREATE TRIGGER reject_login_refresh BEFORE INSERT ON session_refresh_tokens FOR EACH ROW EXECUTE FUNCTION reject_login_refresh();")
        .execute(&pool).await.unwrap();
    assert!(
        issue_initial_session(&pool, &current, &generate().unwrap())
            .await
            .is_err()
    );
    assert_eq!(full_snapshot(&pool).await, before_collision_snapshot);
    sqlx::raw_sql("DROP TRIGGER reject_login_refresh ON session_refresh_tokens; DROP FUNCTION reject_login_refresh();")
        .execute(&pool).await.unwrap();

    let unavailable = common::pool().await;
    unavailable.close().await;
    assert!(lookup_credentials(&unavailable, &email).await.is_err());
    assert!(
        issue_initial_session(&unavailable, &current, &generate().unwrap())
            .await
            .is_err()
    );
    assert_eq!(records(&pool).await.2, before.2 + 1);
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn concurrent_fresh_issuance_and_password_change_are_serialized() {
    let pool = common::pool().await;
    let email = seed(&pool, "race@example.test", false).await;
    let baseline = records(&pool).await;
    let credential = lookup_credentials(&pool, &email).await.unwrap().unwrap();
    let tokens_a = generate().unwrap();
    let tokens_b = generate().unwrap();
    assert_ne!(tokens_a.access.digest(), tokens_b.access.digest());
    let (a, b) = tokio::join!(
        issue_initial_session(&pool, &credential, &tokens_a),
        issue_initial_session(&pool, &credential, &tokens_b)
    );
    assert!(a.unwrap());
    assert!(b.unwrap());
    assert_eq!(records(&pool).await.2, baseline.2 + 2);

    let stale = lookup_credentials(&pool, &email).await.unwrap().unwrap();
    let mut tx = pool.begin().await.unwrap();
    let replacement = hash_password("synthetic concurrent replacement 44").unwrap();
    sqlx::query("UPDATE password_credentials SET password_hash=$1 WHERE user_id=(SELECT id FROM users WHERE email_key=lower($2))")
        .bind(replacement).bind(email.address()).execute(&mut *tx).await.unwrap();
    let task_pool = pool.clone();
    let task = tokio::spawn(async move {
        issue_initial_session(&task_pool, &stale, &generate().unwrap()).await
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'WITH current_credential%')")
                .fetch_one(&pool).await.unwrap();
            if waiting { break; }
            assert!(!task.is_finished(), "issuance did not wait on the credential row lock");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("credential row-lock wait was not observed");
    tx.commit().await.unwrap();
    assert!(!task.await.unwrap().unwrap());
    assert_eq!(records(&pool).await.2, baseline.2 + 2);
}
