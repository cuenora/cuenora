-- Link inverse postings without rewriting any existing movement.
ALTER TABLE account_movements
    ADD COLUMN reverses_movement_id UUID,
    ADD CONSTRAINT account_movements_reversal_target UNIQUE (id, transaction_id, user_id),
    ADD CONSTRAINT account_movements_reverses_owned_event
        FOREIGN KEY (reverses_movement_id, transaction_id, user_id)
        REFERENCES account_movements (id, transaction_id, user_id) ON DELETE RESTRICT,
    ADD CONSTRAINT account_movements_one_reversal UNIQUE (reverses_movement_id),
    ADD CONSTRAINT account_movements_no_self_reversal CHECK (reverses_movement_id IS DISTINCT FROM id);

-- Permanent operation equivalence, not financial-value or response snapshots.
-- One owner-wide namespace binds each key to target, action and normalized input.
CREATE TABLE transaction_mutation_keys (
    user_id UUID NOT NULL,
    key_hash BYTEA NOT NULL,
    request_hash BYTEA NOT NULL,
    transaction_id UUID NOT NULL,
    action TEXT COLLATE "C" NOT NULL CHECK (action IN ('edit', 'void')),
    PRIMARY KEY (user_id, key_hash),
    CONSTRAINT transaction_mutation_keys_owner FOREIGN KEY (transaction_id, user_id)
        REFERENCES transactions (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT transaction_mutation_keys_hashes CHECK (
        octet_length(key_hash) = 32 AND octet_length(request_hash) = 32
    )
);
CREATE FUNCTION reject_transaction_mutation_key_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Transaction mutation keys are permanent' USING ERRCODE = '55000';
END;
$$;
CREATE TRIGGER transaction_mutation_keys_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON transaction_mutation_keys
    FOR EACH STATEMENT EXECUTE FUNCTION reject_transaction_mutation_key_change();
