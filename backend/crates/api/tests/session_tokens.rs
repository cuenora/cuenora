use cuenora_api::session_tokens::{
    ACCESS_TOKEN_LIFETIME_SECONDS, REFRESH_TOKEN_LIFETIME_SECONDS, TokenUnavailable, generate,
};
use sha2::{Digest, Sha256};

#[test]
fn emits_distinct_purpose_bound_random_credentials_and_matching_digests() {
    let first = generate().unwrap();
    let second = generate().unwrap();
    for (token, prefix) in [
        (&first.access, "ca1_"),
        (&first.refresh, "cr1_"),
        (&second.access, "ca1_"),
        (&second.refresh, "cr1_"),
    ] {
        let raw = token.expose_secret();
        assert_eq!(raw.len(), 68);
        assert!(raw.starts_with(prefix));
        assert!(
            raw[4..]
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        );
        let expected: [u8; 32] = Sha256::digest(raw.as_bytes()).into();
        assert_eq!(token.digest(), expected);
        assert_eq!(token.digest().len(), 32);
    }
    assert_ne!(first.access.expose_secret(), second.access.expose_secret());
    assert_ne!(
        first.refresh.expose_secret(),
        second.refresh.expose_secret()
    );
    assert_ne!(first.access.digest(), first.refresh.digest());
    assert_eq!(ACCESS_TOKEN_LIFETIME_SECONDS, 15 * 60);
    assert_eq!(REFRESH_TOKEN_LIFETIME_SECONDS, 30 * 24 * 60 * 60);
    assert_eq!(format!("{TokenUnavailable:?}"), "TokenUnavailable");
}
