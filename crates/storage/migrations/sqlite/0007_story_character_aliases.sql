CREATE TABLE IF NOT EXISTS story_character_aliases (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    novel_id TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
    job_id TEXT NOT NULL REFERENCES analysis_jobs(id) ON DELETE CASCADE,
    entity_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    alias_text TEXT NOT NULL,
    alias_key TEXT NOT NULL,
    alias_type TEXT NOT NULL,
    alias_label TEXT NOT NULL,
    confidence REAL,
    first_chapter_num INTEGER NOT NULL,
    evidence_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, job_id, entity_key, alias_key)
);

CREATE INDEX IF NOT EXISTS idx_story_character_aliases_job_entity
    ON story_character_aliases(job_id, entity_key, first_chapter_num);

CREATE INDEX IF NOT EXISTS idx_story_character_aliases_alias_key
    ON story_character_aliases(project_id, job_id, alias_key);
