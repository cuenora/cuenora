-- Optional configuration only: no financial rows or default terms are created.
-- Effective dates are independent streams: changing cadence does not reset rates.
CREATE TABLE savings_terms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL,
    user_id UUID NOT NULL,
    account_type TEXT COLLATE "C" GENERATED ALWAYS AS ('savings'::TEXT) STORED,
    effective_on DATE NOT NULL CHECK (
        effective_on >= DATE '0001-01-01' AND effective_on < DATE '10000-01-01'
    ),
    payout_frequency TEXT COLLATE "C" NOT NULL CHECK (
        payout_frequency IN ('daily', 'monthly', 'quarterly', 'annual')
    ),
    capitalize BOOLEAN NOT NULL,
    rate_basis TEXT COLLATE "C" NOT NULL CHECK (rate_basis = 'nominal_annual'),
    day_count TEXT COLLATE "C" NOT NULL CHECK (day_count = 'actual_calendar_year'),
    balance_timing TEXT COLLATE "C" NOT NULL CHECK (balance_timing = 'daily_close'),
    rounding TEXT COLLATE "C" NOT NULL CHECK (rounding = 'period_end_half_even'),
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp() CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01+00' AND created_at < TIMESTAMPTZ '10000-01-01+00'
    ),
    CONSTRAINT savings_terms_slot UNIQUE (account_id, effective_on),
    CONSTRAINT savings_terms_account FOREIGN KEY (account_id, user_id, account_type)
        REFERENCES accounts(id, user_id, account_type) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE TABLE savings_rate_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL,
    user_id UUID NOT NULL,
    account_type TEXT COLLATE "C" GENERATED ALWAYS AS ('savings'::TEXT) STORED,
    effective_on DATE NOT NULL CHECK (
        effective_on >= DATE '0001-01-01' AND effective_on < DATE '10000-01-01'
    ),
    -- No NUMERIC(p,s) typmod: it would round input before CHECK sees its scale.
    nominal_annual_percent NUMERIC NOT NULL CHECK (
        nominal_annual_percent >= 0 AND nominal_annual_percent < 'Infinity'::NUMERIC
        AND scale(nominal_annual_percent) <= 6
    ),
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp() CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01+00' AND created_at < TIMESTAMPTZ '10000-01-01+00'
    ),
    CONSTRAINT savings_rates_slot UNIQUE (account_id, effective_on),
    CONSTRAINT savings_rates_account FOREIGN KEY (account_id, user_id, account_type)
        REFERENCES accounts(id, user_id, account_type) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE INDEX savings_terms_owner_idx ON savings_terms(user_id, account_id);
CREATE INDEX savings_rates_owner_idx ON savings_rate_periods(user_id, account_id);

CREATE FUNCTION reject_savings_history_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Savings configuration history is immutable';
END;
$$;
CREATE TRIGGER savings_terms_preserve_history
    BEFORE UPDATE OR DELETE OR TRUNCATE ON savings_terms
    FOR EACH STATEMENT EXECUTE FUNCTION reject_savings_history_mutation();
CREATE TRIGGER savings_rates_preserve_history
    BEFORE UPDATE OR DELETE OR TRUNCATE ON savings_rate_periods
    FOR EACH STATEMENT EXECUTE FUNCTION reject_savings_history_mutation();
