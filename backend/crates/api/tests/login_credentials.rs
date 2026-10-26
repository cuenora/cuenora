use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version, password_hash::SaltString};
use cuenora_api::{
    login_credentials::{VerificationUnavailable, verify_password},
    registration_policy::hash_password,
};

fn encoded(password: &str) -> String {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, Some(32)).unwrap(),
    )
    .hash_password(
        password.as_bytes(),
        &SaltString::encode_b64(b"synthetic-salt01").unwrap(),
    )
    .unwrap()
    .to_string()
}

#[test]
fn verifies_existing_registration_bytes_without_normalization_or_policy_reapplication() {
    let password = "  lavender hills 水 e\u{301} \0";
    let hash = hash_password(password).unwrap();
    assert_eq!(verify_password(password, Some(&hash)), Ok(true));
    for wrong in [
        password.trim(),
        "  lavender hills 水 é \0",
        "wrong violet password",
        "",
    ] {
        assert_eq!(verify_password(wrong, Some(&hash)), Ok(false));
    }
    for legacy in ["PASSWORDPASSWORD", "short", ""] {
        assert_eq!(verify_password(legacy, Some(&encoded(legacy))), Ok(true));
    }
    let boundary = "🦀".repeat(128);
    assert_eq!(
        verify_password(&boundary, Some(&encoded(&boundary))),
        Ok(true)
    );
    assert_eq!(verify_password(&"水".repeat(129), Some(&hash)), Ok(false));
    assert_eq!(verify_password(&"x".repeat(513), Some(&hash)), Ok(false));
}

#[test]
fn missing_identity_never_authenticates_even_if_dummy_password_matches() {
    assert_eq!(
        verify_password(
            "cuenora synthetic missing-user password",
            Some(&encoded("cuenora synthetic missing-user password"))
        ),
        Ok(true)
    );
    for password in [
        "violet hills beside winter snow",
        "cuenora synthetic missing-user password",
        "",
        "水",
    ] {
        assert_eq!(verify_password(password, None), Ok(false));
    }
}

#[test]
fn rejects_unsupported_or_malformed_stored_phc_before_unbounded_work() {
    let hash = encoded("violet hills beside winter snow");
    assert_eq!(
        verify_password("violet hills beside winter snow", Some(&hash)),
        Ok(true)
    );
    for invalid in [
        "".to_owned(),
        "private database detail".to_owned(),
        hash.replace("argon2id", "argon2i"),
        hash.replace("v=19", "v=16"),
        hash.replace("m=19456", "m=4294967295"),
        hash.replace("t=2", "t=4294967295"),
        hash.replace("p=1", "p=32"),
        hash.replace("p=1", "p=4294967295"),
        hash.replace("v=19$", ""),
        hash.replace("p=1", "p=1,data=YWJj"),
        hash.replace("m=19456,t=2", "m=19456,m=19456,t=2"),
        hash.replace("m=19456", "m=8"),
        hash.replace("p=1", "p=1,keyid=c2VjcmV0"),
        format!("{hash}$private"),
        "x".repeat(1025),
        hash.rsplit_once('$').unwrap().0.to_owned(),
    ] {
        assert_eq!(
            verify_password("synthetic password", Some(&invalid)),
            Err(VerificationUnavailable)
        );
    }
    let short_salt = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, Some(32)).unwrap(),
    )
    .hash_password(
        b"synthetic password",
        &SaltString::encode_b64(b"shortsalt").unwrap(),
    )
    .unwrap()
    .to_string();
    assert_eq!(
        verify_password("synthetic password", Some(&short_salt)),
        Err(VerificationUnavailable)
    );
    assert_eq!(
        format!("{VerificationUnavailable:?}"),
        "VerificationUnavailable"
    );
}
