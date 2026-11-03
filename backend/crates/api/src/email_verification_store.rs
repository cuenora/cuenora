//! Transactional storage for single-use, session-bound email verification.

use crate::auth_identity::AuthenticatedIdentity;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

const CHALLENGE_LIFETIME_HOURS: i64 = 24;
const SEND_COOLDOWN_SECONDS: i64 = 60;
const SEND_WINDOW_HOURS: i64 = 1;
const SEND_WINDOW_LIMIT: i64 = 5;
const LIVE_CHALLENGE_LIMIT: i64 = 5;
const MICROSECONDS_PER_SECOND: i64 = 1_000_000;
const SECONDS_PER_HOUR: i64 = 3_600;

/// An issuance result contains the one-time secret and must never be logged or
/// serialized implicitly. The caller owns delivery and may expose it only there.
pub struct IssuedChallenge {
    pub email: String,
    pub token: String,
    pub idempotency_key: String,
}

/// Safe storage outcomes; neither variants nor formatting contain input data.
#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    Unavailable,
    InvalidChallenge,
    InvalidSession,
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
    Ok(encode_token(&bytes))
}

fn encode_token(bytes: &[u8; 32]) -> SecretToken {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(68);
    value.push_str("ev1_");
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 15) as usize] as char);
    }
    SecretToken(value)
}

fn token_digest(token: &str) -> Option<[u8; 32]> {
    let secret = token.strip_prefix("ev1_")?;
    if secret.len() != 64
        || !secret
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    Some(Sha256::digest(token.as_bytes()).into())
}

/// Create a new challenge if the account and resend policy permit it.
/// Suppressed requests are read-only and return `Ok(None)`.
pub async fn issue(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
) -> Result<Option<IssuedChallenge>, StoreError> {
    let token = generate_token()?;
    let mut tx = pool.begin().await.map_err(|_| StoreError::Unavailable)?;
    set_read_committed(&mut tx).await?;

    let credential = lock_credential(&mut tx, identity.user_id()).await?;
    let Some(credential) = credential else {
        return Err(StoreError::InvalidSession);
    };
    lock_challenges(&mut tx, identity.user_id()).await?;
    lock_authorizing_family(&mut tx, identity).await?;

    if credential.verified {
        return Ok(None);
    }

    let now = database_now(&mut tx).await?;
    let blocked: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM email_verification_challenges
            WHERE user_id=$1::uuid
              AND extract(epoch FROM created_at) * 1000000 > $2::numeric - ($5::numeric * $6::numeric)
         ) OR (
            SELECT count(*) >= $3 FROM email_verification_challenges
            WHERE user_id=$1::uuid
              AND extract(epoch FROM created_at) * 1000000 >= $2::numeric - ($7::numeric * $8::numeric)
         ) OR (
            SELECT count(*) >= $4 FROM email_verification_challenges
            WHERE user_id=$1::uuid AND consumed_at IS NULL AND invalidated_at IS NULL
              AND extract(epoch FROM expires_at) * 1000000 > $2::numeric
         )",
    )
    .bind(identity.user_id())
    .bind(now)
    .bind(SEND_WINDOW_LIMIT)
    .bind(LIVE_CHALLENGE_LIMIT)
    .bind(SEND_COOLDOWN_SECONDS)
    .bind(MICROSECONDS_PER_SECOND)
    .bind(SEND_WINDOW_HOURS)
    .bind(SECONDS_PER_HOUR * MICROSECONDS_PER_SECOND)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    if blocked {
        return Ok(None);
    }

    // Keep recent terminal challenges until their hourly send quota expires.
    // Expired or terminal older rows are reclaimable without touching account or
    // session history, and no cleanup occurs on a suppressed request.
    sqlx::query(
        "DELETE FROM email_verification_challenges
         WHERE user_id=$1::uuid
           AND extract(epoch FROM created_at) * 1000000 < $2::numeric - ($3::numeric * $4::numeric)
           AND (extract(epoch FROM expires_at) * 1000000 <= $2::numeric OR consumed_at IS NOT NULL OR invalidated_at IS NOT NULL)",
    )
    .bind(identity.user_id())
    .bind(now)
    .bind(SEND_WINDOW_HOURS)
    .bind(SECONDS_PER_HOUR * MICROSECONDS_PER_SECOND)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;

    let digest = token.digest();
    let id: String = sqlx::query_scalar(
        "WITH issued_at AS MATERIALIZED (SELECT clock_timestamp() AS created_at)
         INSERT INTO email_verification_challenges
             (user_id, email, credential_revision, token_hash, created_at, expires_at)
         SELECT $1::uuid, $2, $3, $4, issued_at.created_at,
                issued_at.created_at + ($5 * interval '1 hour')
         FROM issued_at
         RETURNING id::text",
    )
    .bind(identity.user_id())
    .bind(&credential.email)
    .bind(credential.revision)
    .bind(digest.as_slice())
    .bind(CHALLENGE_LIFETIME_HOURS)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    tx.commit().await.map_err(|_| StoreError::Unavailable)?;

    Ok(Some(IssuedChallenge {
        email: credential.recipient,
        token: token.expose_secret().to_owned(),
        idempotency_key: format!("email-verification-{id}"),
    }))
}

