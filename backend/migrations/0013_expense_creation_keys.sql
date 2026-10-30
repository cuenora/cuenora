-- Permanent accountless-expense retry equivalence. No financial value snapshots.
CREATE TABLE expense_creation_keys (
    user_id UUID NOT NULL,
    key_hash BYTEA NOT NULL,
    request_hash BYTEA NOT NULL,
    transaction_id UUID NOT NULL UNIQUE,
    PRIMARY KEY (user_id, key_hash),
    CONSTRAINT expense_creation_keys_owner FOREIGN KEY (transaction_id, user_id)
        REFERENCES transactions (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT expense_creation_keys_hashes CHECK (
        octet_length(key_hash) = 32 AND octet_length(request_hash) = 32
    )
);

CREATE FUNCTION reject_expense_creation_key_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Expense creation keys are permanent'
        USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER expense_creation_keys_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON expense_creation_keys
    FOR EACH STATEMENT EXECUTE FUNCTION reject_expense_creation_key_mutation();
