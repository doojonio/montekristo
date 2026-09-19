ALTER TABLE accounts
    ADD COLUMN parent_id TEXT REFERENCES accounts (id);

CREATE INDEX idx_accounts_parent_id ON accounts (parent_id);
