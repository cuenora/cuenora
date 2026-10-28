use crate::Currency;
use std::fmt;

/// Account identity as opaque UUID bytes; generation belongs outside the domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountId([u8; 16]);

impl AccountId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// An owner's opaque UUID identity, not evidence of authenticated ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountOwnerId([u8; 16]);

impl AccountOwnerId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountType {
    Normal,
    Savings,
}

/// Business state only; archive audit timestamps belong to the outer layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountStatus {
    Active,
    Archived,
}

/// A display name, not a unique account identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountName(String);

impl AccountName {
    /// Trim Rust Unicode whitespace at both ends, then validate 1–255 Unicode
    /// scalar values with no ASCII controls. Preserve all remaining text exactly.
    pub fn parse(input: &str) -> Result<Self, AccountNameError> {
        let name = input.trim();
        if name.is_empty() {
            return Err(AccountNameError::Empty);
        }
        if name.chars().any(|character| character.is_ascii_control()) {
            return Err(AccountNameError::ControlCharacter);
        }
        if name.chars().count() > 255 {
            return Err(AccountNameError::TooLong);
        }
        Ok(Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Name validation errors never retain the supplied account name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountNameError {
    Empty,
    ControlCharacter,
    TooLong,
}

impl fmt::Display for AccountNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "Account name must not be empty",
            Self::ControlCharacter => "Account name must not contain ASCII control characters",
            Self::TooLong => "Account name must contain at most 255 characters",
        })
    }
}

impl std::error::Error for AccountNameError {}

/// Account metadata without a balance, opening amount or interest projection.
///
/// Construction does not authorize a request or persist a new account. Application
/// use cases must resolve the authenticated owner and enforce financial gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    owner_id: AccountOwnerId,
    name: AccountName,
    account_type: AccountType,
    currency: Currency,
    status: AccountStatus,
}

impl Account {
    pub fn new(
        id: AccountId,
        owner_id: AccountOwnerId,
        name: AccountName,
        account_type: AccountType,
        currency: Currency,
        status: AccountStatus,
    ) -> Self {
        Self {
            id,
            owner_id,
            name,
            account_type,
            currency,
            status,
        }
    }

    pub fn id(&self) -> AccountId {
        self.id
    }

    pub fn owner_id(&self) -> AccountOwnerId {
        self.owner_id
    }

    pub fn name(&self) -> &AccountName {
        &self.name
    }

    pub fn account_type(&self) -> AccountType {
        self.account_type
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn status(&self) -> AccountStatus {
        self.status
    }
}
