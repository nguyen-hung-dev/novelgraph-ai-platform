use std::{collections::HashMap, path::Path, str::FromStr};

use novelgraph_core::{
    detect_basic_source_language, split_chapters, split_source_segments, AnalysisChapterRun,
    AnalysisJob, ByokProviderConfigRecord, Chapter, CreateTranslationJobInput, DeleteProjectResult,
    JobEvent, Novel, NovelImportInput, NovelImportResult, NovelMetadataUpdateInput, Project,
    StoryCharacterAliasView, StoryEvidenceSpan, StoryExtractionDocument,
    StoryExtractionFieldValueView, StoryExtractionFieldView, StoryExtractionRecordPayload,
    StoryExtractionRecordView, TranslationJob,
};
use novelgraph_jobs::{cancel_event_type, validate_transition, JobKind, JobStatus};
use serde_json::json;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};
use uuid::Uuid;

use crate::{StorageError, StorageResult};

const LOCAL_USER_ID: &str = "user_local";
const LOCAL_WORKSPACE_ID: &str = "ws_local";

#[derive(Debug, Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn connect(database_path: Option<&str>) -> StorageResult<Self> {
        if let Some(path) = database_path {
            if let Some(parent) = Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
        }

        let options = match database_path {
            Some(path) => SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .foreign_keys(true),
            None => SqliteConnectOptions::new()
                .in_memory(true)
                .create_if_missing(true)
                .foreign_keys(true),
        };
        let max_connections = if database_path.is_some() { 5 } else { 1 };
        Self::connect_with_options(options, max_connections).await
    }

    pub async fn connect_url(database_url: &str) -> StorageResult<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true);
        Self::connect_with_options(options, 5).await
    }

    pub async fn connect_in_memory() -> StorageResult<Self> {
        Self::connect(None).await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create_project(&self, name: &str) -> StorageResult<Project> {
        let name = require_text(name, "project name")?;
        self.ensure_local_workspace().await?;

        let project_id = prefixed_id("proj");
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, name, visibility) VALUES (?, ?, ?, 'private')",
        )
        .bind(&project_id)
        .bind(LOCAL_WORKSPACE_ID)
        .bind(name)
        .execute(&self.pool)
        .await?;

        self.get_project(&project_id)
            .await?
            .ok_or(StorageError::NotFound("project"))
    }

    pub async fn list_projects(&self) -> StorageResult<Vec<Project>> {
        self.ensure_local_workspace().await?;

        let rows = sqlx::query(
            "SELECT id, workspace_id, name, visibility, created_at, updated_at
             FROM projects
             WHERE deleted_at IS NULL
             ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(project_from_row).collect())
    }

    pub async fn list_archived_projects(&self) -> StorageResult<Vec<Project>> {
        self.ensure_local_workspace().await?;

        let rows = sqlx::query(
            "SELECT id, workspace_id, name, visibility, created_at, updated_at
             FROM projects
             WHERE deleted_at IS NOT NULL
             ORDER BY updated_at DESC, created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(project_from_row).collect())
    }

    pub async fn get_project(&self, project_id: &str) -> StorageResult<Option<Project>> {
        let row = sqlx::query(
            "SELECT id, workspace_id, name, visibility, created_at, updated_at
             FROM projects
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(project_from_row))
    }

    pub async fn delete_project(
        &self,
        project_id: &str,
        purge_data: bool,
    ) -> StorageResult<DeleteProjectResult> {
        let existing = self
            .get_project_including_deleted(project_id)
            .await?
            .ok_or(StorageError::NotFound("project"))?;

        if existing.deleted_at.is_some() {
            if purge_data {
                sqlx::query("DELETE FROM projects WHERE id = ?")
                    .bind(project_id)
                    .execute(&self.pool)
                    .await?;

                return Ok(DeleteProjectResult {
                    project_id: project_id.to_string(),
                    action: "purged".to_string(),
                    data_retained: false,
                });
            }

            return Err(StorageError::InvalidInput(
                "project is already archived".to_string(),
            ));
        }

        if purge_data {
            sqlx::query("DELETE FROM projects WHERE id = ?")
                .bind(project_id)
                .execute(&self.pool)
                .await?;

            return Ok(DeleteProjectResult {
                project_id: project_id.to_string(),
                action: "purged".to_string(),
                data_retained: false,
            });
        }

        sqlx::query(
            "UPDATE projects
             SET deleted_at = CURRENT_TIMESTAMP,
                 visibility = 'archived',
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
        )
        .bind(project_id)
        .execute(&self.pool)
        .await?;

        Ok(DeleteProjectResult {
            project_id: project_id.to_string(),
            action: "archived".to_string(),
            data_retained: true,
        })
    }

    pub async fn restore_project(&self, project_id: &str) -> StorageResult<Project> {
        let existing = self
            .get_project_including_deleted(project_id)
            .await?
            .ok_or(StorageError::NotFound("project"))?;

        if existing.deleted_at.is_none() {
            return Err(StorageError::InvalidInput(
                "project is not archived".to_string(),
            ));
        }

        sqlx::query(
            "UPDATE projects
             SET deleted_at = NULL,
                 visibility = 'private',
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
        )
        .bind(project_id)
        .execute(&self.pool)
        .await?;

        self.get_project(project_id)
            .await?
            .ok_or(StorageError::NotFound("project"))
    }

    pub async fn import_novel(
        &self,
        project_id: &str,
        input: NovelImportInput,
    ) -> StorageResult<NovelImportResult> {
        let title = require_text(&input.title, "novel title")?;
        let text = require_text(&input.text, "novel text")?;
        let chapters = split_chapters(&text);
        if chapters.is_empty() {
            return Err(StorageError::InvalidInput(
                "novel text did not produce any chapter".to_string(),
            ));
        }
        let chapter_count = chapters.len();

        self.require_project(project_id).await?;

        let novel_id = prefixed_id("novel");
        let analysis_job_id = prefixed_id("job");
        let mut source_segment_count = 0usize;
        let mut tx = self.pool.begin().await?;

        let source_language = normalize_source_language(input.source_language.clone(), Some(&text));

        sqlx::query(
            "INSERT INTO novels (
                id, project_id, title, author, source_language, genre, description
             )
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&novel_id)
        .bind(project_id)
        .bind(title)
        .bind(optional_trimmed(input.author))
        .bind(source_language)
        .bind(optional_trimmed(input.genre))
        .bind(optional_trimmed(input.description))
        .execute(&mut *tx)
        .await?;

        for chapter in chapters {
            let chapter_id = prefixed_id("chap");
            sqlx::query(
                "INSERT INTO chapters (id, novel_id, chapter_num, title, content)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&chapter_id)
            .bind(&novel_id)
            .bind(chapter.chapter_num)
            .bind(&chapter.title)
            .bind(&chapter.content)
            .execute(&mut *tx)
            .await?;

            for segment in split_source_segments(&chapter.content) {
                let source_segment_id = prefixed_id("seg");
                sqlx::query(
                    "INSERT INTO source_segments (
                        id, novel_id, chapter_id, segment_index, start_char, end_char,
                        segment_kind, text
                     )
                     VALUES (?, ?, ?, ?, ?, ?, 'paragraph', ?)",
                )
                .bind(source_segment_id)
                .bind(&novel_id)
                .bind(&chapter_id)
                .bind(segment.segment_index)
                .bind(segment.start_char as i64)
                .bind(segment.end_char as i64)
                .bind(segment.text)
                .execute(&mut *tx)
                .await?;
                source_segment_count += 1;
            }
        }

        let payload_json = json!({
            "novel_id": novel_id,
            "source": "import_confirm",
        })
        .to_string();
        sqlx::query(
            "INSERT INTO analysis_jobs (id, project_id, novel_id, job_type, status, payload_json)
             VALUES (?, ?, ?, 'chapter_analysis_batch', 'pending', ?)",
        )
        .bind(&analysis_job_id)
        .bind(project_id)
        .bind(&novel_id)
        .bind(payload_json)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO job_events (
                id, project_id, job_id, job_kind, sequence, event_type, payload_json
             )
             VALUES (?, ?, ?, 'analysis', 1, 'analysis_job_created', ?)",
        )
        .bind(prefixed_id("evt"))
        .bind(project_id)
        .bind(&analysis_job_id)
        .bind(
            json!({
                "novel_id": novel_id,
                "chapter_count": chapter_count,
                "source_segment_count": source_segment_count,
            })
            .to_string(),
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let novel = self
            .get_novel(project_id, &novel_id)
            .await?
            .ok_or(StorageError::NotFound("novel"))?;
        let chapters = self.list_chapters(project_id, &novel_id).await?;
        let analysis_job = self
            .get_analysis_job(project_id, &analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))?;

        Ok(NovelImportResult {
            novel,
            chapters,
            source_segment_count,
            analysis_job,
        })
    }

    pub async fn get_novel(
        &self,
        project_id: &str,
        novel_id: &str,
    ) -> StorageResult<Option<Novel>> {
        let row = sqlx::query(
            "SELECT id, project_id, title, author, source_language, genre, description,
                    created_at, updated_at
             FROM novels
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(novel_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(novel_from_row))
    }

    pub async fn list_novels(&self, project_id: &str) -> StorageResult<Vec<Novel>> {
        self.require_project(project_id).await?;

        let rows = sqlx::query(
            "SELECT id, project_id, title, author, source_language, genre, description,
                    created_at, updated_at
             FROM novels
             WHERE project_id = ?
             ORDER BY updated_at DESC, created_at DESC, id DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(novel_from_row).collect())
    }

    pub async fn update_novel_metadata(
        &self,
        project_id: &str,
        novel_id: &str,
        input: NovelMetadataUpdateInput,
    ) -> StorageResult<Novel> {
        self.require_novel(project_id, novel_id).await?;

        let current = self
            .get_novel(project_id, novel_id)
            .await?
            .ok_or(StorageError::NotFound("novel"))?;
        let title = optional_trimmed(input.title).unwrap_or(current.title);
        if title.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "novel title is required".to_string(),
            ));
        }

        sqlx::query(
            "UPDATE novels
             SET title = ?,
                 author = ?,
                 source_language = ?,
                 genre = ?,
                 description = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(title)
        .bind(optional_trimmed(input.author))
        .bind(normalize_source_language(input.source_language, None))
        .bind(optional_trimmed(input.genre))
        .bind(optional_trimmed(input.description))
        .bind(project_id)
        .bind(novel_id)
        .execute(&self.pool)
        .await?;

        self.get_novel(project_id, novel_id)
            .await?
            .ok_or(StorageError::NotFound("novel"))
    }

    pub async fn list_chapters(
        &self,
        project_id: &str,
        novel_id: &str,
    ) -> StorageResult<Vec<Chapter>> {
        self.require_novel(project_id, novel_id).await?;

        let rows = sqlx::query(
            "SELECT id, novel_id, chapter_num, title, content, created_at, updated_at
             FROM chapters
             WHERE novel_id = ?
             ORDER BY chapter_num ASC",
        )
        .bind(novel_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(chapter_from_row).collect())
    }

    pub async fn get_latest_analysis_job_for_novel(
        &self,
        project_id: &str,
        novel_id: &str,
    ) -> StorageResult<Option<AnalysisJob>> {
        self.require_novel(project_id, novel_id).await?;

        let row = sqlx::query(
            "SELECT id, project_id, novel_id, job_type, status, payload_json,
                    started_at, finished_at, error_code, error_message, created_at, updated_at
             FROM analysis_jobs
             WHERE project_id = ? AND novel_id = ?
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(project_id)
        .bind(novel_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(analysis_job_from_row))
    }

    pub async fn create_translation_job(
        &self,
        project_id: &str,
        input: CreateTranslationJobInput,
    ) -> StorageResult<TranslationJob> {
        self.require_project(project_id).await?;
        let novel = self
            .get_novel(project_id, &input.novel_id)
            .await?
            .ok_or(StorageError::NotFound("novel"))?;
        let target_language = require_text(&input.target_language, "target language")?;
        let source_language = input
            .source_language
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or(novel.source_language);
        let provider = optional_trimmed(input.provider);
        let model = optional_trimmed(input.model);
        let translation_job_id = prefixed_id("tjob");
        let payload_json = json!({
            "novel_id": input.novel_id,
            "target_language": target_language,
        })
        .to_string();

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO translation_jobs (
                id, project_id, novel_id, source_language, target_language,
                provider, model, status, payload_json
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)",
        )
        .bind(&translation_job_id)
        .bind(project_id)
        .bind(&input.novel_id)
        .bind(source_language)
        .bind(&target_language)
        .bind(provider)
        .bind(model)
        .bind(payload_json)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO job_events (
                id, project_id, job_id, job_kind, sequence, event_type, payload_json
             )
             VALUES (?, ?, ?, 'translation', 1, 'translation_job_created', ?)",
        )
        .bind(prefixed_id("evt"))
        .bind(project_id)
        .bind(&translation_job_id)
        .bind(
            json!({
                "novel_id": input.novel_id,
                "target_language": target_language,
            })
            .to_string(),
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_translation_job(project_id, &translation_job_id)
            .await?
            .ok_or(StorageError::NotFound("translation_job"))
    }

    pub async fn list_job_events(
        &self,
        project_id: &str,
        job_id: &str,
    ) -> StorageResult<Vec<JobEvent>> {
        self.require_project(project_id).await?;

        let rows = sqlx::query(
            "SELECT id, project_id, job_id, job_kind, sequence, event_type, payload_json, created_at
             FROM job_events
             WHERE project_id = ? AND job_id = ?
             ORDER BY sequence ASC",
        )
        .bind(project_id)
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(job_event_from_row).collect())
    }

    pub async fn get_analysis_job(
        &self,
        project_id: &str,
        analysis_job_id: &str,
    ) -> StorageResult<Option<AnalysisJob>> {
        let row = sqlx::query(
            "SELECT id, project_id, novel_id, job_type, status, payload_json,
                    started_at, finished_at, error_code, error_message, created_at, updated_at
             FROM analysis_jobs
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(analysis_job_from_row))
    }

    pub async fn list_analysis_chapter_runs(
        &self,
        project_id: &str,
        analysis_job_id: &str,
    ) -> StorageResult<Vec<AnalysisChapterRun>> {
        self.require_project(project_id).await?;

        let rows = sqlx::query(
            "SELECT id, project_id, job_id, novel_id, chapter_id, chapter_num, status,
                    attempt, prompt_schema_version, output_json, error_code, error_message,
                    started_at, finished_at, created_at, updated_at
             FROM analysis_chapter_runs
             WHERE project_id = ? AND job_id = ?
             ORDER BY chapter_num ASC",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(analysis_chapter_run_from_row)
            .collect())
    }

    pub async fn list_story_extraction_records(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        group_key: &str,
    ) -> StorageResult<Vec<StoryExtractionRecordView>> {
        self.require_project(project_id).await?;

        let record_rows = sqlx::query(
            "SELECT id, project_id, novel_id, chapter_id, job_id, run_id, chapter_num,
                    group_key, group_label, entity_key, display_name, prompt_schema_version,
                    raw_record_json, created_at, updated_at
             FROM story_extraction_records
             WHERE project_id = ? AND job_id = ? AND group_key = ?
             ORDER BY chapter_num ASC, display_name ASC, id ASC",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(group_key)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::with_capacity(record_rows.len());
        for record_row in record_rows {
            let record_id: String = record_row.get("id");
            let raw_record_json: String = record_row.get("raw_record_json");
            let mentions = serde_json::from_str::<StoryExtractionRecordPayload>(&raw_record_json)
                .map(|record| record.mentions)
                .unwrap_or_default();
            let field_rows = sqlx::query(
                "SELECT id, record_id, field_key, field_label, created_at, updated_at
                 FROM story_extraction_fields
                 WHERE record_id = ?
                 ORDER BY field_label ASC, field_key ASC, id ASC",
            )
            .bind(&record_id)
            .fetch_all(&self.pool)
            .await?;

            let mut fields = Vec::with_capacity(field_rows.len());
            for field_row in field_rows {
                let field_id: String = field_row.get("id");
                let value_rows = sqlx::query(
                    "SELECT id, field_id, value_text, confidence,
                            related_character, relationship_type, relationship_label,
                            relationship_direction, evidence_json, created_at, updated_at
                     FROM story_extraction_values
                     WHERE field_id = ?
                     ORDER BY created_at ASC, id ASC",
                )
                .bind(&field_id)
                .fetch_all(&self.pool)
                .await?;

                let values = value_rows
                    .into_iter()
                    .map(story_extraction_value_from_row)
                    .collect::<StorageResult<Vec<_>>>()?;

                fields.push(StoryExtractionFieldView {
                    id: field_id,
                    record_id: field_row.get("record_id"),
                    field_key: field_row.get("field_key"),
                    field_label: field_row.get("field_label"),
                    values,
                    created_at: field_row.get("created_at"),
                    updated_at: field_row.get("updated_at"),
                });
            }

            records.push(StoryExtractionRecordView {
                id: record_id,
                project_id: record_row.get("project_id"),
                novel_id: record_row.get("novel_id"),
                chapter_id: record_row.get("chapter_id"),
                job_id: record_row.get("job_id"),
                run_id: record_row.get("run_id"),
                chapter_num: record_row.get("chapter_num"),
                group_key: record_row.get("group_key"),
                group_label: record_row.get("group_label"),
                entity_key: record_row.get("entity_key"),
                display_name: record_row.get("display_name"),
                prompt_schema_version: record_row.get("prompt_schema_version"),
                mentions,
                fields,
                created_at: record_row.get("created_at"),
                updated_at: record_row.get("updated_at"),
            });
        }

        Ok(records)
    }

    pub async fn list_story_character_aliases(
        &self,
        project_id: &str,
        analysis_job_id: &str,
    ) -> StorageResult<Vec<StoryCharacterAliasView>> {
        self.require_project(project_id).await?;

        let rows = sqlx::query(
            "SELECT id, project_id, novel_id, job_id, entity_key, display_name,
                    alias_text, alias_key, alias_type, alias_label, confidence,
                    first_chapter_num, evidence_json, created_at, updated_at
             FROM story_character_aliases
             WHERE project_id = ? AND job_id = ?
             ORDER BY display_name ASC, first_chapter_num ASC, alias_text ASC, id ASC",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(story_character_alias_from_row)
            .collect()
    }

    pub async fn reset_analysis_run(
        &self,
        project_id: &str,
        analysis_job_id: &str,
    ) -> StorageResult<AnalysisJob> {
        let current = self
            .get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))?;

        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM analysis_chapter_runs WHERE project_id = ? AND job_id = ?")
            .bind(project_id)
            .bind(analysis_job_id)
            .execute(&mut *tx)
            .await?;
        rebuild_story_character_aliases_tx(&mut tx, project_id, analysis_job_id).await?;

        sqlx::query(
            "UPDATE analysis_jobs
             SET status = 'pending',
                 started_at = NULL,
                 finished_at = NULL,
                 error_code = NULL,
                 error_message = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "analysis_run_reset",
            json!({
                "from_status": current.status,
                "mode": "force_rerun",
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))
    }

    pub async fn reset_analysis_run_range(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        from_chapter_num: i64,
        to_chapter_num: i64,
    ) -> StorageResult<AnalysisJob> {
        if from_chapter_num > to_chapter_num {
            return Err(StorageError::InvalidInput(
                "from chapter must be less than or equal to to chapter".to_string(),
            ));
        }

        let current = self
            .get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "DELETE FROM analysis_chapter_runs
             WHERE project_id = ?
               AND job_id = ?
               AND chapter_num >= ?
               AND chapter_num <= ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(from_chapter_num)
        .bind(to_chapter_num)
        .execute(&mut *tx)
        .await?;
        rebuild_story_character_aliases_tx(&mut tx, project_id, analysis_job_id).await?;

        sqlx::query(
            "UPDATE analysis_jobs
             SET status = 'pending',
                 started_at = NULL,
                 finished_at = NULL,
                 error_code = NULL,
                 error_message = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "analysis_run_range_reset",
            json!({
                "from_status": current.status,
                "from_chapter_num": from_chapter_num,
                "to_chapter_num": to_chapter_num,
                "mode": "force_rerun_range",
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))
    }

    pub async fn mark_analysis_job_running(
        &self,
        project_id: &str,
        analysis_job_id: &str,
    ) -> StorageResult<AnalysisJob> {
        let current = self
            .get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))?;
        let from_status = JobStatus::parse(&current.status)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        if from_status == JobStatus::Running {
            return Ok(current);
        }

        validate_transition(from_status, JobStatus::Running)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        let event_type = if from_status == JobStatus::Paused {
            "analysis_job_resumed"
        } else {
            "analysis_job_started"
        };

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE analysis_jobs
             SET status = 'running',
                 started_at = COALESCE(started_at, CURRENT_TIMESTAMP),
                 finished_at = NULL,
                 error_code = NULL,
                 error_message = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            event_type,
            json!({
                "from_status": from_status.as_str(),
                "to_status": "running",
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))
    }

    pub async fn pause_analysis_job(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        reason: &str,
        error_code: Option<&str>,
        automatic: bool,
    ) -> StorageResult<AnalysisJob> {
        let current = self
            .get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))?;
        let from_status = JobStatus::parse(&current.status)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        if from_status == JobStatus::Paused {
            return Ok(current);
        }

        validate_transition(from_status, JobStatus::Paused)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE analysis_jobs
             SET status = 'paused',
                 error_code = ?,
                 error_message = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(error_code)
        .bind(reason)
        .bind(project_id)
        .bind(analysis_job_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            if automatic {
                "analysis_job_auto_paused"
            } else {
                "analysis_job_paused"
            },
            json!({
                "from_status": from_status.as_str(),
                "to_status": "paused",
                "reason": reason,
                "error_code": error_code,
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))
    }

    pub async fn complete_analysis_job(
        &self,
        project_id: &str,
        analysis_job_id: &str,
    ) -> StorageResult<AnalysisJob> {
        let current = self
            .get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))?;
        let from_status = JobStatus::parse(&current.status)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        if from_status == JobStatus::Completed {
            return Ok(current);
        }

        validate_transition(from_status, JobStatus::Completed)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE analysis_jobs
             SET status = 'completed',
                 finished_at = CURRENT_TIMESTAMP,
                 error_code = NULL,
                 error_message = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "analysis_job_completed",
            json!({
                "from_status": from_status.as_str(),
                "to_status": "completed",
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))
    }

    pub async fn start_analysis_chapter_run(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        novel_id: &str,
        chapter: &Chapter,
    ) -> StorageResult<AnalysisChapterRun> {
        let run_id = prefixed_id("arun");
        let existing_attempt: Option<i64> = sqlx::query_scalar(
            "SELECT attempt
             FROM analysis_chapter_runs
             WHERE project_id = ? AND job_id = ? AND chapter_id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(&chapter.id)
        .fetch_optional(&self.pool)
        .await?;
        let next_attempt = existing_attempt.unwrap_or(0) + 1;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO analysis_chapter_runs (
                id, project_id, job_id, novel_id, chapter_id, chapter_num, status,
                attempt, output_json, error_code, error_message, started_at, finished_at
             )
             VALUES (?, ?, ?, ?, ?, ?, 'running', ?, NULL, NULL, NULL, CURRENT_TIMESTAMP, NULL)
             ON CONFLICT(job_id, chapter_id) DO UPDATE SET
                status = 'running',
                attempt = excluded.attempt,
                output_json = NULL,
                error_code = NULL,
                error_message = NULL,
                started_at = CURRENT_TIMESTAMP,
                finished_at = NULL,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&run_id)
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(novel_id)
        .bind(&chapter.id)
        .bind(chapter.chapter_num)
        .bind(next_attempt)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "analysis_chapter_started",
            json!({
                "chapter_id": chapter.id,
                "chapter_num": chapter.chapter_num,
                "title": chapter.title,
                "attempt": next_attempt,
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_chapter_run(project_id, analysis_job_id, &chapter.id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))
    }

    pub async fn complete_analysis_chapter_run(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        chapter_id: &str,
        prompt_schema_version: &str,
        output_json: &str,
    ) -> StorageResult<AnalysisChapterRun> {
        let current = self
            .get_analysis_chapter_run(project_id, analysis_job_id, chapter_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE analysis_chapter_runs
             SET status = 'completed',
                 prompt_schema_version = ?,
                 output_json = ?,
                 error_code = NULL,
                 error_message = NULL,
                 finished_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND job_id = ? AND chapter_id = ?",
        )
        .bind(prompt_schema_version)
        .bind(output_json)
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(chapter_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "analysis_chapter_completed",
            json!({
                "chapter_id": current.chapter_id,
                "chapter_num": current.chapter_num,
                "attempt": current.attempt,
                "prompt_schema_version": prompt_schema_version,
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_chapter_run(project_id, analysis_job_id, chapter_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))
    }

    pub async fn replace_story_extraction_records_for_chapter(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        chapter_id: &str,
        prompt_schema_version: &str,
        extraction: &StoryExtractionDocument,
        phase: &str,
    ) -> StorageResult<()> {
        let current = self
            .get_analysis_chapter_run(project_id, analysis_job_id, chapter_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))?;

        let mut tx = self.pool.begin().await?;
        let (
            character_record_count,
            character_mention_count,
            relationship_record_count,
            character_alias_count,
        ) = replace_story_extraction_records_tx(
            &mut tx,
            &current,
            project_id,
            analysis_job_id,
            chapter_id,
            prompt_schema_version,
            extraction,
        )
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "character_extraction_partial_persisted",
            json!({
                "chapter_id": &current.chapter_id,
                "chapter_num": current.chapter_num,
                "group_key": "character",
                "phase": phase,
                "record_count": character_record_count,
                "mention_count": character_mention_count,
                "relationship_record_count": relationship_record_count,
                "character_alias_count": character_alias_count,
                "prompt_schema_version": prompt_schema_version,
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        Ok(())
    }

    pub async fn complete_analysis_chapter_run_with_story_extraction(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        chapter_id: &str,
        prompt_schema_version: &str,
        output_json: &str,
        extraction: &StoryExtractionDocument,
    ) -> StorageResult<AnalysisChapterRun> {
        let current = self
            .get_analysis_chapter_run(project_id, analysis_job_id, chapter_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE analysis_chapter_runs
             SET status = 'completed',
                 prompt_schema_version = ?,
                 output_json = ?,
                 error_code = NULL,
                 error_message = NULL,
                 finished_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND job_id = ? AND chapter_id = ?",
        )
        .bind(prompt_schema_version)
        .bind(output_json)
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(chapter_id)
        .execute(&mut *tx)
        .await?;

        let (
            character_record_count,
            character_mention_count,
            relationship_record_count,
            character_alias_count,
        ) = replace_story_extraction_records_tx(
            &mut tx,
            &current,
            project_id,
            analysis_job_id,
            chapter_id,
            prompt_schema_version,
            extraction,
        )
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "character_extraction_records_persisted",
            json!({
                "chapter_id": &current.chapter_id,
                "chapter_num": current.chapter_num,
                "group_key": "character",
                "record_count": character_record_count,
                "mention_count": character_mention_count,
                "relationship_record_count": relationship_record_count,
                "character_alias_count": character_alias_count,
                "prompt_schema_version": prompt_schema_version,
            })
            .to_string(),
        )
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "analysis_chapter_completed",
            json!({
                "chapter_id": &current.chapter_id,
                "chapter_num": current.chapter_num,
                "attempt": current.attempt,
                "prompt_schema_version": prompt_schema_version,
                "character_record_count": character_record_count,
                "relationship_record_count": relationship_record_count,
                "character_alias_count": character_alias_count,
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_chapter_run(project_id, analysis_job_id, chapter_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))
    }

    pub async fn fail_analysis_chapter_run(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        chapter_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> StorageResult<AnalysisChapterRun> {
        let current = self
            .get_analysis_chapter_run(project_id, analysis_job_id, chapter_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE analysis_chapter_runs
             SET status = 'failed',
                 error_code = ?,
                 error_message = ?,
                 finished_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND job_id = ? AND chapter_id = ?",
        )
        .bind(error_code)
        .bind(error_message)
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(chapter_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            "analysis_chapter_failed",
            json!({
                "chapter_id": current.chapter_id,
                "chapter_num": current.chapter_num,
                "attempt": current.attempt,
                "error_code": error_code,
                "error_message": error_message,
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_chapter_run(project_id, analysis_job_id, chapter_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_chapter_run"))
    }

    pub async fn cancel_analysis_job(
        &self,
        project_id: &str,
        analysis_job_id: &str,
    ) -> StorageResult<AnalysisJob> {
        let current = self
            .get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))?;
        let from_status = JobStatus::parse(&current.status)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        validate_transition(from_status, JobStatus::Cancelled)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        if from_status == JobStatus::Cancelled {
            return Ok(current);
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE analysis_jobs
             SET status = 'cancelled',
                 finished_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            analysis_job_id,
            JobKind::Analysis,
            cancel_event_type(JobKind::Analysis),
            json!({
                "from_status": from_status.as_str(),
                "to_status": "cancelled",
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_analysis_job(project_id, analysis_job_id)
            .await?
            .ok_or(StorageError::NotFound("analysis_job"))
    }

    pub async fn get_translation_job(
        &self,
        project_id: &str,
        translation_job_id: &str,
    ) -> StorageResult<Option<TranslationJob>> {
        let row = sqlx::query(
            "SELECT id, project_id, novel_id, source_language, target_language,
                    provider, model, status, payload_json, started_at, finished_at,
                    error_code, error_message, created_at, updated_at
             FROM translation_jobs
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(translation_job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(translation_job_from_row))
    }

    pub async fn cancel_translation_job(
        &self,
        project_id: &str,
        translation_job_id: &str,
    ) -> StorageResult<TranslationJob> {
        let current = self
            .get_translation_job(project_id, translation_job_id)
            .await?
            .ok_or(StorageError::NotFound("translation_job"))?;
        let from_status = JobStatus::parse(&current.status)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        validate_transition(from_status, JobStatus::Cancelled)
            .map_err(|err| StorageError::InvalidJobTransition(err.to_string()))?;

        if from_status == JobStatus::Cancelled {
            return Ok(current);
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE translation_jobs
             SET status = 'cancelled',
                 finished_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE project_id = ? AND id = ?",
        )
        .bind(project_id)
        .bind(translation_job_id)
        .execute(&mut *tx)
        .await?;

        insert_job_event(
            &mut tx,
            project_id,
            translation_job_id,
            JobKind::Translation,
            cancel_event_type(JobKind::Translation),
            json!({
                "from_status": from_status.as_str(),
                "to_status": "cancelled",
            })
            .to_string(),
        )
        .await?;
        tx.commit().await?;

        self.get_translation_job(project_id, translation_job_id)
            .await?
            .ok_or(StorageError::NotFound("translation_job"))
    }

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
        .fetch_optional(&self.pool)
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
        .fetch_optional(&self.pool)
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
        .execute(&self.pool)
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn connect_with_options(
        options: SqliteConnectOptions,
        max_connections: u32,
    ) -> StorageResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;
        sqlx::migrate!("migrations/sqlite").run(&pool).await?;

        Ok(Self { pool })
    }

    async fn ensure_local_workspace(&self) -> StorageResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO users (id, email, display_name)
             VALUES (?, NULL, 'Local User')",
        )
        .bind(LOCAL_USER_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT OR IGNORE INTO workspaces (id, owner_user_id, name)
             VALUES (?, ?, 'Local Workspace')",
        )
        .bind(LOCAL_WORKSPACE_ID)
        .bind(LOCAL_USER_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT OR IGNORE INTO workspace_members (workspace_id, user_id, role)
             VALUES (?, ?, 'owner')",
        )
        .bind(LOCAL_WORKSPACE_ID)
        .bind(LOCAL_USER_ID)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn require_project(&self, project_id: &str) -> StorageResult<()> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(1) FROM projects WHERE id = ? AND deleted_at IS NULL")
                .bind(project_id)
                .fetch_one(&self.pool)
                .await?;

        if count == 0 {
            return Err(StorageError::NotFound("project"));
        }

        Ok(())
    }

    async fn require_novel(&self, project_id: &str, novel_id: &str) -> StorageResult<()> {
        if self.get_novel(project_id, novel_id).await?.is_none() {
            return Err(StorageError::NotFound("novel"));
        }

        Ok(())
    }

    async fn get_analysis_chapter_run(
        &self,
        project_id: &str,
        analysis_job_id: &str,
        chapter_id: &str,
    ) -> StorageResult<Option<AnalysisChapterRun>> {
        let row = sqlx::query(
            "SELECT id, project_id, job_id, novel_id, chapter_id, chapter_num, status,
                    attempt, prompt_schema_version, output_json, error_code, error_message,
                    started_at, finished_at, created_at, updated_at
             FROM analysis_chapter_runs
             WHERE project_id = ? AND job_id = ? AND chapter_id = ?",
        )
        .bind(project_id)
        .bind(analysis_job_id)
        .bind(chapter_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(analysis_chapter_run_from_row))
    }

    async fn get_project_including_deleted(
        &self,
        project_id: &str,
    ) -> StorageResult<Option<ProjectRow>> {
        let row = sqlx::query(
            "SELECT id, workspace_id, name, visibility, created_at, updated_at, deleted_at
             FROM projects
             WHERE id = ?",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(project_row_from_row))
    }
}

async fn replace_story_extraction_records_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    current: &AnalysisChapterRun,
    project_id: &str,
    analysis_job_id: &str,
    chapter_id: &str,
    prompt_schema_version: &str,
    extraction: &StoryExtractionDocument,
) -> StorageResult<(usize, usize, usize, usize)> {
    let character_record_count = extraction
        .records
        .iter()
        .filter(|record| record.group_key == "character")
        .count();
    let character_mention_count = extraction
        .records
        .iter()
        .filter(|record| record.group_key == "character")
        .map(|record| record.mentions.len())
        .sum::<usize>();
    let relationship_record_count = extraction
        .records
        .iter()
        .filter(|record| record.group_key == "relationship")
        .count();

    sqlx::query(
        "DELETE FROM story_extraction_records
         WHERE project_id = ? AND job_id = ? AND chapter_id = ?
           AND group_key IN ('character', 'relationship')",
    )
    .bind(project_id)
    .bind(analysis_job_id)
    .bind(chapter_id)
    .execute(&mut **tx)
    .await?;

    for record in extraction
        .records
        .iter()
        .filter(|record| matches!(record.group_key.as_str(), "character" | "relationship"))
    {
        let record_id = prefixed_id("srec");
        let raw_record_json = serde_json::to_string(record).map_err(|err| {
            StorageError::InvalidInput(format!("invalid extraction record: {err}"))
        })?;

        sqlx::query(
            "INSERT INTO story_extraction_records (
                id, project_id, novel_id, chapter_id, job_id, run_id, chapter_num,
                group_key, group_label, entity_key, display_name, prompt_schema_version,
                raw_record_json
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record_id)
        .bind(project_id)
        .bind(&current.novel_id)
        .bind(&current.chapter_id)
        .bind(analysis_job_id)
        .bind(&current.id)
        .bind(current.chapter_num)
        .bind(&record.group_key)
        .bind(&record.group_label)
        .bind(record.entity_key.as_deref())
        .bind(&record.display_name)
        .bind(prompt_schema_version)
        .bind(raw_record_json)
        .execute(&mut **tx)
        .await?;

        for field in &record.fields {
            let field_id = prefixed_id("sfld");
            sqlx::query(
                "INSERT INTO story_extraction_fields (
                    id, record_id, field_key, field_label
                 )
                 VALUES (?, ?, ?, ?)",
            )
            .bind(&field_id)
            .bind(&record_id)
            .bind(&field.field_key)
            .bind(&field.field_label)
            .execute(&mut **tx)
            .await?;

            for value in &field.values {
                let evidence_json = serde_json::to_string(&value.evidence).map_err(|err| {
                    StorageError::InvalidInput(format!("invalid extraction evidence: {err}"))
                })?;

                sqlx::query(
                    "INSERT INTO story_extraction_values (
                        id, field_id, value_text, confidence, related_character,
                        relationship_type, relationship_label, relationship_direction,
                        evidence_json
                     )
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(prefixed_id("sval"))
                .bind(&field_id)
                .bind(&value.value)
                .bind(value.confidence)
                .bind(value.related_character.as_deref())
                .bind(value.relationship_type.as_deref())
                .bind(value.relationship_label.as_deref())
                .bind(value.relationship_direction.as_deref())
                .bind(evidence_json)
                .execute(&mut **tx)
                .await?;
            }
        }
    }

    let character_alias_count =
        rebuild_story_character_aliases_tx(tx, project_id, analysis_job_id).await?;

    Ok((
        character_record_count,
        character_mention_count,
        relationship_record_count,
        character_alias_count,
    ))
}

#[derive(Debug, Clone)]
struct StoryCharacterAliasUpsert {
    project_id: String,
    novel_id: String,
    job_id: String,
    entity_key: String,
    display_name: String,
    alias_text: String,
    alias_key: String,
    alias_type: String,
    alias_label: String,
    confidence: Option<f64>,
    first_chapter_num: i64,
    evidence_json: String,
}

async fn rebuild_story_character_aliases_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    analysis_job_id: &str,
) -> StorageResult<usize> {
    sqlx::query(
        "DELETE FROM story_character_aliases
         WHERE project_id = ? AND job_id = ?",
    )
    .bind(project_id)
    .bind(analysis_job_id)
    .execute(&mut **tx)
    .await?;

    let record_rows = sqlx::query(
        "SELECT id, project_id, novel_id, job_id, chapter_num, entity_key, display_name
         FROM story_extraction_records
         WHERE project_id = ? AND job_id = ? AND group_key = 'character'
         ORDER BY chapter_num ASC, display_name ASC, id ASC",
    )
    .bind(project_id)
    .bind(analysis_job_id)
    .fetch_all(&mut **tx)
    .await?;

    let mut aliases = HashMap::<String, StoryCharacterAliasUpsert>::new();
    for record_row in record_rows {
        let record_id: String = record_row.get("id");
        let display_name: String = record_row.get("display_name");
        let entity_key = record_row
            .get::<Option<String>, _>("entity_key")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| normalized_story_alias_key(&display_name));
        if entity_key.trim().is_empty() {
            continue;
        }

        let project_id_value: String = record_row.get("project_id");
        let novel_id: String = record_row.get("novel_id");
        let job_id: String = record_row.get("job_id");
        let chapter_num: i64 = record_row.get("chapter_num");

        push_story_character_alias(
            &mut aliases,
            StoryCharacterAliasUpsert {
                project_id: project_id_value.clone(),
                novel_id: novel_id.clone(),
                job_id: job_id.clone(),
                entity_key: entity_key.clone(),
                display_name: display_name.clone(),
                alias_text: display_name.clone(),
                alias_key: normalized_story_alias_key(&display_name),
                alias_type: "canonical_name".to_string(),
                alias_label: "Tên chính".to_string(),
                confidence: Some(1.0),
                first_chapter_num: chapter_num,
                evidence_json: "[]".to_string(),
            },
        );

        let alias_rows = sqlx::query(
            "SELECT f.field_key, f.field_label, v.value_text, v.confidence, v.evidence_json
             FROM story_extraction_fields f
             JOIN story_extraction_values v ON v.field_id = f.id
             WHERE f.record_id = ?
             ORDER BY f.field_key ASC, v.created_at ASC, v.id ASC",
        )
        .bind(&record_id)
        .fetch_all(&mut **tx)
        .await?;

        for alias_row in alias_rows {
            let field_key: String = alias_row.get("field_key");
            if !is_story_character_alias_field_key(&field_key) {
                continue;
            }

            let alias_text: String = alias_row.get("value_text");
            let alias_key = normalized_story_alias_key(&alias_text);
            if alias_key.is_empty() || alias_key == normalized_story_alias_key(&display_name) {
                continue;
            }

            push_story_character_alias(
                &mut aliases,
                StoryCharacterAliasUpsert {
                    project_id: project_id_value.clone(),
                    novel_id: novel_id.clone(),
                    job_id: job_id.clone(),
                    entity_key: entity_key.clone(),
                    display_name: display_name.clone(),
                    alias_text,
                    alias_key,
                    alias_type: normalize_story_alias_type(&field_key),
                    alias_label: alias_row.get("field_label"),
                    confidence: alias_row.get("confidence"),
                    first_chapter_num: chapter_num,
                    evidence_json: alias_row.get("evidence_json"),
                },
            );
        }
    }

    let alias_count = aliases.len();
    for alias in aliases.into_values() {
        sqlx::query(
            "INSERT INTO story_character_aliases (
                id, project_id, novel_id, job_id, entity_key, display_name,
                alias_text, alias_key, alias_type, alias_label, confidence,
                first_chapter_num, evidence_json
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(prefixed_id("sali"))
        .bind(alias.project_id)
        .bind(alias.novel_id)
        .bind(alias.job_id)
        .bind(alias.entity_key)
        .bind(alias.display_name)
        .bind(alias.alias_text)
        .bind(alias.alias_key)
        .bind(alias.alias_type)
        .bind(alias.alias_label)
        .bind(alias.confidence)
        .bind(alias.first_chapter_num)
        .bind(alias.evidence_json)
        .execute(&mut **tx)
        .await?;
    }

    Ok(alias_count)
}

fn push_story_character_alias(
    aliases: &mut HashMap<String, StoryCharacterAliasUpsert>,
    alias: StoryCharacterAliasUpsert,
) {
    if alias.alias_key.is_empty() {
        return;
    }
    if alias.alias_type != "canonical_name"
        && (!is_story_alias_type_persistable(&alias.alias_type)
            || !is_stable_story_character_alias_surface(&alias.alias_text))
    {
        return;
    }

    let key = format!("{}:{}", alias.entity_key, alias.alias_key);
    if let Some(existing) = aliases.get_mut(&key) {
        if alias.first_chapter_num < existing.first_chapter_num {
            existing.first_chapter_num = alias.first_chapter_num;
            existing.alias_text = alias.alias_text;
        }
        if alias.confidence.unwrap_or(0.0) > existing.confidence.unwrap_or(0.0) {
            existing.confidence = alias.confidence;
        }
        if existing.evidence_json.trim() == "[]" && alias.evidence_json.trim() != "[]" {
            existing.evidence_json = alias.evidence_json;
        }
        return;
    }

    aliases.insert(key, alias);
}

fn is_stable_story_character_alias_surface(value: &str) -> bool {
    let surface = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if surface.is_empty() {
        return false;
    }

    let key = normalized_story_alias_key(&surface);
    if key.is_empty() {
        return false;
    }

    let tokens = surface.split_whitespace().collect::<Vec<_>>();
    let token_count = tokens.len();
    let char_count = surface.chars().filter(|ch| ch.is_alphanumeric()).count();
    let has_uppercase_token = tokens
        .iter()
        .any(|token| token.chars().next().is_some_and(char::is_uppercase));

    if token_count == 0 || char_count <= 3 {
        return false;
    }

    if !has_uppercase_token && token_count == 1 && char_count <= 4 {
        return false;
    }

    if !has_uppercase_token && token_count <= 2 && char_count <= 6 {
        return false;
    }

    if !has_uppercase_token && token_count > 3 {
        return false;
    }

    true
}

fn is_story_character_alias_field_key(field_key: &str) -> bool {
    matches!(
        normalize_story_alias_type(field_key).as_str(),
        "alias" | "aliases" | "other_alias" | "other_name" | "other_names" | "nickname"
    )
}

fn is_story_alias_type_persistable(alias_type: &str) -> bool {
    matches!(
        normalize_story_alias_type(alias_type).as_str(),
        "alias" | "aliases" | "other_alias" | "other_name" | "other_names" | "nickname"
    )
}

fn normalize_story_alias_type(field_key: &str) -> String {
    match normalized_story_alias_key(field_key).as_str() {
        "nickname" | "biet_danh" => "nickname".to_string(),
        "alias" | "aliases" => "alias".to_string(),
        "other_name" | "other_names" | "ten_goi_khac" | "ten_khac" => "other_name".to_string(),
        "other_alias" => "other_alias".to_string(),
        "pronoun"
        | "personal_pronoun"
        | "dai_tu"
        | "dai_tu_nhan_xung"
        | "temporary_reference"
        | "grammatical_reference"
        | "descriptive_phrase"
        | "event_phrase"
        | "group_reference"
        | "possessive_phrase"
        | "generic_reference"
        | "unstable_reference" => "unstable_reference".to_string(),
        value => value.to_string(),
    }
}

async fn insert_job_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    job_id: &str,
    job_kind: JobKind,
    event_type: &str,
    payload_json: String,
) -> StorageResult<()> {
    let sequence: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sequence), 0) + 1
         FROM job_events
         WHERE job_id = ?",
    )
    .bind(job_id)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO job_events (
            id, project_id, job_id, job_kind, sequence, event_type, payload_json
         )
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(prefixed_id("evt"))
    .bind(project_id)
    .bind(job_id)
    .bind(job_kind.as_str())
    .bind(sequence)
    .bind(event_type)
    .bind(payload_json)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn require_text(value: &str, field: &str) -> StorageResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(StorageError::InvalidInput(format!("{field} is required")));
    }

    Ok(value.to_string())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_source_language(value: Option<String>, text: Option<&str>) -> Option<String> {
    let trimmed = optional_trimmed(value);
    match trimmed.as_deref() {
        Some("auto") | Some("detect") => text.and_then(detect_basic_source_language),
        Some(value) => Some(value.to_string()),
        None => text.and_then(detect_basic_source_language),
    }
}

fn prefixed_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7().simple())
}

