-- Permanent owner-scoped creation retries, independent of mutable rule terms.
CREATE TABLE recurring_rule_creation_keys (
    user_id UUID NOT NULL,
    key_hash BYTEA NOT NULL,
    request_hash BYTEA NOT NULL,
    rule_id UUID NOT NULL UNIQUE,
    PRIMARY KEY (user_id, key_hash),
    FOREIGN KEY (rule_id, user_id) REFERENCES recurring_rules (id, user_id) ON DELETE RESTRICT,
    CHECK (octet_length(key_hash) = 32 AND octet_length(request_hash) = 32)
);

CREATE FUNCTION reject_recurring_rule_creation_key_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Recurring rule creation keys are permanent' USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER recurring_rule_creation_keys_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON recurring_rule_creation_keys
    FOR EACH STATEMENT EXECUTE FUNCTION reject_recurring_rule_creation_key_mutation();
