-- Canonical entered quote only; never infer one or rewrite historical amounts.
ALTER TABLE transactions
    ADD COLUMN quoted_rate TEXT COLLATE "C",
    ADD CONSTRAINT transactions_quoted_rate_shape CHECK (
        CASE
            WHEN quoted_rate IS NULL THEN TRUE
            WHEN kind = 'transfer'
                AND octet_length(quoted_rate) <= 52
                AND quoted_rate ~ '^(0|[1-9][0-9]*)(\.[0-9]{0,11}[1-9])?$'
                AND quoted_rate <> '0'
            THEN replace(quoted_rate, '.', '')::numeric
                <= 170141183460469231731687303715884105727
                AND replace(quoted_rate, '.', '')::numeric * amount_minor
                    <= 170141183460469231731687303715884105727
                AND quoted_rate::numeric * amount_minor < 9223372036854775807.5
            ELSE FALSE
        END
    );
