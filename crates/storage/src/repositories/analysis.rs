use novelgraph_core::{
    AnalysisChapterRun, AnalysisJob, Chapter, StoryCharacterAliasView, StoryExtractionDocument,
    StoryExtractionFieldView, StoryExtractionRecordPayload, StoryExtractionRecordView,
};
use novelgraph_jobs::{cancel_event_type, validate_transition, JobKind, JobStatus};
use serde_json::json;
use sqlx::Row;

use crate::sqlite::*;
use crate::{StorageError, StorageResult};

use super::story_aliases::rebuild_story_character_aliases_tx;

impl SqliteStore {
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
        .fetch_optional(self.pool())
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
        .fetch_all(self.pool())
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
        .fetch_all(self.pool())
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
            .fetch_all(self.pool())
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
                .fetch_all(self.pool())
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
        .fetch_all(self.pool())
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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
        .fetch_optional(self.pool())
        .await?;
        let next_attempt = existing_attempt.unwrap_or(0) + 1;

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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

        let mut tx = self.pool().begin().await?;
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
