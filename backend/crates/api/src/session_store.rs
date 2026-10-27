//! Transactional refresh rotation and possession-scoped session revocation.

use crate::session_tokens::{
    ACCESS_TOKEN_LIFETIME_SECONDS, InitialTokens, REFRESH_TOKEN_LIFETIME_SECONDS, generate,
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};

#[derive(Debug, PartialEq, Eq)]
pub struct SessionStoreUnavailable;

impl From<sqlx::Error> for SessionStoreUnavailable {
    fn from(_: sqlx::Error) -> Self {
        Self
    }
}

fn refresh_digest(refresh: &str) -> Option<[u8; 32]> {
    let secret = refresh.strip_prefix("cr1_")?;
    if secret.len() != 64
        || !secret
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    Some(Sha256::digest(refresh.as_bytes()).into())
}

// Both operations lock the family, not just the presented generation. Tokens
// are retained, so a fresh READ COMMITTED statement after this lock sees any
// predecessor consumption, successor or revocation committed by a prior waiter.
async fn lock_family(
    tx: &mut Transaction<'_, Postgres>,
    digest: &[u8; 32],
) -> Result<Option<String>, SessionStoreUnavailable> {
    Ok(sqlx::query_scalar(
        "/* refresh_family_lock */ SELECT s.id::text
         FROM session_refresh_tokens r
         JOIN user_sessions s ON s.id=r.session_id AND s.user_id=r.user_id
         WHERE r.token_hash=$1 FOR UPDATE OF s",
    )
    .bind(digest.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

async fn revoke_locked(
    tx: &mut Transaction<'_, Postgres>,
    session: &str,
) -> Result<(), SessionStoreUnavailable> {
    sqlx::query(
        "UPDATE user_sessions SET revoked_at=clock_timestamp()
         WHERE id=$1::uuid AND revoked_at IS NULL",
    )
    .bind(session)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Consume a refresh credential once and issue a new 15-minute/30-day pair.
/// None is a generic authentication rejection. Reusing any consumed generation,
/// even after its expiry, commits owning-family revocation before returning None.
/// Malformed, unknown, expired-unconsumed and already-revoked tokens write nothing.
/// Lost/cancelled responses around commit do not imply that rotation rolled back.
pub async fn rotate(
    pool: &PgPool,
    refresh: &str,
) -> Result<Option<InitialTokens>, SessionStoreUnavailable> {
    let Some(digest) = refresh_digest(refresh) else {
        return Ok(None);
    };
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    let Some(session) = lock_family(&mut tx, &digest).await? else {
        return Ok(None);
    };
    // Do not reuse the lock query's snapshot or its start time for eligibility.
    let row = sqlx::query(
        "SELECT r.id::text AS id, r.user_id::text AS user_id,
                r.used_at IS NOT NULL AS consumed,
                r.expires_at > clock_timestamp() AS unexpired,
                s.revoked_at IS NOT NULL AS revoked
         FROM session_refresh_tokens r JOIN user_sessions s ON s.id=r.session_id
         WHERE r.token_hash=$1 AND s.id=$2::uuid",
    )
    .bind(digest.as_slice())
    .bind(&session)
    .fetch_one(&mut *tx)
    .await?;
    if row.try_get::<bool, _>("revoked")? {
        return Ok(None);
    }
    if row.try_get::<bool, _>("consumed")? {
        revoke_locked(&mut tx, &session).await?;
        tx.commit().await?;
        return Ok(None);
    }
    if !row.try_get::<bool, _>("unexpired")? {
        return Ok(None);
    }
    let tokens = generate().map_err(|_| SessionStoreUnavailable)?;
    // Insert the successor first: the predecessor link has an immediate FK.
    // Its single database instant also timestamps the access row and consumption.
    let successor: String = sqlx::query_scalar(
        "INSERT INTO session_refresh_tokens
             (session_id, user_id, token_hash, created_at, expires_at)
         SELECT $1::uuid, $2::uuid, $3, instant, instant+($4*interval '1 second')
         FROM (SELECT clock_timestamp() AS instant) clock RETURNING id::text",
    )
    .bind(&session)
    .bind(row.try_get::<String, _>("user_id")?)
    .bind(tokens.refresh.digest().as_slice())
    .bind(REFRESH_TOKEN_LIFETIME_SECONDS as i64)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO session_access_tokens
             (session_id, user_id, token_hash, created_at, expires_at)
         SELECT session_id, user_id, $2, created_at, created_at+($3*interval '1 second')
         FROM session_refresh_tokens WHERE id=$1::uuid",
    )
    .bind(&successor)
    .bind(tokens.access.digest().as_slice())
    .bind(ACCESS_TOKEN_LIFETIME_SECONDS as i64)
    .execute(&mut *tx)
    .await?;
    let updated = sqlx::query(
        "UPDATE session_refresh_tokens r SET used_at=n.created_at, replaced_by=n.id
         FROM session_refresh_tokens n
         WHERE r.id=$1::uuid AND n.id=$2::uuid AND r.used_at IS NULL",
    )
    .bind(row.try_get::<String, _>("id")?)
    .bind(successor)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(SessionStoreUnavailable);
    }
    tx.commit().await?;
    Ok(Some(tokens))
}

/// Revoke only the family identified by possession of a retained refresh secret.
/// Accepts consumed/expired generations too, and is idempotent for revoked ones.
/// False means malformed or unknown; no caller-supplied owner IDs are accepted.
/// This is a persistence boundary, not an HTTP logout/global-revocation endpoint.
pub async fn revoke(pool: &PgPool, refresh: &str) -> Result<bool, SessionStoreUnavailable> {
    let Some(digest) = refresh_digest(refresh) else {
        return Ok(false);
    };
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    let Some(session) = lock_family(&mut tx, &digest).await? else {
        return Ok(false);
    };
    revoke_locked(&mut tx, &session).await?;
    tx.commit().await?;
    Ok(true)
}
