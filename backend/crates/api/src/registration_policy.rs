//! Credential input policy. No HTTP, database, clock or financial dependencies.

use argon2::{
    Algorithm, Argon2, Params, PasswordHasher, Version,
    password_hash::{SaltString, rand_core::OsRng},
};

/// A safe error category, never containing submitted credentials.
#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    InvalidEmail,
    InvalidPassword,
    HashUnavailable,
}

/// Original and validated transport addresses. Neither representation is logged.
pub struct Email {
    original: String,
    address: String,
}

impl Email {
    pub fn parse(input: &str) -> Result<Self, PolicyError> {
        let invalid = PolicyError::InvalidEmail;
        if input.len() > 1024 || input.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(invalid);
        }
        let (local, domain) = input.split_once('@').ok_or(PolicyError::InvalidEmail)?;
        // Deliberately accept mailbox dot-atoms, not display names/quoted strings.
        if local.is_empty()
            || local.len() > 64
            || !local.is_ascii()
            || local.starts_with('.')
            || local.ends_with('.')
            || local.contains("..")
            || !local
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&c))
        {
            return Err(invalid);
        }
        let domain = idna::domain_to_ascii_strict(domain).map_err(|_| PolicyError::InvalidEmail)?;
        let domain = domain.to_ascii_lowercase();
        if !domain.contains('.')
            || domain.ends_with('.')
            || domain.split('.').any(|label| {
                label.is_empty()
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
        {
            return Err(invalid);
        }
        let address = format!("{local}@{domain}");
        if address.len() > 254 {
            return Err(invalid);
        }
        Ok(Self {
            original: input.to_owned(),
            address,
        })
    }

    pub fn original(&self) -> &str {
        &self.original
    }

    pub fn address(&self) -> &str {
        &self.address
    }
}

pub fn validate_password(password: &str) -> Result<(), PolicyError> {
    let length = password.chars().count();
    if !(15..=128).contains(&length)
        || passwords::analyzer::is_common_password(password.to_lowercase())
    {
        return Err(PolicyError::InvalidPassword);
    }
    Ok(())
}

/// Hash exact UTF-8 bytes; never trim, truncate or normalize the password.
/// The caller must bound admission and run CPU work off its async executor.
pub fn hash_password(password: &str) -> Result<String, PolicyError> {
    validate_password(password)?;
    let mut salt_bytes = [0u8; 16];
    use argon2::password_hash::rand_core::RngCore;
    OsRng
        .try_fill_bytes(&mut salt_bytes)
        .map_err(|_| PolicyError::HashUnavailable)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| PolicyError::HashUnavailable)?;
    let params =
        Params::new(19 * 1024, 2, 1, Some(32)).map_err(|_| PolicyError::HashUnavailable)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PolicyError::HashUnavailable)
}
