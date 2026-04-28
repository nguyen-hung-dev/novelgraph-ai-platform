use novelgraph_core::ByokProviderConfigRecord;
use sqlx::{sqlite::SqliteRow, Row};

use crate::{
    sqlite::{prefixed_id, require_text, SqliteStore, LOCAL_USER_ID},
    StorageError, StorageResult,
};

impl SqliteStore {
    pub async fn get_local_byok_provider_config(
        &self,
    ) -> StorageResult<Option<ByokProviderConfigRecord>> {
        self.ensure_local_workspace().await?;

        let row = sqlx::query(
            "SELECT id, user_id, provider, display_name, base_url, model, api_format,
                    encrypted_secret_ref, key_fingerprint, session_only, last_checked_at,
                    last_health_status, created_at, updated_at
             FROM llm_provider_configs
             WHERE user_id = ?
             ORDER BY updated_at DESC, created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(LOCAL_USER_ID)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(byok_provider_config_from_row))
    }

    pub async fn get_local_byok_provider_config_for_provider(
        &self,
        provider: &str,
    ) -> StorageResult<Option<ByokProviderConfigRecord>> {
        self.ensure_local_workspace().await?;
        let provider = require_text(provider, "provider")?;

        let row = sqlx::query(
            "SELECT id, user_id, provider, display_name, base_url, model, api_format,
                    encrypted_secret_ref, key_fingerprint, session_only, last_checked_at,
                    last_health_status, created_at, updated_at
             FROM llm_provider_configs
             WHERE user_id = ? AND provider = ?",
        )
        .bind(LOCAL_USER_ID)
        .bind(provider)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(byok_provider_config_from_row))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn save_local_byok_provider_config(
        &self,
        provider: &str,
        display_name: &str,
        base_url: &str,
        model: &str,
        api_format: &str,
        encrypted_secret_ref: Option<&str>,
        key_fingerprint: Option<&str>,
        session_only: bool,
    ) -> StorageResult<ByokProviderConfigRecord> {
        self.ensure_local_workspace().await?;
        let provider = require_text(provider, "provider")?;
        let display_name = require_text(display_name, "provider display name")?;
        let base_url = require_text(base_url, "provider base URL")?;
        let model = require_text(model, "provider model")?;
        let api_format = require_text(api_format, "provider API format")?;
        let config_id = prefixed_id("llm_cfg");

        sqlx::query(
            "INSERT INTO llm_provider_configs (
                id, user_id, provider, display_name, base_url, model, api_format,
                encrypted_secret_ref, key_fingerprint, session_only
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id, provider) DO UPDATE SET
                display_name = excluded.display_name,
                base_url = excluded.base_url,
                model = excluded.model,
                api_format = excluded.api_format,
                encrypted_secret_ref = COALESCE(
                    excluded.encrypted_secret_ref,
                    llm_provider_configs.encrypted_secret_ref
                ),
                key_fingerprint = COALESCE(
                    excluded.key_fingerprint,
                    llm_provider_configs.key_fingerprint
                ),
                session_only = excluded.session_only,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(config_id)
        .bind(LOCAL_USER_ID)
        .bind(provider.as_str())
        .bind(display_name)
        .bind(base_url)
        .bind(model)
        .bind(api_format)
        .bind(encrypted_secret_ref)
        .bind(key_fingerprint)
        .bind(if session_only { 1 } else { 0 })
        .execute(self.pool())
        .await?;

        self.get_local_byok_provider_config_for_provider(&provider)
            .await?
            .ok_or(StorageError::NotFound("llm_provider_config"))
    }

    pub async fn update_local_byok_provider_health(
        &self,
        provider: &str,
        last_health_status: &str,
    ) -> StorageResult<()> {
        self.ensure_local_workspace().await?;
        let provider = require_text(provider, "provider")?;
        let last_health_status = require_text(last_health_status, "health status")?;

        sqlx::query(
            "UPDATE llm_provider_configs
             SET last_checked_at = CURRENT_TIMESTAMP,
                 last_health_status = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE user_id = ? AND provider = ?",
        )
        .bind(last_health_status)
        .bind(LOCAL_USER_ID)
        .bind(provider)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}

fn byok_provider_config_from_row(row: SqliteRow) -> ByokProviderConfigRecord {
    let session_only: i64 = row.get("session_only");

    ByokProviderConfigRecord {
        id: row.get("id"),
        user_id: row.get("user_id"),
        provider: row.get("provider"),
        display_name: row.get("display_name"),
        base_url: row.get("base_url"),
        model: row.get("model"),
        api_format: row.get("api_format"),
        encrypted_secret_ref: row.get("encrypted_secret_ref"),
        key_fingerprint: row.get("key_fingerprint"),
        session_only: session_only != 0,
        last_checked_at: row.get("last_checked_at"),
        last_health_status: row.get("last_health_status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
