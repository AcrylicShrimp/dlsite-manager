CREATE TABLE download_finalizations (
    work_id TEXT PRIMARY KEY NOT NULL,
    operation_id TEXT UNIQUE NOT NULL,
    staging_path TEXT NOT NULL,
    final_path TEXT NOT NULL,
    old_path TEXT,
    temporary_path TEXT NOT NULL,
    committed INTEGER NOT NULL DEFAULT 0 CHECK (committed IN (0, 1))
);
