//! PostgreSQL registration adapter; no HTTP or environment configuration.

use crate::registration_policy::Email;
use sqlx::PgPool;

/// Does not contain database errors, addresses or password material.
#[derive(Debug, PartialEq, Eq)]
pub struct RegistrationUnavailable;

/// Persist a pending user and a server-generated encoded hash atomically.
/// Existing identities are unchanged and return the same success as new ones.
/// Callers supply a bounded pool and a hash produced by `hash_password`.
pub async fn persist_registration(
    pool: &PgPool,
    email: &Email,
    password_hash: &str,
) -> Result<(), RegistrationUnavailable> {
    sqlx::query(
        "WITH pending_user AS (
            INSERT INTO users (email_original, email_address) VALUES ($1, $2)
            ON CONFLICT (email_key) DO NOTHING RETURNING id
        )
        INSERT INTO password_credentials (user_id, password_hash)
        SELECT id, $3 FROM pending_user",
    )
    .bind(email.original())
    .bind(email.address())
    .bind(password_hash)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|_| RegistrationUnavailable)
}
