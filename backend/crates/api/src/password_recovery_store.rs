//! Transactional storage for single-use password recovery and safe reset notices.

use crate::registration_policy::Email;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

const RECOVERY_LIFETIME_MINUTES: i64 = 30;
const SEND_COOLDOWN_SECONDS: i64 = 60;
const SEND_WINDOW_HOURS: i64 = 1;
const SEND_WINDOW_LIMIT: i64 = 5;
const LIVE_CHALLENGE_LIMIT: i64 = 5;
const NOTIFICATION_LEASE_SECONDS: i64 = 120;
const NOTIFICATION_RETRY_SECONDS: i64 = 60;
const NOTIFICATION_ATTEMPT_LIMIT: i16 = 5;

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    Unavailable,
    InvalidChallenge,
}

pub struct IssuedChallenge {
    pub email: String,
    pub token: String,
    pub idempotency_key: String,
}

pub struct Notification {
    pub id: String,
    pub attempt: i16,
    pub email: String,
    pub idempotency_key: String,
}

struct SecretToken(String);

impl SecretToken {
    fn expose_secret(&self) -> &str {
        &self.0
    }

    fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.0.as_bytes()).into()
    }
}

fn generate_token() -> Result<SecretToken, StoreError> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| StoreError::Unavailable)?;
    let mut token = String::with_capacity(68);
    token.push_str("pr1_");
    for byte in bytes {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        token.push(HEX[(byte >> 4) as usize] as char);
        token.push(HEX[(byte & 15) as usize] as char);
    }
    Ok(SecretToken(token))
}

