-- Creation-only idempotency: retain the original normalized result for the
-- lifetime of its owned account. No TTL or background expiry is permitted.
CREATE TABLE account_creation_keys (
    user_id UUID NOT NULL,
    key_hash BYTEA NOT NULL,
    account_id UUID NOT NULL UNIQUE,
    name TEXT NOT NULL,
    account_type TEXT COLLATE "C" NOT NULL,
    currency TEXT COLLATE "C" NOT NULL,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    PRIMARY KEY (user_id, key_hash),
    CONSTRAINT account_creation_keys_owner FOREIGN KEY (account_id, user_id)
        REFERENCES accounts (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT account_creation_keys_hash_length CHECK (octet_length(key_hash) = 32),
    CONSTRAINT account_creation_keys_name CHECK (
        char_length(name) BETWEEN 1 AND 255
        AND name COLLATE "C" !~ '[[:cntrl:]]'
        AND name COLLATE "C" ~ '[^[:space:]]'
    ),
    CONSTRAINT account_creation_keys_type CHECK (account_type IN ('normal', 'savings')),
    CONSTRAINT account_creation_keys_currency CHECK (currency IN ('SEK', 'EUR', 'USD')),
    CONSTRAINT account_creation_keys_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    )
);
