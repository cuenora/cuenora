//! Server-owned credential snapshots and atomic initial-session persistence.

use crate::{
    registration_policy::Email,
    session_tokens::{
        ACCESS_TOKEN_LIFETIME_SECONDS, InitialTokens, REFRESH_TOKEN_LIFETIME_SECONDS,
    },
};
use sqlx::{PgPool, Row};

#[derive(Debug, PartialEq, Eq)]
pub struct LoginStoreUnavailable;

/// Server-owned, not yet password-verified. Never format or serialize a snapshot.
///
/// ```compile_fail
/// let value: Option<cuenora_api::login_store::CredentialSnapshot> = None;
/// println!("{:?}", value);
/// ```
/// ```compile_fail
/// let value: Option<cuenora_api::login_store::CredentialSnapshot> = None;
/// let _ = serde_json::to_string(&value);
/// ```
/// ```compile_fail
/// fn format_snapshot(value: cuenora_api::login_store::CredentialSnapshot) {
///     println!("{}", value);
/// }
/// ```
/// ```compile_fail
/// let forged = cuenora_api::login_store::CredentialSnapshot {
///     user_id: "another-user".into(),
///     encoded_hash: "forged".into(),
///     password_changed_at_micros: 0,
/// };
/// ```
pub struct CredentialSnapshot {
    user_id: String,
    encoded_hash: String,
    password_changed_at_micros: i64,
}

impl CredentialSnapshot {
    pub fn encoded_hash(&self) -> &str {
        &self.encoded_hash
    }
}

pub async fn lookup_credentials(
    pool: &PgPool,
    email: &Email,
) -> Result<Option<CredentialSnapshot>, LoginStoreUnavailable> {
    let row = sqlx::query(
        "SELECT u.id::text AS user_id, c.password_hash,
                (extract(epoch FROM c.password_changed_at) * 1000000)::bigint AS revision
         FROM users u
         JOIN password_credentials c ON c.user_id = u.id
         WHERE u.email_key = lower($1 COLLATE \"C\")",
    )
    .bind(email.address())
    .fetch_optional(pool)
    .await
    .map_err(|_| LoginStoreUnavailable)?;
    row.map(|row| {
        Ok(CredentialSnapshot {
            user_id: row.try_get("user_id").map_err(|_| LoginStoreUnavailable)?,
            encoded_hash: row
                .try_get("password_hash")
                .map_err(|_| LoginStoreUnavailable)?,
            password_changed_at_micros: row
                .try_get("revision")
                .map_err(|_| LoginStoreUnavailable)?,
        })
    })
    .transpose()
}

/// Persist a complete family only while the server snapshot remains current.
/// The caller MUST first verify the supplied password against `encoded_hash()`;
/// lookup alone does not authenticate anyone. False means a stale snapshot, and
/// errors contain no connection, credential or token material.
pub async fn issue_initial_session(
    pool: &PgPool,
    credential: &CredentialSnapshot,
    tokens: &InitialTokens,
) -> Result<bool, LoginStoreUnavailable> {
    // The purpose prefixes are part of the public token scheme, but checking them
    // here also prevents callers from accidentally persisting swapped digests.
    if !tokens.access.expose_secret().starts_with("ca1_")
        || !tokens.refresh.expose_secret().starts_with("cr1_")
    {
        return Err(LoginStoreUnavailable);
    }

    let inserted = sqlx::query_scalar::<_, i64>(
        "WITH current_credential AS MATERIALIZED (
            SELECT u.id, c.password_hash, c.password_changed_at
            FROM users u
            JOIN password_credentials c ON c.user_id = u.id
            WHERE u.id = $1::uuid
              AND c.password_hash = $2
              AND (extract(epoch FROM c.password_changed_at) * 1000000)::bigint = $3
            FOR UPDATE OF c
         ), new_session AS (
            INSERT INTO user_sessions (user_id)
            SELECT id FROM current_credential
            RETURNING id, user_id, created_at
         ), new_access AS (
            INSERT INTO session_access_tokens (session_id, user_id, token_hash, created_at, expires_at)
            SELECT id, user_id, $4, created_at, created_at + ($6 * interval '1 second')
            FROM new_session
            RETURNING session_id, user_id
         ), new_refresh AS (
            INSERT INTO session_refresh_tokens (session_id, user_id, token_hash, created_at, expires_at)
            SELECT s.id, s.user_id, $5, s.created_at, s.created_at + ($7 * interval '1 second')
            FROM new_access a
            JOIN new_session s ON s.id = a.session_id AND s.user_id = a.user_id
            RETURNING id
         )
         SELECT count(*)::bigint FROM new_refresh",
    )
    .bind(&credential.user_id)
    .bind(&credential.encoded_hash)
    .bind(credential.password_changed_at_micros)
    .bind(tokens.access.digest().to_vec())
    .bind(tokens.refresh.digest().to_vec())
    .bind(ACCESS_TOKEN_LIFETIME_SECONDS as i64)
    .bind(REFRESH_TOKEN_LIFETIME_SECONDS as i64)
    .fetch_one(pool)
    .await
    .map_err(|_| LoginStoreUnavailable)?;
    Ok(inserted == 1)
}

