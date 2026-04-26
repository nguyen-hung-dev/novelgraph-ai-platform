use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use novelgraph_core::{
    build_import_preview, AppConfig, CreateProjectInput, CreateTranslationJobInput,
    NovelImportInput,
};
use novelgraph_storage::{SqliteStore, StorageError};
use serde::Serialize;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: SqliteStore,
}

pub fn build_router(config: AppConfig, store: SqliteStore) -> Router {
    let state = AppState { config, store };

    Router::new()
        .route("/health", get(health))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{project_id}", get(get_project))
        .route(
            "/api/projects/{project_id}/novels/import/preview",
            post(preview_novel_import),
        )
        .route(
            "/api/projects/{project_id}/novels/import/confirm",
            post(confirm_novel_import),
        )
        .route(
            "/api/projects/{project_id}/novels/{novel_id}",
            get(get_novel),
        )
        .route(
            "/api/projects/{project_id}/novels/{novel_id}/chapters",
            get(list_chapters),
        )
        .route(
            "/api/projects/{project_id}/translation/jobs",
            post(create_translation_job),
        )
        .route(
            "/api/projects/{project_id}/jobs/{job_id}/events",
            get(list_job_events),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub app_mode: &'static str,
    pub version: &'static str,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app_mode: state.config.mode.as_str(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<novelgraph_core::Project>>, ApiError> {
    Ok(Json(state.store.list_projects().await?))
}

async fn create_project(
    State(state): State<AppState>,
    Json(input): Json<CreateProjectInput>,
) -> Result<Json<novelgraph_core::Project>, ApiError> {
    Ok(Json(state.store.create_project(&input.name).await?))
}

async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<novelgraph_core::Project>, ApiError> {
    let project = state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;

    Ok(Json(project))
}

async fn preview_novel_import(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<NovelImportInput>,
) -> Result<Json<novelgraph_core::ImportPreview>, ApiError> {
    state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;
    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("novel text is required"));
    }

    Ok(Json(build_import_preview(&input)))
}

async fn confirm_novel_import(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<NovelImportInput>,
) -> Result<Json<novelgraph_core::NovelImportResult>, ApiError> {
    Ok(Json(state.store.import_novel(&project_id, input).await?))
}

async fn get_novel(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
) -> Result<Json<novelgraph_core::Novel>, ApiError> {
    let novel = state
        .store
        .get_novel(&project_id, &novel_id)
        .await?
        .ok_or(ApiError::not_found("novel"))?;

    Ok(Json(novel))
}

async fn list_chapters(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
) -> Result<Json<Vec<novelgraph_core::Chapter>>, ApiError> {
    Ok(Json(
        state.store.list_chapters(&project_id, &novel_id).await?,
    ))
}

async fn create_translation_job(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<CreateTranslationJobInput>,
) -> Result<Json<novelgraph_core::TranslationJob>, ApiError> {
    Ok(Json(
        state
            .store
            .create_translation_job(&project_id, input)
            .await?,
    ))
}

async fn list_job_events(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<Vec<novelgraph_core::JobEvent>>, ApiError> {
    Ok(Json(
        state.store.list_job_events(&project_id, &job_id).await?,
    ))
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
        }
    }

    fn not_found(resource: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: format!("{resource} was not found"),
        }
    }
}

impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::InvalidInput(message) => Self::bad_request(message),
            StorageError::NotFound(resource) => Self::not_found(resource),
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "storage_error",
                message: "storage operation failed".to_string(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
            },
        };

        (self.status, Json(body)).into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}
