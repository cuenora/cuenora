-- Planning storage only: no financial event or movement is generated here.
CREATE FUNCTION recurring_date_valid(value DATE) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT value >= DATE '0001-01-01' AND value < DATE '10000-01-01'
$$;

CREATE FUNCTION recurring_exclusions_valid(value DATEMULTIRANGE) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT NOT EXISTS (
        SELECT 1 FROM unnest(value) AS r
        WHERE lower_inf(r) OR upper_inf(r)
            OR NOT recurring_date_valid(lower(r)) OR upper(r) > DATE '10000-01-01'
    )
$$;

CREATE TABLE recurring_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    amount_minor BIGINT NOT NULL CHECK (amount_minor > 0),
    currency TEXT COLLATE "C" NOT NULL CHECK (currency IN ('SEK', 'EUR', 'USD')),
    account_id UUID,
    category_id UUID,
    category_kind TEXT COLLATE "C" GENERATED ALWAYS AS ('expense'::TEXT) STORED,
    description TEXT,
    cadence TEXT COLLATE "C" NOT NULL CHECK (cadence IN ('weekly', 'monthly', 'quarterly', 'annual')),
    starts_on DATE NOT NULL CHECK (recurring_date_valid(starts_on)),
    nominal_day SMALLINT GENERATED ALWAYS AS (extract(day FROM starts_on)::SMALLINT) STORED,
    ends_on DATE CHECK (recurring_date_valid(ends_on) AND ends_on >= starts_on),
    time_zone TEXT NOT NULL CHECK (
        octet_length(time_zone) BETWEEN 1 AND 255
        AND time_zone COLLATE "C" !~ '[[:space:][:cntrl:]]'
        AND time_zone !~ '^[+-]'
    ),
    state TEXT COLLATE "C" NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'paused')),
    paused_on DATE CHECK (recurring_date_valid(paused_on)),
    excluded_dates DATEMULTIRANGE NOT NULL DEFAULT '{}' CHECK (recurring_exclusions_valid(excluded_dates)),
    generated_through DATE CHECK (recurring_date_valid(generated_through)),
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    updated_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    CONSTRAINT recurring_rules_owner UNIQUE (id, user_id),
    CONSTRAINT recurring_rules_pause_shape CHECK ((state = 'paused') = (paused_on IS NOT NULL)),
    CONSTRAINT recurring_rules_account FOREIGN KEY (account_id, user_id, currency)
        REFERENCES accounts(id, user_id, currency) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT recurring_rules_category FOREIGN KEY (category_id, category_kind)
        REFERENCES categories(id, kind) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK (created_at >= TIMESTAMPTZ '0001-01-01+00' AND created_at < TIMESTAMPTZ '10000-01-01+00'),
    CHECK (updated_at >= TIMESTAMPTZ '0001-01-01+00' AND updated_at < TIMESTAMPTZ '10000-01-01+00')
);

-- The composite reference protects expense kind against reverse updates as well.
ALTER TABLE transactions ADD CONSTRAINT transactions_owner_kind UNIQUE (id, user_id, kind);

CREATE TABLE recurring_occurrences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id UUID NOT NULL,
    user_id UUID NOT NULL,
    scheduled_on DATE NOT NULL CHECK (recurring_date_valid(scheduled_on)),
    due_on DATE NOT NULL CHECK (recurring_date_valid(due_on)),
    amount_minor BIGINT NOT NULL CHECK (amount_minor > 0),
    currency TEXT COLLATE "C" NOT NULL CHECK (currency IN ('SEK', 'EUR', 'USD')),
    account_id UUID,
    category_id UUID,
    expense_kind TEXT COLLATE "C" GENERATED ALWAYS AS ('expense'::TEXT) STORED,
    description TEXT,
    state TEXT COLLATE "C" NOT NULL DEFAULT 'planned' CHECK (state IN ('planned', 'confirmed', 'skipped')),
    transaction_id UUID UNIQUE,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    updated_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    CONSTRAINT recurring_occurrences_slot UNIQUE (rule_id, scheduled_on),
    CONSTRAINT recurring_occurrences_rule FOREIGN KEY (rule_id, user_id)
        REFERENCES recurring_rules(id, user_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT recurring_occurrences_account FOREIGN KEY (account_id, user_id, currency)
        REFERENCES accounts(id, user_id, currency) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT recurring_occurrences_category FOREIGN KEY (category_id, expense_kind)
        REFERENCES categories(id, kind) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT recurring_occurrences_expense FOREIGN KEY (transaction_id, user_id, expense_kind)
        REFERENCES transactions(id, user_id, kind) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT recurring_occurrences_confirmation CHECK ((state = 'confirmed') = (transaction_id IS NOT NULL)),
    CHECK (created_at >= TIMESTAMPTZ '0001-01-01+00' AND created_at < TIMESTAMPTZ '10000-01-01+00'),
    CHECK (updated_at >= TIMESTAMPTZ '0001-01-01+00' AND updated_at < TIMESTAMPTZ '10000-01-01+00')
);

CREATE INDEX recurring_rules_owner_state_idx ON recurring_rules(user_id, state);
CREATE INDEX recurring_occurrences_pending_idx ON recurring_occurrences(user_id, due_on, id)
    WHERE state = 'planned';

CREATE FUNCTION reject_recurring_deletion() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Recurrence history cannot be removed';
END;
$$;
CREATE TRIGGER recurring_rules_preserve_history BEFORE DELETE OR TRUNCATE ON recurring_rules
    FOR EACH STATEMENT EXECUTE FUNCTION reject_recurring_deletion();
CREATE TRIGGER recurring_occurrences_preserve_history BEFORE DELETE OR TRUNCATE ON recurring_occurrences
    FOR EACH STATEMENT EXECUTE FUNCTION reject_recurring_deletion();

CREATE FUNCTION guard_recurring_identity() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(NEW.id, NEW.user_id, NEW.created_at) IS DISTINCT FROM ROW(OLD.id, OLD.user_id, OLD.created_at) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Recurrence identity is immutable';
    END IF;
    IF TG_TABLE_NAME = 'recurring_occurrences' THEN
        IF ROW(NEW.rule_id, NEW.scheduled_on) IS DISTINCT FROM ROW(OLD.rule_id, OLD.scheduled_on)
            OR (OLD.state <> 'planned' AND NEW IS DISTINCT FROM OLD) THEN
            RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Occurrence identity and terminal history are immutable';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER recurring_rules_identity BEFORE UPDATE ON recurring_rules
    FOR EACH ROW EXECUTE FUNCTION guard_recurring_identity();
CREATE TRIGGER recurring_occurrences_identity BEFORE UPDATE ON recurring_occurrences
    FOR EACH ROW EXECUTE FUNCTION guard_recurring_identity();