pub fn valid_token(token: &str) -> bool {
    token.len() == 68
        && token.starts_with("pr1_")
        && token[4..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn token_digest(token: &str) -> Option<[u8; 32]> {
    valid_token(token).then(|| Sha256::digest(token.as_bytes()).into())
}

/// Issue a bound challenge when the known account's cooldown and quotas allow it.
/// Unknown addresses and suppressed requests are read-only and return `Ok(None)`.
pub async fn issue(pool: &PgPool, email: &Email) -> Result<Option<IssuedChallenge>, StoreError> {
    let token = generate_token()?;
    let mut tx = pool.begin().await.map_err(|_| StoreError::Unavailable)?;
    read_committed(&mut tx).await?;

    let account = sqlx::query(
        "SELECT u.id::text AS user_id, u.email_key, u.email_address,
                c.credential_revision
         FROM users u JOIN password_credentials c ON c.user_id=u.id
         WHERE u.email_key=lower($1 COLLATE \"C\")
         FOR UPDATE OF c,u",
    )
    .bind(email.address())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    let Some(account) = account else {
        return Ok(None);
    };
    let user_id: String = account
        .try_get("user_id")
        .map_err(|_| StoreError::Unavailable)?;
    let email_key: String = account
        .try_get("email_key")
        .map_err(|_| StoreError::Unavailable)?;
    let recipient: String = account
        .try_get("email_address")
        .map_err(|_| StoreError::Unavailable)?;
    let revision: i64 = account
        .try_get("credential_revision")
        .map_err(|_| StoreError::Unavailable)?;

    lock_challenges(&mut tx, &user_id).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp()) * 1000000)::bigint")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| StoreError::Unavailable)?;
    let suppressed: bool = sqlx::query_scalar(
        "SELECT
            EXISTS (SELECT 1 FROM password_recovery_challenges
                    WHERE user_id=$1::uuid
                      AND extract(epoch FROM created_at)*1000000 > $2::numeric-($5::numeric*1000000))
            OR (SELECT count(*) >= $3 FROM password_recovery_challenges
                WHERE user_id=$1::uuid
                  AND extract(epoch FROM created_at)*1000000 >= $2::numeric-($6::numeric*3600000000))
            OR (SELECT count(*) >= $4 FROM password_recovery_challenges
                WHERE user_id=$1::uuid AND consumed_at IS NULL AND invalidated_at IS NULL
                  AND extract(epoch FROM expires_at)*1000000 > $2::numeric)",
    )
    .bind(&user_id)
    .bind(now)
    .bind(SEND_WINDOW_LIMIT)
    .bind(LIVE_CHALLENGE_LIMIT)
    .bind(SEND_COOLDOWN_SECONDS)
    .bind(SEND_WINDOW_HOURS)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    if suppressed {
        return Ok(None);
    }

    // Preserve quota evidence while it is relevant; reclaim only terminal rows
    // older than the send window, and only after admission permits the request.
    sqlx::query(
        "DELETE FROM password_recovery_challenges
         WHERE user_id=$1::uuid
           AND extract(epoch FROM created_at)*1000000 < $2::numeric-($3::numeric*3600000000)
           AND (extract(epoch FROM expires_at)*1000000 <= $2::numeric
                OR consumed_at IS NOT NULL OR invalidated_at IS NOT NULL)",
    )
    .bind(&user_id)
    .bind(now)
    .bind(SEND_WINDOW_HOURS)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;

    let digest = token.digest();
    let id: String = sqlx::query_scalar(
        "WITH issued AS MATERIALIZED (SELECT clock_timestamp() AS created_at)
         INSERT INTO password_recovery_challenges
             (user_id,email,credential_revision,token_hash,created_at,expires_at)
         SELECT $1::uuid,$2,$3,$4,issued.created_at,
                issued.created_at+($5*interval '1 minute')
         FROM issued RETURNING id::text",
    )
    .bind(&user_id)
    .bind(&email_key)
    .bind(revision)
    .bind(digest.as_slice())
    .bind(RECOVERY_LIFETIME_MINUTES)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    tx.commit().await.map_err(|_| StoreError::Unavailable)?;
    Ok(Some(IssuedChallenge {
        email: recipient,
        token: token.expose_secret().to_owned(),
        idempotency_key: format!("password-recovery-{id}"),
    }))
}

/// Atomically replace the credential, revoke every session, invalidate siblings,
/// and persist a notification containing only the current mailbox and opaque key.
pub async fn confirm(pool: &PgPool, token: &str, new_hash: &str) -> Result<(), StoreError> {
    let digest = token_digest(token).ok_or(StoreError::InvalidChallenge)?;
    let user_id: Option<String> = sqlx::query_scalar(
        "SELECT user_id::text FROM password_recovery_challenges WHERE token_hash=$1",
    )
    .bind(digest.as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    let user_id = user_id.ok_or(StoreError::InvalidChallenge)?;

    let mut tx = pool.begin().await.map_err(|_| StoreError::Unavailable)?;
    read_committed(&mut tx).await?;
    let account = sqlx::query(
        "SELECT u.email_key,u.email_address,c.credential_revision
         FROM password_credentials c JOIN users u ON u.id=c.user_id
         WHERE c.user_id=$1::uuid FOR UPDATE OF c,u",
    )
    .bind(&user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?
    .ok_or(StoreError::InvalidChallenge)?;
    let email_key: String = account
        .try_get("email_key")
        .map_err(|_| StoreError::Unavailable)?;
    let recipient: String = account
        .try_get("email_address")
        .map_err(|_| StoreError::Unavailable)?;
    let revision: i64 = account
        .try_get("credential_revision")
        .map_err(|_| StoreError::Unavailable)?;

    lock_challenges(&mut tx, &user_id).await?;
    // Global logout and login take the credential lock before stable UUID-order
    // session locks. Preserve preexisting revocation instants and history.
    let _ =
        sqlx::query("SELECT id FROM user_sessions WHERE user_id=$1::uuid ORDER BY id FOR UPDATE")
            .bind(&user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_| StoreError::Unavailable)?;
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp()) * 1000000)::bigint")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| StoreError::Unavailable)?;
    let challenge = sqlx::query(
        "SELECT id::text AS id,email,credential_revision,
                extract(epoch FROM created_at)*1000000 <= $3::numeric AS created,
                extract(epoch FROM expires_at)*1000000 > $3::numeric AS unexpired,
                consumed_at IS NULL AS unused,invalidated_at IS NULL AS valid
         FROM password_recovery_challenges
         WHERE user_id=$1::uuid AND token_hash=$2",
    )
    .bind(&user_id)
    .bind(digest.as_slice())
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?
    .ok_or(StoreError::InvalidChallenge)?;
    let challenge_id: String = challenge
        .try_get("id")
        .map_err(|_| StoreError::Unavailable)?;
    let bound_email: String = challenge
        .try_get("email")
        .map_err(|_| StoreError::Unavailable)?;
    let bound_revision: i64 = challenge
        .try_get("credential_revision")
        .map_err(|_| StoreError::Unavailable)?;
    let created: bool = challenge
        .try_get("created")
        .map_err(|_| StoreError::Unavailable)?;
    let unexpired: bool = challenge
        .try_get("unexpired")
        .map_err(|_| StoreError::Unavailable)?;
    let unused: bool = challenge
        .try_get("unused")
        .map_err(|_| StoreError::Unavailable)?;
    let valid: bool = challenge
        .try_get("valid")
        .map_err(|_| StoreError::Unavailable)?;
    if bound_email != email_key
        || bound_revision != revision
        || !created
        || !unexpired
        || !unused
        || !valid
    {
        return Err(StoreError::InvalidChallenge);
    }

    let updated = sqlx::query(
        "UPDATE password_credentials SET password_hash=$2,
                credential_revision=credential_revision+1,password_changed_at=clock_timestamp()
         WHERE user_id=$1::uuid AND credential_revision=$3 AND credential_revision < 9223372036854775807",
    )
    .bind(&user_id)
    .bind(new_hash)
    .bind(revision)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Unavailable);
    }
    sqlx::query(
        "UPDATE password_recovery_challenges SET consumed_at=clock_timestamp() WHERE id=$1::uuid",
    )
    .bind(&challenge_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    sqlx::query(
        "UPDATE password_recovery_challenges SET invalidated_at=clock_timestamp()
         WHERE user_id=$1::uuid AND id<>$2::uuid AND consumed_at IS NULL AND invalidated_at IS NULL",
    )
    .bind(&user_id)
    .bind(&challenge_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE user_id=$1::uuid AND revoked_at IS NULL")
        .bind(&user_id).execute(&mut *tx).await.map_err(|_| StoreError::Unavailable)?;

    sqlx::query(
        "INSERT INTO password_reset_notifications(user_id,email,idempotency_key)
         VALUES($1::uuid,$2,$3)",
    )
    .bind(&user_id)
    .bind(&recipient)
    .bind(format!("password-reset-{challenge_id}"))
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    tx.commit().await.map_err(|_| StoreError::Unavailable)
}

/// Claim one ready notification atomically; a lease exceeds the provider deadline.
pub async fn claim_notification(pool: &PgPool) -> Result<Option<Notification>, StoreError> {
    let row = sqlx::query(
        "WITH exhausted AS (
            UPDATE password_reset_notifications
            SET failed_at=clock_timestamp(),lease_until=NULL
            WHERE delivered_at IS NULL AND failed_at IS NULL AND attempts >= $1
              AND (lease_until IS NULL OR lease_until <= clock_timestamp())
            RETURNING id
         ), candidate AS (
            SELECT id FROM password_reset_notifications
            WHERE delivered_at IS NULL AND failed_at IS NULL AND attempts < $1
              AND available_at <= clock_timestamp()
              AND (lease_until IS NULL OR lease_until <= clock_timestamp())
            ORDER BY created_at,id FOR UPDATE SKIP LOCKED LIMIT 1
         )
         UPDATE password_reset_notifications n
         SET attempts=n.attempts+1,lease_until=clock_timestamp()+($2*interval '1 second')
         FROM candidate WHERE n.id=candidate.id
         RETURNING n.id::text AS id,n.email,n.idempotency_key,n.attempts",
    )
    .bind(NOTIFICATION_ATTEMPT_LIMIT)
    .bind(NOTIFICATION_LEASE_SECONDS)
    .fetch_optional(pool)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    row.map(|row| {
        Ok(Notification {
            attempt: row
                .try_get("attempts")
                .map_err(|_| StoreError::Unavailable)?,
            id: row.try_get("id").map_err(|_| StoreError::Unavailable)?,
            email: row.try_get("email").map_err(|_| StoreError::Unavailable)?,
            idempotency_key: row
                .try_get("idempotency_key")
                .map_err(|_| StoreError::Unavailable)?,
        })
    })
    .transpose()
}

