use std::{path::Path, str::FromStr};

use novelgraph_core::{
    detect_basic_source_language, split_chapters, split_source_segments, AnalysisChapterRun,
    AnalysisJob, Chapter, CreateTranslationJobInput, DeleteProjectResult, JobEvent, Novel,
    NovelImportInput, NovelImportResult, NovelMetadataUpdateInput, Project,
    StoryCharacterAliasView, StoryEvidenceSpan, StoryExtractionFieldValueView, TranslationJob,
};
use novelgraph_jobs::{cancel_event_type, validate_transition, JobKind, JobStatus};
use serde_json::json;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};
use uuid::Uuid;

use crate::{StorageError, StorageResult};

pub(crate) const LOCAL_USER_ID: &str = "user_local";
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

    pub(crate) async fn ensure_local_workspace(&self) -> StorageResult<()> {
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

    pub(crate) async fn require_project(&self, project_id: &str) -> StorageResult<()> {
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

    pub(crate) async fn require_novel(
        &self,
        project_id: &str,
        novel_id: &str,
    ) -> StorageResult<()> {
        if self.get_novel(project_id, novel_id).await?.is_none() {
            return Err(StorageError::NotFound("novel"));
        }

        Ok(())
    }

    pub(crate) async fn get_analysis_chapter_run(
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

pub(crate) async fn insert_job_event(
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

pub(crate) fn require_text(value: &str, field: &str) -> StorageResult<String> {
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

pub(crate) fn prefixed_id(prefix: &str) -> String {
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

pub(crate) fn analysis_job_from_row(row: SqliteRow) -> AnalysisJob {
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

pub(crate) fn analysis_chapter_run_from_row(row: SqliteRow) -> AnalysisChapterRun {
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

pub(crate) fn story_extraction_value_from_row(
    row: SqliteRow,
) -> StorageResult<StoryExtractionFieldValueView> {
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

pub(crate) fn story_character_alias_from_row(
    row: SqliteRow,
) -> StorageResult<StoryCharacterAliasView> {
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

pub(crate) fn normalized_story_alias_key(value: &str) -> String {
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
