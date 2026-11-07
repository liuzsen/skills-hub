-- Migration number: 0001
CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) = 64),
    email_key TEXT NOT NULL UNIQUE,
    verified_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE admin_roles (
    account_id TEXT PRIMARY KEY NOT NULL REFERENCES accounts(id),
    granted_at INTEGER NOT NULL
);

-- Three independent explicit grants. Admin role does not imply these grants.
CREATE TABLE account_capabilities (
    account_id TEXT PRIMARY KEY NOT NULL REFERENCES accounts(id),
    view_all INTEGER NOT NULL DEFAULT 0 CHECK(view_all IN (0, 1)),
    download_all INTEGER NOT NULL DEFAULT 0 CHECK(download_all IN (0, 1)),
    upload_edit INTEGER NOT NULL DEFAULT 0 CHECK(upload_edit IN (0, 1))
);

CREATE TABLE email_issuance (
    email_key TEXT PRIMARY KEY NOT NULL,
    current_challenge_hash TEXT NOT NULL UNIQUE,
    next_send_at INTEGER NOT NULL
);

CREATE TABLE email_challenges (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) = 64),
    email_key TEXT NOT NULL,
    code_mac TEXT NOT NULL CHECK(length(code_mac) = 64),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0 CHECK(attempts BETWEEN 0 AND 5),
    consumed_op TEXT UNIQUE,
    CHECK(expires_at > created_at)
);
CREATE INDEX email_challenges_email ON email_challenges(email_key);

CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY NOT NULL CHECK(length(token_hash) = 64),
    account_id TEXT NOT NULL REFERENCES accounts(id),
    challenge_id TEXT NOT NULL UNIQUE REFERENCES email_challenges(id),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    revoked_at INTEGER,
    CHECK(expires_at > created_at)
);
CREATE INDEX sessions_account ON sessions(account_id);

-- Local testing only. Plaintext is intentionally confined to ignored local D1 state.
CREATE TABLE local_mail_outbox (
    challenge_id TEXT PRIMARY KEY NOT NULL REFERENCES email_challenges(id),
    recipient TEXT NOT NULL,
    code TEXT NOT NULL CHECK(length(code) = 6),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE TABLE admin_bootstrap_audit (
    account_id TEXT PRIMARY KEY NOT NULL REFERENCES accounts(id),
    initialized_at INTEGER NOT NULL
);
