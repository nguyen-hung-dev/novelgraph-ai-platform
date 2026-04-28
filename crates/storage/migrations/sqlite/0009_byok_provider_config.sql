ALTER TABLE llm_provider_configs ADD COLUMN base_url TEXT;
ALTER TABLE llm_provider_configs ADD COLUMN model TEXT;
ALTER TABLE llm_provider_configs ADD COLUMN api_format TEXT NOT NULL DEFAULT 'openai';
ALTER TABLE llm_provider_configs ADD COLUMN key_fingerprint TEXT;
ALTER TABLE llm_provider_configs ADD COLUMN last_checked_at TEXT;
ALTER TABLE llm_provider_configs ADD COLUMN last_health_status TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_provider_configs_user_provider
    ON llm_provider_configs(user_id, provider);
