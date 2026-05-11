CREATE TABLE work_downloads (
    work_id TEXT PRIMARY KEY NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('downloading', 'downloaded', 'failed', 'cancelled')),
    local_path TEXT NULL,
    staging_path TEXT NULL,
    unpack_policy TEXT NOT NULL DEFAULT 'unpack_when_recognized',
    bytes_received INTEGER NOT NULL DEFAULT 0 CHECK (bytes_received >= 0),
    bytes_total INTEGER NULL CHECK (bytes_total IS NULL OR bytes_total >= 0),
    error_code TEXT NULL,
    error_message TEXT NULL,
    started_at TEXT NULL,
    completed_at TEXT NULL,
    updated_at TEXT NOT NULL
);
