-- Migration number: 0003
-- Raw codes and bearer tokens never enter this table.
CREATE TABLE redemption_codes (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) = 64),
    code_hash TEXT NOT NULL UNIQUE CHECK(length(code_hash) = 64),
    batch_id TEXT NOT NULL CHECK(length(batch_id) = 64),
    skill_id TEXT NOT NULL REFERENCES skills(id),
    created_by TEXT NOT NULL REFERENCES accounts(id),
    created_at INTEGER NOT NULL,
    bound_account_id TEXT REFERENCES accounts(id),
    guest_token_hash TEXT UNIQUE CHECK(guest_token_hash IS NULL OR length(guest_token_hash) = 64),
    guest_claimed_at INTEGER,
    guest_expires_at INTEGER,
    CHECK (
        (guest_token_hash IS NULL AND guest_claimed_at IS NULL AND guest_expires_at IS NULL)
        OR
        (guest_token_hash IS NOT NULL AND guest_claimed_at IS NOT NULL AND guest_expires_at IS NOT NULL
            AND guest_expires_at = guest_claimed_at + 600)
    )
);
CREATE INDEX redemption_codes_account_skill ON redemption_codes(bound_account_id, skill_id);
CREATE INDEX redemption_codes_batch ON redemption_codes(batch_id);
