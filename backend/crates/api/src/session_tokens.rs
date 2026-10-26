//! Initial opaque credentials, without HTTP, database or clock dependencies.

use argon2::password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

pub const ACCESS_TOKEN_LIFETIME_SECONDS: u64 = 900;
pub const REFRESH_TOKEN_LIFETIME_SECONDS: u64 = 2_592_000;

#[derive(Debug, PartialEq, Eq)]
pub struct TokenUnavailable;

/// Raw credentials deliberately implement neither Debug, Display nor Serialize.
/// Expose the value only when building the successful authentication response.
///
/// ```compile_fail
/// let tokens = cuenora_api::session_tokens::generate().unwrap();
/// println!("{:?}", tokens.access);
/// ```
/// ```compile_fail
/// let tokens = cuenora_api::session_tokens::generate().unwrap();
/// println!("{}", tokens.refresh);
/// ```
/// ```compile_fail
/// let tokens = cuenora_api::session_tokens::generate().unwrap();
/// let _ = serde_json::to_string(&tokens.access);
/// ```
pub struct SecretToken(String);

impl SecretToken {
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Store this digest of the entire emitted token, including purpose/version.
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.0.as_bytes()).into()
    }
}

/// A pair is equally private; embedding secrets must not make them loggable.
///
/// ```compile_fail
/// let tokens = cuenora_api::session_tokens::generate().unwrap();
/// println!("{:?}", tokens);
/// ```
/// ```compile_fail
/// let tokens = cuenora_api::session_tokens::generate().unwrap();
/// println!("{}", tokens);
/// ```
/// ```compile_fail
/// let tokens = cuenora_api::session_tokens::generate().unwrap();
/// let _ = serde_json::to_string(&tokens);
/// ```
pub struct InitialTokens {
    pub access: SecretToken,
    pub refresh: SecretToken,
}

/// Generate two independent 256-bit secrets. Any OS entropy failure fails closed.
pub fn generate() -> Result<InitialTokens, TokenUnavailable> {
    generate_with(|bytes| OsRng.try_fill_bytes(bytes).map_err(|_| TokenUnavailable))
}

fn generate_with(
    fill: impl FnOnce(&mut [u8]) -> Result<(), TokenUnavailable>,
) -> Result<InitialTokens, TokenUnavailable> {
    let mut entropy = [0u8; 64];
    fill(&mut entropy)?;
    Ok(InitialTokens {
        access: encode("ca1_", &entropy[..32]),
        refresh: encode("cr1_", &entropy[32..]),
    })
}

fn encode(prefix: &str, bytes: &[u8]) -> SecretToken {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(68);
    value.push_str(prefix);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 15) as usize] as char);
    }
    SecretToken(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_all_entropy_bytes_and_returns_no_credentials_on_entropy_failure() {
        let tokens = generate_with(|bytes| {
            assert_eq!(bytes.len(), 64);
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = i as u8;
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(
            tokens.access.expose_secret(),
            "ca1_000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        );
        assert_eq!(
            tokens.refresh.expose_secret(),
            "cr1_202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f"
        );
        assert_eq!(
            generate_with(|bytes| {
                bytes[..32].fill(42); // Even a partial fill must not produce a token pair.
                Err(TokenUnavailable)
            })
            .err(),
            Some(TokenUnavailable)
        );
    }
}
