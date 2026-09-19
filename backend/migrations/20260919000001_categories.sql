CREATE TABLE categories (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    category_type TEXT NOT NULL CHECK (category_type IN ('expense', 'income')),
    icon          TEXT
) STRICT;

ALTER TABLE transactions
    ADD COLUMN category_id TEXT REFERENCES categories (id);

CREATE INDEX idx_transactions_category_id ON transactions (category_id);
