-- Planning retry results only; existing occurrence/financial history is untouched.
ALTER TABLE recurring_occurrences ADD CONSTRAINT recurring_occurrences_owner
    UNIQUE (id, user_id);

CREATE TABLE recurring_occurrence_action_keys (
    user_id UUID NOT NULL,
    key_hash BYTEA NOT NULL,
    request_hash BYTEA NOT NULL,
    occurrence_id UUID NOT NULL,
    action TEXT COLLATE "C" NOT NULL CHECK (action IN ('skip', 'postpone')),
    result_due_on DATE NOT NULL CHECK (recurring_date_valid(result_due_on)),
    PRIMARY KEY (user_id, key_hash),
    FOREIGN KEY (occurrence_id, user_id) REFERENCES recurring_occurrences(id, user_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK (octet_length(key_hash) = 32 AND octet_length(request_hash) = 32)
);

CREATE FUNCTION reject_recurring_occurrence_action_key_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Occurrence action keys are permanent' USING ERRCODE = '55000';
END;
$$;
CREATE TRIGGER recurring_occurrence_action_keys_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON recurring_occurrence_action_keys
    FOR EACH STATEMENT EXECUTE FUNCTION reject_recurring_occurrence_action_key_mutation();
