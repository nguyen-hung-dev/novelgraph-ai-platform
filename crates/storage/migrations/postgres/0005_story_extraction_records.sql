CREATE TABLE IF NOT EXISTS story_extraction_records (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    novel_id TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    job_id TEXT NOT NULL REFERENCES analysis_jobs(id) ON DELETE CASCADE,
    run_id TEXT NOT NULL REFERENCES analysis_chapter_runs(id) ON DELETE CASCADE,
    chapter_num INTEGER NOT NULL,
    group_key TEXT NOT NULL,
    group_label TEXT NOT NULL,
    entity_key TEXT,
    display_name TEXT NOT NULL,
    prompt_schema_version TEXT NOT NULL,
    raw_record_json TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_story_extraction_records_job_group
    ON story_extraction_records(job_id, group_key, chapter_num);

CREATE INDEX IF NOT EXISTS idx_story_extraction_records_chapter
    ON story_extraction_records(chapter_id, group_key);

CREATE TABLE IF NOT EXISTS story_extraction_fields (
    id TEXT PRIMARY KEY,
    record_id TEXT NOT NULL REFERENCES story_extraction_records(id) ON DELETE CASCADE,
    field_key TEXT NOT NULL,
    field_label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_story_extraction_fields_record
    ON story_extraction_fields(record_id, field_key);

CREATE TABLE IF NOT EXISTS story_extraction_values (
    id TEXT PRIMARY KEY,
    field_id TEXT NOT NULL REFERENCES story_extraction_fields(id) ON DELETE CASCADE,
    value_text TEXT NOT NULL,
    confidence DOUBLE PRECISION,
    evidence_json TEXT NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_story_extraction_values_field
    ON story_extraction_values(field_id);