/// Atomically consume a matching challenge and verify its bound account.
pub async fn confirm(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    token: &str,
) -> Result<(), StoreError> {
    let Some(digest) = token_digest(token) else {
        return Err(StoreError::InvalidChallenge);
    };
    let mut tx = pool.begin().await.map_err(|_| StoreError::Unavailable)?;
    set_read_committed(&mut tx).await?;

    let credential = lock_credential(&mut tx, identity.user_id()).await?;
    let Some(credential) = credential else {
        return Err(StoreError::InvalidSession);
    };
    lock_challenges(&mut tx, identity.user_id()).await?;
    lock_authorizing_family(&mut tx, identity).await?;
    let now = database_now(&mut tx).await?;

    let challenge = sqlx::query(
        "SELECT id::text AS id, email, credential_revision,
                extract(epoch FROM created_at) * 1000000 <= $3::numeric AS created,
                extract(epoch FROM expires_at) * 1000000 > $3::numeric AS unexpired,
                consumed_at IS NULL AS unused,
                invalidated_at IS NULL AS not_invalidated
         FROM email_verification_challenges
         WHERE user_id=$1::uuid AND token_hash=$2",
    )
    .bind(identity.user_id())
    .bind(digest.as_slice())
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    let Some(challenge) = challenge else {
        return Err(StoreError::InvalidChallenge);
    };
    let challenge_id: String = challenge
        .try_get("id")
        .map_err(|_| StoreError::Unavailable)?;
    let email: String = challenge
        .try_get("email")
        .map_err(|_| StoreError::Unavailable)?;
    let revision: i64 = challenge
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
    let not_invalidated: bool = challenge
        .try_get("not_invalidated")
        .map_err(|_| StoreError::Unavailable)?;

    if email != credential.email
        || revision != credential.revision
        || !created
        || !unexpired
        || !unused
        || !not_invalidated
        || credential.verified
    {
        return Err(StoreError::InvalidChallenge);
    }

    sqlx::query(
        "UPDATE email_verification_challenges SET consumed_at=clock_timestamp()
         WHERE id=$1::uuid AND consumed_at IS NULL AND invalidated_at IS NULL",
    )
    .bind(&challenge_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    let verified = sqlx::query(
        "UPDATE users SET email_verified_at=clock_timestamp(), updated_at=clock_timestamp()
         WHERE id=$1::uuid AND email_key=$2 COLLATE \"C\" AND email_verified_at IS NULL",
    )
    .bind(identity.user_id())
    .bind(&credential.email)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    if verified.rows_affected() != 1 {
        return Err(StoreError::InvalidChallenge);
    }
    sqlx::query(
        "UPDATE email_verification_challenges SET invalidated_at=clock_timestamp()
         WHERE user_id=$1::uuid AND id<>$3::uuid AND consumed_at IS NULL
           AND invalidated_at IS NULL
           AND extract(epoch FROM expires_at) * 1000000 > $2::numeric",
    )
    .bind(identity.user_id())
    .bind(now)
    .bind(&challenge_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    tx.commit().await.map_err(|_| StoreError::Unavailable)
}

struct Credential {
    email: String,
    recipient: String,
    revision: i64,
    verified: bool,
}

async fn set_read_committed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), StoreError> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut **tx)
        .await
        .map_err(|_| StoreError::Unavailable)?;
    Ok(())
}

async fn lock_credential(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<Option<Credential>, StoreError> {
    let row = sqlx::query(
        "SELECT u.email_key, u.email_address, c.credential_revision, u.email_verified_at IS NOT NULL AS verified
         FROM password_credentials c JOIN users u ON u.id=c.user_id
         WHERE c.user_id=$1::uuid FOR UPDATE OF c, u",
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    row.map(|row| {
        Ok(Credential {
            recipient: row
                .try_get("email_address")
                .map_err(|_| StoreError::Unavailable)?,
            email: row
                .try_get("email_key")
                .map_err(|_| StoreError::Unavailable)?,
            revision: row
                .try_get("credential_revision")
                .map_err(|_| StoreError::Unavailable)?,
            verified: row
                .try_get("verified")
                .map_err(|_| StoreError::Unavailable)?,
        })
    })
    .transpose()
}

async fn lock_challenges(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), StoreError> {
    let _ = sqlx::query(
        "SELECT id FROM email_verification_challenges
         WHERE user_id=$1::uuid ORDER BY id FOR UPDATE",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    Ok(())
}

async fn lock_authorizing_family(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    identity: &AuthenticatedIdentity,
) -> Result<(), StoreError> {
    let session: Option<String> = sqlx::query_scalar(
        "SELECT id::text FROM user_sessions
         WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE",
    )
    .bind(identity.session_id())
    .bind(identity.user_id())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    if session.is_none() {
        return Err(StoreError::InvalidSession);
    }

    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM session_access_tokens a
            JOIN user_sessions s ON s.id=a.session_id AND s.user_id=a.user_id
            WHERE a.token_hash=$1 AND a.session_id=$2::uuid AND a.user_id=$3::uuid
              AND a.expires_at > clock_timestamp() AND s.revoked_at IS NULL
         )",
    )
    .bind(identity.access_digest().as_slice())
    .bind(identity.session_id())
    .bind(identity.user_id())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StoreError::Unavailable)?;
    if !active {
        return Err(StoreError::InvalidSession);
    }
    Ok(())
}

async fn database_now(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<i64, StoreError> {
    sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp()) * 1000000)::bigint")
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| StoreError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_tokens_use_the_exact_versioned_lowercase_hex_shape() {
        let bytes = std::array::from_fn(|index| index as u8);
        let token = encode_token(&bytes);
        assert_eq!(
            token.expose_secret(),
            "ev1_000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        );
        let digest = token_digest(token.expose_secret()).unwrap();
        let expected: [u8; 32] = Sha256::digest(token.expose_secret().as_bytes()).into();
        assert_eq!(digest, expected);
        assert!(
            token_digest("pr1_000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
                .is_none()
        );
        assert!(
            token_digest("ev1_000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1F")
                .is_none()
        );
        assert!(
            token_digest("ev1_000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0")
                .is_none()
        );
    }
}