/// Record delivery or schedule a bounded retry; exhausted events remain stored.
pub async fn finish_notification(
    pool: &PgPool,
    id: &str,
    attempt: i16,
    delivered: bool,
) -> Result<(), StoreError> {
    let result = if delivered {
        sqlx::query(
            "UPDATE password_reset_notifications SET delivered_at=clock_timestamp(),lease_until=NULL
             WHERE id=$1::uuid AND delivered_at IS NULL AND failed_at IS NULL AND attempts=$2 AND lease_until > clock_timestamp()",
        )
        .bind(id)
        .bind(attempt)
        .execute(pool)
        .await
    } else {
        sqlx::query(
            "UPDATE password_reset_notifications
             SET failed_at=CASE WHEN attempts >= $2 THEN clock_timestamp() ELSE NULL END,
                 available_at=CASE WHEN attempts >= $2 THEN available_at ELSE clock_timestamp()+($3*interval '1 second') END,
                 lease_until=NULL
             WHERE id=$1::uuid AND delivered_at IS NULL AND failed_at IS NULL AND attempts=$4 AND lease_until > clock_timestamp()",
        )
        .bind(id)
        .bind(NOTIFICATION_ATTEMPT_LIMIT)
        .bind(NOTIFICATION_RETRY_SECONDS)
        .bind(attempt)
        .execute(pool)
        .await
    }
    .map_err(|_| StoreError::Unavailable)?;
    if result.rows_affected() == 0 {
        return Err(StoreError::InvalidChallenge);
    }
    Ok(())
}

async fn read_committed(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<(), StoreError> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut **tx)
        .await
        .map_err(|_| StoreError::Unavailable)?;
    Ok(())
}

async fn lock_challenges(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), StoreError> {
    let _ = sqlx::query(
        "SELECT id FROM password_recovery_challenges
         WHERE user_id=$1::uuid ORDER BY id FOR UPDATE",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_digest_is_full_purpose_prefixed_token() {
        let token = format!("pr1_{}", "a".repeat(64));
        let expected: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        assert_eq!(token_digest(&token).unwrap(), expected);
        assert!(token_digest(&format!("ev1_{}", "a".repeat(64))).is_none());
    }
}
