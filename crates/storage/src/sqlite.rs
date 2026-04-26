use std::{path::Path, str::FromStr};

use novelgraph_core::{
    split_chapters, split_source_segments, AnalysisJob, Chapter, CreateTranslationJobInput,
    DeleteProjectResult, JobEvent, Novel, NovelImportInput, NovelImportResult, Project,
    TranslationJob,
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

        sqlx::query(
            "INSERT INTO novels (id, project_id, title, author, source_language)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&novel_id)
        .bind(project_id)
        .bind(title)
        .bind(
            input
                .author
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            input
                .source_language
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
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
            "SELECT id, project_id, title, author, source_language, created_at, updated_at
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
            "SELECT id, project_id, title, author, source_language, created_at, updated_at
             FROM novels
             WHERE project_id = ?
             ORDER BY updated_at DESC, created_at DESC, id DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(novel_from_row).collect())
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
