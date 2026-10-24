-- Business events only; movements, audit and authenticated actions come later.
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    kind TEXT COLLATE "C" NOT NULL,
    state TEXT COLLATE "C" NOT NULL DEFAULT 'posted',
    occurred_on DATE NOT NULL,
    amount_minor BIGINT NOT NULL,
    currency TEXT COLLATE "C" NOT NULL,
    account_id UUID,
    destination_account_id UUID,
    received_amount_minor BIGINT,
    received_currency TEXT COLLATE "C",
    category_id UUID,
    category_kind TEXT COLLATE "C" GENERATED ALWAYS AS (
        CASE WHEN kind = 'expense' THEN 'expense'
             WHEN kind IN ('income', 'interest_income') THEN 'income' END
    ) STORED,
    description TEXT,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    updated_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    CONSTRAINT transactions_owner UNIQUE (id, user_id),
    CONSTRAINT transactions_account_owner FOREIGN KEY (account_id, user_id)
        REFERENCES accounts (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT transactions_destination_owner FOREIGN KEY (destination_account_id, user_id)
        REFERENCES accounts (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT transactions_category_kind FOREIGN KEY (category_id, category_kind)
        REFERENCES categories (id, kind) ON DELETE RESTRICT,
    CONSTRAINT transactions_kind_supported CHECK (
        kind IN ('income', 'expense', 'transfer', 'opening_balance', 'interest_income')
    ),
    CONSTRAINT transactions_state_supported CHECK (state IN ('posted', 'voided')),
    CONSTRAINT transactions_amount_storage CHECK (
        amount_minor <> '-9223372036854775808'::BIGINT
        AND (kind = 'opening_balance' OR amount_minor > 0)
    ),
    CONSTRAINT transactions_currency_supported CHECK (currency IN ('SEK', 'EUR', 'USD')),
    CONSTRAINT transactions_received_currency_supported CHECK (
        received_currency IS NULL OR received_currency IN ('SEK', 'EUR', 'USD')
    ),
    CONSTRAINT transactions_linked_event CHECK (
        kind NOT IN ('opening_balance', 'interest_income') OR account_id IS NOT NULL
    ),
    CONSTRAINT transactions_transfer_shape CHECK (
        (kind = 'transfer'
            AND account_id IS NOT NULL AND destination_account_id IS NOT NULL
            AND account_id <> destination_account_id
            AND received_amount_minor IS NOT NULL AND received_amount_minor > 0
            AND received_currency IS NOT NULL
            AND (currency <> received_currency OR amount_minor = received_amount_minor))
        OR (kind <> 'transfer' AND destination_account_id IS NULL
            AND received_amount_minor IS NULL AND received_currency IS NULL)
    ),
    CONSTRAINT transactions_category_shape CHECK (
        category_id IS NULL OR category_kind IS NOT NULL
    ),
    CONSTRAINT transactions_date_range CHECK (
        occurred_on >= DATE '0001-01-01' AND occurred_on < DATE '10000-01-01'
    ),
    CONSTRAINT transactions_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT transactions_updated_at_range CHECK (
        updated_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND updated_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    )
);

CREATE INDEX transactions_user_id_idx ON transactions (user_id);
