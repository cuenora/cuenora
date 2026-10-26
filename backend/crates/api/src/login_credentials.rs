//! Exact-byte password verification; the async caller owns bounded admission.

use argon2::{
    Argon2, PasswordVerifier,
    password_hash::{Error, PasswordHash},
};

#[derive(Debug, PartialEq, Eq)]
pub struct VerificationUnavailable;

// Public synthetic fixture, not an account credential. Missing identities still
// perform one verification with the same cost as the supported stored profile.
const DUMMY_PHC: &str = "$argon2id$v=19$m=19456,t=2,p=1$c3ludGhldGljLXNhbHQwMQ$Z8raDknNUDP05Kevs6Eg0WgLPyMIZYQI8LWM5fx94lE";

/// Verify existing credentials without reapplying registration/password-list rules.
/// A missing account always returns false, even if its dummy hash matches.
/// Stored hashes must use the bounded profile produced by P04.02. Future profile
/// upgrades need an explicit compatibility policy, not unchecked PHC allocation.
pub fn verify_password(
    password: &str,
    stored_hash: Option<&str>,
) -> Result<bool, VerificationUnavailable> {
    if password.len() > 512 || password.chars().count() > 128 {
        return Ok(false);
    }
    let encoded = stored_hash.unwrap_or(DUMMY_PHC);
    if encoded.len() > 1024 {
        return Err(VerificationUnavailable);
    }
    let hash = PasswordHash::new(encoded).map_err(|_| VerificationUnavailable)?;
    if hash.algorithm.as_str() != "argon2id"
        || hash.version != Some(19)
        || hash.params.iter().count() != 3
        || hash.params.get_decimal("m") != Some(19456)
        || hash.params.get_decimal("t") != Some(2)
        || hash.params.get_decimal("p") != Some(1)
        || hash.hash.as_ref().map(|value| value.len()) != Some(32)
    {
        return Err(VerificationUnavailable);
    }
    let mut salt = [0u8; 64];
    if hash
        .salt
        .ok_or(VerificationUnavailable)?
        .decode_b64(&mut salt)
        .map_err(|_| VerificationUnavailable)?
        .len()
        < 16
    {
        return Err(VerificationUnavailable);
    }
    let matched = match Argon2::default().verify_password(password.as_bytes(), &hash) {
        Ok(()) => true,
        Err(Error::Password) => false,
        Err(_) => return Err(VerificationUnavailable),
    };
    Ok(stored_hash.is_some() && matched)
}
