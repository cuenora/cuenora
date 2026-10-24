-- Storage only: authenticated account actions and posted movements come later.
CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    account_type TEXT COLLATE "C" NOT NULL,
    currency TEXT COLLATE "C" NOT NULL,
    archived_at TIMESTAMPTZ(6),
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    updated_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    CONSTRAINT accounts_owner UNIQUE (id, user_id),
    CONSTRAINT accounts_name_storage CHECK (
        char_length(name) BETWEEN 1 AND 255
        AND name COLLATE "C" !~ '[[:cntrl:]]'
        AND name COLLATE "C" ~ '[^[:space:]]'
    ),
    CONSTRAINT accounts_type_supported CHECK (account_type IN ('normal', 'savings')),
    CONSTRAINT accounts_currency_supported CHECK (currency IN ('SEK', 'EUR', 'USD')),
    CONSTRAINT accounts_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT accounts_updated_at_range CHECK (
        updated_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND updated_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT accounts_archived_at_range CHECK (
        archived_at IS NULL OR (
            archived_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
            AND archived_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
        )
    )
);

CREATE INDEX accounts_user_id_idx ON accounts (user_id);
