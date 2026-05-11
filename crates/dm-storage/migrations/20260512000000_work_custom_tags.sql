CREATE TABLE work_custom_tags (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (trim(name) <> ''),
    CHECK (trim(normalized_name) <> ''),
    PRIMARY KEY(work_id, normalized_name)
);

CREATE INDEX work_custom_tags_name_idx ON work_custom_tags(normalized_name, work_id);
