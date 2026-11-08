-- Current-pointer mutations and revision inserts are committed in one guarded D1 batch.
-- The pointer is intentionally not a circular FK; skill_revisions owns the FK to skills.
CREATE TABLE skills (
    id TEXT PRIMARY KEY NOT NULL,
    current_revision_id TEXT NOT NULL,
    publish_op TEXT NOT NULL UNIQUE
);

CREATE TABLE skill_revisions (
    id TEXT PRIMARY KEY NOT NULL,
    skill_id TEXT NOT NULL REFERENCES skills(id),
    publish_op TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    tags_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL CHECK(updated_at >= 0),
    files_json TEXT NOT NULL,
    manifest_key TEXT NOT NULL,
    manifest_size INTEGER NOT NULL CHECK(manifest_size >= 0),
    zip_key TEXT NOT NULL,
    zip_size INTEGER NOT NULL CHECK(zip_size >= 0),
    total_bytes INTEGER NOT NULL CHECK(total_bytes BETWEEN 0 AND 8388608),
    UNIQUE(skill_id, id)
);

CREATE INDEX skill_revisions_skill ON skill_revisions(skill_id);
