use cuenora_domain::{
    Account, AccountId, AccountName, AccountNameError, AccountOwnerId, AccountStatus, AccountType,
    Currency,
};

#[test]
fn names_trim_outer_whitespace_without_changing_internal_text() {
    for (input, expected) in [
        ("  Savings  ", "Savings"),
        ("\tMy  Savings\n", "My  Savings"),
        ("\u{2003}Épargne 日本\u{a0}", "Épargne 日本"),
        (" e\u{301} ", "e\u{301}"),
    ] {
        assert_eq!(AccountName::parse(input).unwrap().as_str(), expected);
    }
}

#[test]
fn empty_names_and_remaining_ascii_controls_are_rejected() {
    for input in ["", " \t\r\n", "\u{2003}\u{a0}"] {
        assert_eq!(AccountName::parse(input), Err(AccountNameError::Empty));
    }
    for control in (0_u8..=31).chain(std::iter::once(127)) {
        let name = format!("My{}Savings", char::from(control));
        assert_eq!(
            AccountName::parse(&name),
            Err(AccountNameError::ControlCharacter)
        );
    }
}

#[test]
fn name_limit_counts_characters_after_trimming_not_utf8_bytes() {
    for character in ['a', 'é', '💰'] {
        let limit = character.to_string().repeat(255);
        assert_eq!(
            AccountName::parse(&format!(" {limit} ")).unwrap().as_str(),
            limit
        );
        assert_eq!(
            AccountName::parse(&character.to_string().repeat(256)),
            Err(AccountNameError::TooLong)
        );
    }
}

#[test]
fn accounts_retain_identity_owner_type_currency_and_status_without_a_balance() {
    let id = AccountId::from_bytes([1; 16]);
    let owner = AccountOwnerId::from_bytes([2; 16]);
    assert_eq!(id.into_bytes(), [1; 16]);
    assert_eq!(owner.into_bytes(), [2; 16]);
    for account_type in [AccountType::Normal, AccountType::Savings] {
        for currency in [Currency::Sek, Currency::Eur, Currency::Usd] {
            for status in [AccountStatus::Active, AccountStatus::Archived] {
                let account = Account::new(
                    id,
                    owner,
                    AccountName::parse("Savings").unwrap(),
                    account_type,
                    currency,
                    status,
                );
                assert_eq!(account.id(), id);
                assert_eq!(account.owner_id(), owner);
                assert_eq!(account.name().as_str(), "Savings");
                assert_eq!(account.account_type(), account_type);
                assert_eq!(account.currency(), currency);
                assert_eq!(account.status(), status);
            }
        }
    }
}

#[test]
fn duplicate_names_do_not_merge_distinct_accounts() {
    let owner = AccountOwnerId::from_bytes([2; 16]);
    let first = Account::new(
        AccountId::from_bytes([1; 16]),
        owner,
        AccountName::parse("Cash").unwrap(),
        AccountType::Normal,
        Currency::Sek,
        AccountStatus::Active,
    );
    let second = Account::new(
        AccountId::from_bytes([3; 16]),
        owner,
        AccountName::parse("Cash").unwrap(),
        AccountType::Normal,
        Currency::Sek,
        AccountStatus::Active,
    );
    assert_eq!(first.name(), second.name());
    assert_ne!(first.id(), second.id());
}
