-- Effective posting snapshots only; corrections append compensating entries.
CREATE TABLE account_movements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    transaction_id UUID NOT NULL,
    account_id UUID NOT NULL,
    signed_amount_minor BIGINT NOT NULL,
    currency TEXT COLLATE "C" NOT NULL,
    occurred_on DATE NOT NULL,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    CONSTRAINT account_movements_account_owner FOREIGN KEY (account_id, user_id)
        REFERENCES accounts (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT account_movements_transaction_owner FOREIGN KEY (transaction_id, user_id)
        REFERENCES transactions (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT account_movements_amount_storage CHECK (
        signed_amount_minor <> '-9223372036854775808'::BIGINT
    ),
    CONSTRAINT account_movements_currency_supported CHECK (currency IN ('SEK', 'EUR', 'USD')),
    CONSTRAINT account_movements_date_range CHECK (
        occurred_on >= DATE '0001-01-01' AND occurred_on < DATE '10000-01-01'
    ),
    CONSTRAINT account_movements_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    )
);

CREATE FUNCTION reject_account_movement_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Account movements are append-only'
        USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER account_movements_append_only
    BEFORE UPDATE OR DELETE OR TRUNCATE ON account_movements
    FOR EACH STATEMENT EXECUTE FUNCTION reject_account_movement_mutation();

CREATE INDEX account_movements_account_date_idx
    ON account_movements (user_id, account_id, occurred_on);
CREATE INDEX account_movements_user_date_idx
    ON account_movements (user_id, occurred_on);
CREATE INDEX account_movements_transaction_idx
    ON account_movements (transaction_id, user_id);