fn project_from_row(row: SqliteRow) -> Project {
    Project {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        name: row.get("name"),
        visibility: row.get("visibility"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[derive(Debug)]
struct ProjectRow {
    deleted_at: Option<String>,
}

fn project_row_from_row(row: SqliteRow) -> ProjectRow {
    ProjectRow {
        deleted_at: row.get("deleted_at"),
    }
}

fn novel_from_row(row: SqliteRow) -> Novel {
    Novel {
        id: row.get("id"),
        project_id: row.get("project_id"),
        title: row.get("title"),
        author: row.get("author"),
        source_language: row.get("source_language"),
        genre: row.get("genre"),
        description: row.get("description"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn chapter_from_row(row: SqliteRow) -> Chapter {
    Chapter {
        id: row.get("id"),
        novel_id: row.get("novel_id"),
        chapter_num: row.get("chapter_num"),
        title: row.get("title"),
        content: row.get("content"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn analysis_job_from_row(row: SqliteRow) -> AnalysisJob {
    AnalysisJob {
        id: row.get("id"),
        project_id: row.get("project_id"),
        novel_id: row.get("novel_id"),
        job_type: row.get("job_type"),
        status: row.get("status"),
        payload_json: row.get("payload_json"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        error_code: row.get("error_code"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn analysis_chapter_run_from_row(row: SqliteRow) -> AnalysisChapterRun {
    AnalysisChapterRun {
        id: row.get("id"),
        project_id: row.get("project_id"),
        job_id: row.get("job_id"),
        novel_id: row.get("novel_id"),
        chapter_id: row.get("chapter_id"),
        chapter_num: row.get("chapter_num"),
        status: row.get("status"),
        attempt: row.get("attempt"),
        prompt_schema_version: row.get("prompt_schema_version"),
        output_json: row.get("output_json"),
        error_code: row.get("error_code"),
        error_message: row.get("error_message"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn translation_job_from_row(row: SqliteRow) -> TranslationJob {
    TranslationJob {
        id: row.get("id"),
        project_id: row.get("project_id"),
        novel_id: row.get("novel_id"),
        source_language: row.get("source_language"),
        target_language: row.get("target_language"),
        provider: row.get("provider"),
        model: row.get("model"),
        status: row.get("status"),
        payload_json: row.get("payload_json"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        error_code: row.get("error_code"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
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

fn job_event_from_row(row: SqliteRow) -> JobEvent {
    JobEvent {
        id: row.get("id"),
        project_id: row.get("project_id"),
        job_id: row.get("job_id"),
        job_kind: row.get("job_kind"),
        sequence: row.get("sequence"),
        event_type: row.get("event_type"),
        payload_json: row.get("payload_json"),
        created_at: row.get("created_at"),
    }
}

fn story_extraction_value_from_row(row: SqliteRow) -> StorageResult<StoryExtractionFieldValueView> {
    let evidence_json: String = row.get("evidence_json");
    let evidence =
        serde_json::from_str::<Vec<StoryEvidenceSpan>>(&evidence_json).map_err(|err| {
            StorageError::InvalidInput(format!("invalid stored extraction evidence: {err}"))
        })?;

    Ok(StoryExtractionFieldValueView {
        id: row.get("id"),
        field_id: row.get("field_id"),
        value: row.get("value_text"),
        confidence: row.get("confidence"),
        related_character: row.get("related_character"),
        relationship_type: row.get("relationship_type"),
        relationship_label: row.get("relationship_label"),
        relationship_direction: row.get("relationship_direction"),
        evidence,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn story_character_alias_from_row(row: SqliteRow) -> StorageResult<StoryCharacterAliasView> {
    let evidence_json: String = row.get("evidence_json");
    let evidence =
        serde_json::from_str::<Vec<StoryEvidenceSpan>>(&evidence_json).map_err(|err| {
            StorageError::InvalidInput(format!("invalid stored character alias evidence: {err}"))
        })?;

    Ok(StoryCharacterAliasView {
        id: row.get("id"),
        project_id: row.get("project_id"),
        novel_id: row.get("novel_id"),
        job_id: row.get("job_id"),
        entity_key: row.get("entity_key"),
        display_name: row.get("display_name"),
        alias_text: row.get("alias_text"),
        alias_key: row.get("alias_key"),
        alias_type: row.get("alias_type"),
        alias_label: row.get("alias_label"),
        confidence: row.get("confidence"),
        first_chapter_num: row.get("first_chapter_num"),
        evidence,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn normalized_story_alias_key(value: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = true;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            last_was_separator = false;
        } else if let Some(ascii) = fold_story_alias_char(ch) {
            normalized.push(ascii);
            last_was_separator = false;
        } else if ch.is_alphanumeric() {
            normalized.push(ch);
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }

    while normalized.ends_with('_') {
        normalized.pop();
    }

    normalized
}

fn fold_story_alias_char(ch: char) -> Option<char> {
    match ch {
        'a'..='z' | '0'..='9' => Some(ch),
        'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ' | 'ấ' | 'ẩ'
        | 'ẫ' | 'ậ' => Some('a'),
        'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => {
            Some('e')
        }
        'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => Some('i'),
        'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ' | 'ở'
        | 'ỡ' | 'ợ' => Some('o'),
        'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => {
            Some('u')
        }
        'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => Some('y'),
        'đ' => Some('d'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use novelgraph_core::{CreateTranslationJobInput, NovelImportInput};

    use super::SqliteStore;

    #[tokio::test]
    async fn imports_novel_and_creates_translation_job() {
        let store = SqliteStore::connect_in_memory().await.unwrap();
        let project = store.create_project("Demo Project").await.unwrap();
        let projects = store.list_projects().await.unwrap();
        let fetched_project = store.get_project(&project.id).await.unwrap().unwrap();

        assert_eq!(projects.len(), 1);
        assert_eq!(fetched_project.name, "Demo Project");

        let import = store
            .import_novel(
                &project.id,
                NovelImportInput {
                    title: "Truyện Thử".to_string(),
                    author: Some("Tác giả".to_string()),
                    source_language: Some("zh".to_string()),
                    genre: None,
                    description: None,
                    text: "Chương 1\nMở đầu.\n\nChương 2\nTiếp tục.".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(import.chapters.len(), 2);
        assert_eq!(import.source_segment_count, 2);
        assert_eq!(import.analysis_job.status, "pending");
        let novels = store.list_novels(&project.id).await.unwrap();
        assert_eq!(novels.len(), 1);
        assert_eq!(novels[0].title, "Truyện Thử");
        let latest_analysis_job = store
            .get_latest_analysis_job_for_novel(&project.id, &import.novel.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest_analysis_job.id, import.analysis_job.id);
        let analysis_events = store
            .list_job_events(&project.id, &import.analysis_job.id)
            .await
            .unwrap();
        assert_eq!(analysis_events.len(), 1);
        assert_eq!(analysis_events[0].event_type, "analysis_job_created");
        let running_analysis_job = store
            .mark_analysis_job_running(&project.id, &import.analysis_job.id)
            .await
            .unwrap();
        assert_eq!(running_analysis_job.status, "running");
        let chapters = store
            .list_chapters(&project.id, &import.novel.id)
            .await
            .unwrap();
        let started_chapter_run = store
            .start_analysis_chapter_run(
                &project.id,
                &import.analysis_job.id,
                &import.novel.id,
                &chapters[0],
            )
            .await
            .unwrap();
        assert_eq!(started_chapter_run.status, "running");
        assert_eq!(started_chapter_run.attempt, 1);
        let completed_chapter_run = store
            .complete_analysis_chapter_run(
                &project.id,
                &import.analysis_job.id,
                &chapters[0].id,
                "draft.chapter_extraction.v0",
                "{\"persisted\":false}",
            )
            .await
            .unwrap();
        assert_eq!(completed_chapter_run.status, "completed");
        assert!(completed_chapter_run.output_json.is_some());
        let paused_analysis_job = store
            .pause_analysis_job(
                &project.id,
                &import.analysis_job.id,
                "local LLM unavailable",
                Some("local_llm_unreachable"),
                true,
            )
            .await
            .unwrap();
        assert_eq!(paused_analysis_job.status, "paused");
        let resumed_analysis_job = store
            .mark_analysis_job_running(&project.id, &import.analysis_job.id)
            .await
            .unwrap();
        assert_eq!(resumed_analysis_job.status, "running");
        let analysis_chapter_runs = store
            .list_analysis_chapter_runs(&project.id, &import.analysis_job.id)
            .await
            .unwrap();
        assert_eq!(analysis_chapter_runs.len(), 1);
        let reset_analysis_job = store
            .reset_analysis_run(&project.id, &import.analysis_job.id)
            .await
            .unwrap();
        assert_eq!(reset_analysis_job.status, "pending");
        assert!(store
            .list_analysis_chapter_runs(&project.id, &import.analysis_job.id)
            .await
            .unwrap()
            .is_empty());
        let cancelled_analysis_job = store
            .cancel_analysis_job(&project.id, &import.analysis_job.id)
            .await
            .unwrap();
        assert_eq!(cancelled_analysis_job.status, "cancelled");
        assert!(cancelled_analysis_job.finished_at.is_some());

        let translation_job = store
            .create_translation_job(
                &project.id,
                CreateTranslationJobInput {
                    novel_id: import.novel.id.clone(),
                    source_language: None,
                    target_language: "vi".to_string(),
                    provider: Some("openai".to_string()),
                    model: Some("gpt-test".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(translation_job.status, "pending");
        assert_eq!(translation_job.source_language.as_deref(), Some("zh"));
        assert_eq!(translation_job.target_language, "vi");

        let translation_events = store
            .list_job_events(&project.id, &translation_job.id)
            .await
            .unwrap();
        assert_eq!(translation_events.len(), 1);
        assert_eq!(translation_events[0].event_type, "translation_job_created");
        let cancelled_translation_job = store
            .cancel_translation_job(&project.id, &translation_job.id)
            .await
            .unwrap();
        assert_eq!(cancelled_translation_job.status, "cancelled");

        let translation_events = store
            .list_job_events(&project.id, &translation_job.id)
            .await
            .unwrap();
        assert_eq!(translation_events.len(), 2);
        assert_eq!(
            translation_events[1].event_type,
            "translation_job_cancelled"
        );

        let archived_project = store.delete_project(&project.id, false).await.unwrap();
        assert_eq!(archived_project.action, "archived");
        assert!(archived_project.data_retained);
        assert!(store.get_project(&project.id).await.unwrap().is_none());
        let archived_projects = store.list_archived_projects().await.unwrap();
        assert_eq!(archived_projects.len(), 1);
        assert_eq!(archived_projects[0].id, project.id);

        let retained_novel = store
            .get_novel(&project.id, &import.novel.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retained_novel.id, import.novel.id);
        let restored_project = store.restore_project(&project.id).await.unwrap();
        assert_eq!(restored_project.id, project.id);
        assert!(store.list_archived_projects().await.unwrap().is_empty());
        let re_archived_project = store.delete_project(&project.id, false).await.unwrap();
        assert_eq!(re_archived_project.action, "archived");
        let purged_archived_project = store.delete_project(&project.id, true).await.unwrap();
        assert_eq!(purged_archived_project.action, "purged");
        assert!(store.get_project(&project.id).await.unwrap().is_none());

        let hard_delete_project = store.create_project("Hard Delete Project").await.unwrap();
        let second_import = store
            .import_novel(
                &hard_delete_project.id,
                NovelImportInput {
                    title: "Truyện Xóa".to_string(),
                    author: None,
                    source_language: Some("vi".to_string()),
                    genre: None,
                    description: None,
                    text: "Chương 1\nDữ liệu xóa.".to_string(),
                },
            )
            .await
            .unwrap();
        let purged_project = store
            .delete_project(&hard_delete_project.id, true)
            .await
            .unwrap();
        assert_eq!(purged_project.action, "purged");
        assert!(!purged_project.data_retained);
        assert!(store
            .get_project(&hard_delete_project.id)
            .await
            .unwrap()
            .is_none());
        assert!(store
            .get_novel(&hard_delete_project.id, &second_import.novel.id)
            .await
            .unwrap()
            .is_none());
    }
}