/// Look up credentials only for a server-authenticated identity, never a body ID.
/// Password verification is still required before global revocation.
pub async fn lookup_identity_credentials(
    pool: &PgPool,
    identity: &crate::auth_identity::AuthenticatedIdentity,
) -> Result<Option<CredentialSnapshot>, LoginStoreUnavailable> {
    let row = sqlx::query(
        "SELECT password_hash,
                (extract(epoch FROM password_changed_at) * 1000000)::bigint AS revision
         FROM password_credentials WHERE user_id=$1::uuid",
    )
    .bind(identity.user_id())
    .fetch_optional(pool)
    .await
    .map_err(|_| LoginStoreUnavailable)?;
    row.map(|row| {
        Ok(CredentialSnapshot {
            user_id: identity.user_id().to_owned(),
            encoded_hash: row
                .try_get("password_hash")
                .map_err(|_| LoginStoreUnavailable)?,
            password_changed_at_micros: row
                .try_get("revision")
                .map_err(|_| LoginStoreUnavailable)?,
        })
    })
    .transpose()
}

/// Caller MUST verify the snapshot's password first. Revalidate it under the same
/// credential lock as login, then use fresh snapshots to cover prior committed
/// logins. Later logins wait for commit and may open new sessions. No credentials
/// or token history are deleted. A revoked caller family cannot authorize this.
pub async fn revoke_all_sessions(
    pool: &PgPool,
    credential: &CredentialSnapshot,
    identity: &crate::auth_identity::AuthenticatedIdentity,
) -> Result<bool, LoginStoreUnavailable> {
    if credential.user_id != identity.user_id() {
        return Ok(false);
    }
    let mut tx = pool.begin().await.map_err(|_| LoginStoreUnavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await
        .map_err(|_| LoginStoreUnavailable)?;
    let current: Option<String> = sqlx::query_scalar(
        "/* global_credential_lock */ SELECT user_id::text FROM password_credentials
         WHERE user_id=$1::uuid AND password_hash=$2
           AND (extract(epoch FROM password_changed_at)*1000000)::bigint=$3
         FOR UPDATE",
    )
    .bind(&credential.user_id)
    .bind(&credential.encoded_hash)
    .bind(credential.password_changed_at_micros)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| LoginStoreUnavailable)?;
    if current.is_none() {
        return Ok(false);
    }
    // Ordered family locks serialize rotation/replay/logout with this operation.
    // Do not take any family lock before the credential lock (login's order).
    sqlx::query("SELECT id FROM user_sessions WHERE user_id=$1::uuid ORDER BY id FOR UPDATE")
        .bind(&credential.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| LoginStoreUnavailable)?;
    let caller_active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_sessions WHERE id=$1::uuid AND user_id=$2::uuid AND revoked_at IS NULL)",
    ).bind(identity.session_id()).bind(&credential.user_id).fetch_one(&mut *tx).await.map_err(|_| LoginStoreUnavailable)?;
    if !caller_active {
        return Ok(false);
    }
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE user_id=$1::uuid AND revoked_at IS NULL")
        .bind(&credential.user_id).execute(&mut *tx).await.map_err(|_| LoginStoreUnavailable)?;
    tx.commit().await.map_err(|_| LoginStoreUnavailable)?;
    Ok(true)
}
