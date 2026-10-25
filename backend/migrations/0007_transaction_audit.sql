-- Minimal immutable metadata: never copy financial values or free text.
CREATE TABLE transaction_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    actor_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    transaction_id UUID NOT NULL,
    action TEXT COLLATE "C" NOT NULL,
    changed_fields TEXT[] COLLATE "C" NOT NULL,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    CONSTRAINT transaction_audit_transaction_owner FOREIGN KEY (transaction_id, user_id)
        REFERENCES transactions (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT transaction_audit_actor_owner CHECK (actor_id = user_id),
    CONSTRAINT transaction_audit_action_supported CHECK (action IN ('create', 'edit', 'void')),
    CONSTRAINT transaction_audit_safe_fields CHECK (
        CASE
            WHEN cardinality(changed_fields) = 0 THEN TRUE
            WHEN array_ndims(changed_fields) = 1 THEN
                cardinality(changed_fields) <= 11
                AND array_position(changed_fields, NULL) IS NULL
                AND changed_fields <@ ARRAY[
                    'kind', 'state', 'occurred_on', 'amount_minor', 'currency',
                    'account_id', 'destination_account_id', 'received_amount_minor',
                    'received_currency', 'category_id', 'description'
                ]::TEXT[]
            ELSE FALSE
        END
    ),
    CONSTRAINT transaction_audit_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    )
);

CREATE FUNCTION reject_transaction_audit_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Transaction audit is append-only'
        USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER transaction_audit_append_only
    BEFORE UPDATE OR DELETE OR TRUNCATE ON transaction_audit
    FOR EACH STATEMENT EXECUTE FUNCTION reject_transaction_audit_mutation();

CREATE INDEX transaction_audit_transaction_date_idx
    ON transaction_audit (user_id, transaction_id, created_at);
