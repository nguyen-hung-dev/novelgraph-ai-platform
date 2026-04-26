ALTER TABLE analysis_jobs ADD COLUMN started_at TEXT;
ALTER TABLE analysis_jobs ADD COLUMN finished_at TEXT;
ALTER TABLE analysis_jobs ADD COLUMN error_code TEXT;
ALTER TABLE analysis_jobs ADD COLUMN error_message TEXT;

ALTER TABLE translation_jobs ADD COLUMN started_at TEXT;
ALTER TABLE translation_jobs ADD COLUMN finished_at TEXT;
ALTER TABLE translation_jobs ADD COLUMN error_code TEXT;
ALTER TABLE translation_jobs ADD COLUMN error_message TEXT;
