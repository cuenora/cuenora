use cuenora_api::registration_policy::{Email, hash_password, validate_password};

#[test]
fn email_policy_preserves_original_and_canonicalizes_only_the_domain() {
    let email = Email::parse("AnA+tag@mañana.Example").unwrap();
    assert_eq!(email.original(), "AnA+tag@mañana.Example");
    assert_eq!(email.address(), "AnA+tag@xn--maana-pta.example");
    for invalid in [
        "",
        " a@example.test",
        "a@example.test ",
        "a@@example.test",
        "josé@example.test",
        ".a@example.test",
        "a..b@example.test",
        "a@-bad.test",
        "a@localhost",
        "a@example.test\n",
        "a@bad_domain.test",
    ] {
        assert!(Email::parse(invalid).is_err(), "{invalid:?}");
    }
    assert!(Email::parse(&format!("{}@example.test", "a".repeat(65))).is_err());
    assert!(Email::parse("a.na@example.test").is_ok());
    assert!(Email::parse("ana+tag@example.test").is_ok());
    let domain = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(60)
    );
    assert!(Email::parse(&format!("a@{domain}")).is_ok());
    assert!(Email::parse(&format!("aa@{domain}")).is_err());
    assert!(Email::parse(&format!("a@{}.test", "a".repeat(64))).is_err());
    assert!(Email::parse(&format!("{}@example.test", "a".repeat(64))).is_ok());
    assert!(matches!(
        Email::parse("secret\0@example.test"),
        Err(cuenora_api::registration_policy::PolicyError::InvalidEmail)
    ));
}

#[test]
fn password_policy_counts_unicode_scalars_and_preserves_spaces() {
    for password in [
        "夏".repeat(15),
        "夏".repeat(128),
        "🦀".repeat(15),
        "🦀".repeat(128),
        "a".repeat(128),
        "moss violet river stones".to_owned(),
        " \t夏 violet moss \0".to_owned(),
    ] {
        assert!(validate_password(&password).is_ok());
    }
    for password in [
        "夏".repeat(14),
        "夏".repeat(129),
        "🦀".repeat(14),
        "🦀".repeat(129),
        "a".repeat(129),
        String::new(),
        "passwordpassword".to_owned(),
        "pAsSwOrDpAsSwOrD".to_owned(),
        "123456789012345".to_owned(),
        "12345678901234567890".to_owned(),
    ] {
        assert!(validate_password(&password).is_err());
    }
}

#[test]
fn hashing_rejects_invalid_inputs() {
    assert!(hash_password("short").is_err());
    assert_eq!(
        format!("{:?}", validate_password("sensitive").unwrap_err()),
        "InvalidPassword"
    );
}

#[test]
fn salted_argon2id_preserves_the_exact_password_and_records_parameters() {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let password = "  moss 夏 violet river stones\0";
    let first = hash_password(password).unwrap();
    let second = hash_password(password).unwrap();
    assert_ne!(first, second);
    assert!(first.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    let parsed = PasswordHash::new(&first).unwrap();
    assert_eq!(parsed.salt.unwrap().len(), 22);
    assert_eq!(parsed.hash.unwrap().len(), 32);
    assert!(
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    );
    assert!(
        Argon2::default()
            .verify_password(password.trim().as_bytes(), &parsed)
            .is_err()
    );
    assert!(
        Argon2::default()
            .verify_password(b"wrong password", &parsed)
            .is_err()
    );
    for altered in [password.replace('\0', ""), password.replace('夏', "x")] {
        assert!(
            Argon2::default()
                .verify_password(altered.as_bytes(), &parsed)
                .is_err()
        );
    }
}
