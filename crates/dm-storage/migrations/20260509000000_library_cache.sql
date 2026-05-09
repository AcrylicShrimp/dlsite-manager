CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    login_name TEXT,
    credential_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_login_at TEXT,
    last_sync_at TEXT
);

CREATE INDEX accounts_enabled_idx ON accounts(enabled);

CREATE TABLE works (
    work_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    title_json TEXT NOT NULL,
    maker_id TEXT,
    maker_name TEXT,
    maker_json TEXT,
    work_type TEXT,
    age_category TEXT,
    thumbnail_url TEXT,
    registered_at TEXT,
    published_at TEXT,
    updated_at TEXT,
    raw_json TEXT NOT NULL,
    last_detail_sync_at TEXT NOT NULL
);

CREATE INDEX works_title_idx ON works(title);
CREATE INDEX works_maker_name_idx ON works(maker_name);
CREATE INDEX works_published_at_idx ON works(published_at);

CREATE TABLE sync_runs (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('started', 'completed', 'failed', 'cancelled')),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_code TEXT,
    error_message TEXT
);

CREATE INDEX sync_runs_account_started_idx ON sync_runs(account_id, started_at DESC);

CREATE TABLE account_works (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    purchased_at TEXT,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    last_seen_sync_run_id TEXT REFERENCES sync_runs(id) ON DELETE SET NULL,
    is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0, 1)),
    PRIMARY KEY(account_id, work_id)
);

CREATE INDEX account_works_work_current_idx ON account_works(work_id, is_current);
CREATE INDEX account_works_account_current_idx ON account_works(account_id, is_current);
CREATE INDEX account_works_last_seen_sync_run_idx ON account_works(last_seen_sync_run_id);
