-- Additive permanent retry identity; existing planning/financial rows are untouched.
-- Bind each key to the exact owner, occurrence and linked expense as one tuple.
ALTER TABLE recurring_occurrences ADD CONSTRAINT recurring_occurrences_confirmation_target
    UNIQUE (id, user_id, transaction_id);

CREATE TABLE recurring_confirmation_keys (
    user_id UUID NOT NULL,
    key_hash BYTEA NOT NULL,
    request_hash BYTEA NOT NULL,
    occurrence_id UUID NOT NULL UNIQUE,
    transaction_id UUID NOT NULL UNIQUE,
    PRIMARY KEY (user_id, key_hash),
    FOREIGN KEY (occurrence_id, user_id, transaction_id)
        REFERENCES recurring_occurrences (id, user_id, transaction_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK (octet_length(key_hash) = 32 AND octet_length(request_hash) = 32)
);

CREATE FUNCTION reject_recurring_confirmation_key_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Recurring confirmation keys are permanent' USING ERRCODE = '55000';
END;
$$;
CREATE TRIGGER recurring_confirmation_keys_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON recurring_confirmation_keys
    FOR EACH STATEMENT EXECUTE FUNCTION reject_recurring_confirmation_key_mutation();
