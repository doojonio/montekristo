CREATE TABLE accounts (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('asset', 'liability', 'income', 'expense', 'equity')),
    currency     TEXT NOT NULL CHECK (length(currency) = 3)
) STRICT;

CREATE TABLE transactions (
    id          TEXT PRIMARY KEY,
    date        TEXT NOT NULL,
    description TEXT NOT NULL
) STRICT;

CREATE TABLE postings (
    id             INTEGER PRIMARY KEY,
    transaction_id TEXT NOT NULL REFERENCES transactions (id) ON DELETE CASCADE,
    account_id     TEXT NOT NULL REFERENCES accounts (id),
    -- Monetary amounts are stored as decimal text to preserve exact precision.
    amount         TEXT NOT NULL
) STRICT;

CREATE INDEX idx_postings_transaction_id ON postings (transaction_id);
CREATE INDEX idx_postings_account_id ON postings (account_id);
