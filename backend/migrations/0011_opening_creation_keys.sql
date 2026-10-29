-- Extend permanent creation equivalence without replacing any prior key/result.
-- Existing P05.02 records are zero/no-date (no opening instruction).
ALTER TABLE account_creation_keys
    ADD COLUMN opening_amount_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN opening_occurred_on DATE,
    ADD CONSTRAINT account_creation_keys_opening_amount CHECK (
        opening_amount_minor <> '-9223372036854775808'::BIGINT
    ),
    ADD CONSTRAINT account_creation_keys_opening_date CHECK (
        opening_occurred_on IS NULL OR (
            opening_occurred_on >= DATE '0001-01-01'
            AND opening_occurred_on < DATE '10000-01-01'
        )
    ),
    ADD CONSTRAINT account_creation_keys_nonzero_dated CHECK (
        opening_amount_minor = 0 OR opening_occurred_on IS NOT NULL
    );
