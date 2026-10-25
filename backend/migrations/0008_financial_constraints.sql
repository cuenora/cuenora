-- Native reference constraints validate existing data and concurrent key changes.
ALTER TABLE accounts
    ADD CONSTRAINT accounts_owner_currency UNIQUE (id, user_id, currency),
    ADD CONSTRAINT accounts_owner_type UNIQUE (id, user_id, account_type);

ALTER TABLE transactions
    ADD COLUMN income_account_type TEXT COLLATE "C" GENERATED ALWAYS AS (
        CASE WHEN kind = 'income' AND account_id IS NOT NULL THEN 'normal' END
    ) STORED,
    ADD CONSTRAINT transactions_account_currency
        FOREIGN KEY (account_id, user_id, currency)
        REFERENCES accounts (id, user_id, currency)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    ADD CONSTRAINT transactions_destination_currency
        FOREIGN KEY (destination_account_id, user_id, received_currency)
        REFERENCES accounts (id, user_id, currency)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    ADD CONSTRAINT transactions_income_account_type
        FOREIGN KEY (account_id, user_id, income_account_type)
        REFERENCES accounts (id, user_id, account_type)
        ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE account_movements
    ADD CONSTRAINT account_movements_account_currency
        FOREIGN KEY (account_id, user_id, currency)
        REFERENCES accounts (id, user_id, currency)
        ON UPDATE RESTRICT ON DELETE RESTRICT;

CREATE INDEX transactions_user_date_idx ON transactions (user_id, occurred_on);
