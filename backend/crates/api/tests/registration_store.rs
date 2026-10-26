//! Run only through the disposable PostgreSQL infrastructure harness.

use cuenora_api::{
    registration_policy::{Email, hash_password},
    registration_store::persist_registration,
};
use sqlx::Row;
mod common;
use common::{pool, snapshot};

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn registration_creates_only_pending_identity_and_hashed_credential() {
    let pool = pool().await;
    let email = Email::parse("O'Connor+tag@mañana.Example").unwrap();
    let hash = hash_password("violet hills beside summer rivers").unwrap();
    persist_registration(&pool, &email, &hash).await.unwrap();
    let row = sqlx::query("SELECT id::text, email_original, email_address, email_key, email_verified_at IS NULL AND profile_time_zone IS NULL AS pending, password_hash FROM users JOIN password_credentials ON user_id = id WHERE email_key = $1")
        .bind("o'connor+tag@xn--maana-pta.example").fetch_one(&pool).await.unwrap();
    assert_eq!(row.get::<String, _>("email_original"), email.original());
    assert_eq!(row.get::<String, _>("email_address"), email.address());
    assert_eq!(row.get::<String, _>("password_hash"), hash);
    assert!(row.get::<bool, _>("pending"));
    assert_eq!(row.get::<String, _>("id").len(), 36);
    let financial_and_sessions: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM accounts) + (SELECT count(*) FROM transactions) + (SELECT count(*) FROM account_movements) + (SELECT count(*) FROM user_sessions) + (SELECT count(*) FROM session_refresh_tokens) + (SELECT count(*) FROM transaction_audit)")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(financial_and_sessions, 0);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn duplicate_and_concurrent_registration_never_replace_existing_credentials() {
    let pool = pool().await;
    let email = Email::parse("Case+tag@bücher.example").unwrap();
    let equivalent = Email::parse("CASE+TAG@xn--bcher-kva.example").unwrap();
    let first = hash_password("lavender moss across frozen rivers").unwrap();
    let second = hash_password("separate tulips beside quiet mountains").unwrap();
    persist_registration(&pool, &email, &first).await.unwrap();
    sqlx::query("UPDATE users SET email_verified_at = '2026-10-25T12:00:00Z', profile_time_zone = 'Europe/Stockholm' WHERE email_key = $1")
        .bind("case+tag@xn--bcher-kva.example").execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    persist_registration(&pool, &equivalent, &second)
        .await
        .unwrap();
    assert_eq!(snapshot(&pool).await, before);
    let mut jobs = Vec::new();
    for index in 0..16 {
        let pool = pool.clone();
        let hash = if index % 2 == 0 {
            first.clone()
        } else {
            second.clone()
        };
        jobs.push(tokio::spawn(async move {
            let address = if index % 2 == 0 {
                "Race@mañana.example"
            } else {
                "RACE@xn--maana-pta.example"
            };
            let email = Email::parse(address).unwrap();
            persist_registration(&pool, &email, &hash).await
        }));
    }
    for job in jobs {
        assert!(job.await.unwrap().is_ok());
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users JOIN password_credentials ON user_id = id WHERE email_key = 'race@xn--maana-pta.example'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    let stored: String = sqlx::query_scalar("SELECT password_hash FROM password_credentials JOIN users ON user_id = id WHERE email_key = 'race@xn--maana-pta.example'")
        .fetch_one(&pool).await.unwrap();
    assert!(stored == first || stored == second);
    let orphan_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users LEFT JOIN password_credentials ON user_id = id WHERE user_id IS NULL")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(orphan_count, 0);
    for distinct in ["Case@bücher.example", "C.ase+tag@bücher.example"] {
        persist_registration(&pool, &Email::parse(distinct).unwrap(), &first)
            .await
            .unwrap();
    }
    let distinct_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM users WHERE email_key LIKE '%@xn--bcher-kva.example'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(distinct_count, 3);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn credential_failure_rolls_back_the_new_user_and_returns_only_safe_error() {
    let pool = pool().await;
    let preserved = Email::parse("preserved@example.test").unwrap();
    let hash = hash_password("red maple forests beside snowy hills").unwrap();
    persist_registration(&pool, &preserved, &hash)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    let email = Email::parse("failed@example.test").unwrap();
    let error = persist_registration(&pool, &email, "not-a-PHC-hash")
        .await
        .unwrap_err();
    assert_eq!(format!("{error:?}"), "RegistrationUnavailable");
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("CREATE FUNCTION reject_registration_credential() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic rejection'; END $$")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_registration_credential BEFORE INSERT ON password_credentials FOR EACH ROW EXECUTE FUNCTION reject_registration_credential()")
        .execute(&pool).await.unwrap();
    let error = persist_registration(&pool, &email, &hash)
        .await
        .unwrap_err();
    assert_eq!(format!("{error:?}"), "RegistrationUnavailable");
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query("DROP TRIGGER reject_registration_credential ON password_credentials")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_registration_credential()")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL infrastructure harness"]
async fn unavailable_database_returns_safe_error_without_creating_identity() {
    let pool = pool().await;
    let preserved = Email::parse("before-close@example.test").unwrap();
    let hash = hash_password("cedar hills beside a golden meadow").unwrap();
    persist_registration(&pool, &preserved, &hash)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    let email = Email::parse("unavailable@example.test").unwrap();
    pool.close().await;
    assert!(persist_registration(&pool, &email, "unused").await.is_err());
    let reopened = self::pool().await;
    assert_eq!(snapshot(&reopened).await, before);
    reopened.close().await;
}
