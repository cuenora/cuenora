-- Global built-ins only; custom categories and application actions come later.
CREATE TABLE categories (
    id UUID PRIMARY KEY,
    code TEXT COLLATE "C" NOT NULL UNIQUE,
    kind TEXT COLLATE "C" NOT NULL,
    name TEXT NOT NULL,
    CONSTRAINT categories_kind_reference UNIQUE (id, kind),
    CONSTRAINT categories_code_storage CHECK (code ~ '^[a-z]+(_[a-z]+)*$'),
    CONSTRAINT categories_kind_supported CHECK (kind IN ('income', 'expense')),
    CONSTRAINT categories_name_storage CHECK (
        char_length(name) BETWEEN 1 AND 255
        AND name COLLATE "C" !~ '[[:cntrl:]]'
        AND name COLLATE "C" ~ '[^[:space:]]'
    )
);

-- Fixed identities are identical on every fresh database; codes are not labels.
INSERT INTO categories (id, code, kind, name) VALUES
    ('10000000-0000-4000-8000-000000000001', 'salary', 'income', 'Salary'),
    ('10000000-0000-4000-8000-000000000002', 'interest', 'income', 'Interest'),
    ('10000000-0000-4000-8000-000000000003', 'other_income', 'income', 'Other income'),
    ('10000000-0000-4000-8000-000000000004', 'housing', 'expense', 'Housing'),
    ('10000000-0000-4000-8000-000000000005', 'groceries', 'expense', 'Groceries'),
    ('10000000-0000-4000-8000-000000000006', 'transport', 'expense', 'Transport'),
    ('10000000-0000-4000-8000-000000000007', 'utilities', 'expense', 'Utilities'),
    ('10000000-0000-4000-8000-000000000008', 'health', 'expense', 'Health'),
    ('10000000-0000-4000-8000-000000000009', 'leisure', 'expense', 'Leisure'),
    ('10000000-0000-4000-8000-000000000010', 'other_expense', 'expense', 'Other expense');

-- Catalog maintenance requires an intentional migration, not ordinary DML.
CREATE FUNCTION reject_builtin_category_write() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Built-in category catalog is read-only'
        USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER categories_read_only
    BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON categories
    FOR EACH STATEMENT EXECUTE FUNCTION reject_builtin_category_write();
