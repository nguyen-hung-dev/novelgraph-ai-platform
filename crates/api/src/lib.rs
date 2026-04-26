use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
pub mod local_runtime;
use local_runtime::{LocalLlmRuntimeManager, LocalRuntimeError};
use novelgraph_ai::{
    AiError, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, LlamaCppClient, LlmRole,
    LocalLlmHealth, ModelListResponse,
};
use novelgraph_core::{
    build_draft_extraction_prompt, build_import_preview, ActivateManagedLocalModelInput, AppConfig,
    CreateProjectInput, CreateTranslationJobInput, DeleteProjectInput, DeleteProjectResult,
    DraftExtractionInput, DraftExtractionPrompt, LocalLlmRuntimeSnapshot, NovelImportInput,
    ProjectWorkspaceSnapshot, API_VERSION, APP_VERSION, STORAGE_SCHEMA_VERSION,
};
use novelgraph_storage::{SqliteStore, StorageError};
use serde::Serialize;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: SqliteStore,
    pub local_llm: LlamaCppClient,
    pub local_runtime: LocalLlmRuntimeManager,
}

pub fn build_router(
    config: AppConfig,
    store: SqliteStore,
    local_llm: LlamaCppClient,
    local_runtime: LocalLlmRuntimeManager,
) -> Router {
    let state = AppState {
        config,
        store,
        local_llm,
        local_runtime,
    };

    Router::new()
        .route("/health", get(health))
        .route("/api/local-llm/health", get(local_llm_health))
        .route("/api/local-llm/models", get(local_llm_models))
        .route("/api/local-llm/runtime", get(local_llm_runtime))
        .route(
            "/api/local-llm/runtime/select-existing",
            post(local_llm_select_existing_model),
        )
        .route(
            "/api/local-llm/runtime/start-selected",
            post(local_llm_start_selected_model),
        )
        .route("/api/local-llm/runtime/stop", post(local_llm_stop_server))
        .route(
            "/api/local-llm/runtime/models/activate",
            post(local_llm_activate_managed_model),
        )
        .route(
            "/api/local-llm/runtime/presets/{preset_id}/download",
            post(local_llm_download_preset),
        )
        .route(
            "/api/local-llm/chat/completions",
            post(local_llm_chat_completion),
        )
        .route(
            "/api/local-llm/extraction/draft-chapter",
            post(local_llm_draft_chapter_extraction),
        )
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/archived", get(list_archived_projects))
        .route(
            "/api/projects/{project_id}",
            get(get_project).post(delete_project),
        )
        .route("/api/projects/{project_id}/restore", post(restore_project))
        .route(
            "/api/projects/{project_id}/workspace",
            get(get_project_workspace),
        )
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
            "/api/projects/{project_id}/analysis/jobs/{job_id}",
            get(get_analysis_job),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/cancel",
            post(cancel_analysis_job),
        )
        .route(
            "/api/projects/{project_id}/translation/jobs/{job_id}",
            get(get_translation_job),
        )
        .route(
            "/api/projects/{project_id}/translation/jobs/{job_id}/cancel",
            post(cancel_translation_job),
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
    pub api_version: &'static str,
    pub storage_schema_version: &'static str,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app_mode: state.config.mode.as_str(),
        version: APP_VERSION,
        api_version: API_VERSION,
        storage_schema_version: STORAGE_SCHEMA_VERSION,
    })
}

async fn local_llm_health(State(state): State<AppState>) -> Result<Json<LocalLlmHealth>, ApiError> {
    Ok(Json(state.local_llm.health().await?))
}

async fn local_llm_models(
    State(state): State<AppState>,
) -> Result<Json<ModelListResponse>, ApiError> {
    Ok(Json(state.local_llm.list_models().await?))
}

async fn local_llm_runtime(
    State(state): State<AppState>,
) -> Result<Json<LocalLlmRuntimeSnapshot>, ApiError> {
    Ok(Json(state.local_runtime.snapshot().await))
}

async fn local_llm_select_existing_model(
    State(state): State<AppState>,
) -> Result<Json<LocalLlmRuntimeSnapshot>, ApiError> {
    Ok(Json(state.local_runtime.pick_existing_model().await?))
}

async fn local_llm_start_selected_model(
    State(state): State<AppState>,
) -> Result<Json<LocalLlmRuntimeSnapshot>, ApiError> {
    Ok(Json(state.local_runtime.start_selected_model().await?))
}

async fn local_llm_stop_server(
    State(state): State<AppState>,
) -> Result<Json<LocalLlmRuntimeSnapshot>, ApiError> {
    Ok(Json(state.local_runtime.stop_server().await?))
}

async fn local_llm_activate_managed_model(
    State(state): State<AppState>,
    Json(input): Json<ActivateManagedLocalModelInput>,
) -> Result<Json<LocalLlmRuntimeSnapshot>, ApiError> {
    Ok(Json(
        state
            .local_runtime
            .activate_managed_model(&input.path)
            .await?,
    ))
}

async fn local_llm_download_preset(
    State(state): State<AppState>,
    Path(preset_id): Path<String>,
) -> Result<Json<LocalLlmRuntimeSnapshot>, ApiError> {
    Ok(Json(
        state
            .local_runtime
            .download_or_activate_preset(&preset_id)
            .await?,
    ))
}

async fn local_llm_chat_completion(
    State(state): State<AppState>,
    Json(input): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, ApiError> {
    Ok(Json(state.local_llm.chat_completion(input).await?))
}

async fn local_llm_draft_chapter_extraction(
    State(state): State<AppState>,
    Json(input): Json<DraftExtractionInput>,
) -> Result<Json<DraftExtractionResponse>, ApiError> {
    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("chapter text is required"));
    }

    let prompt = build_draft_extraction_prompt(&input);
    let llm_response = state
        .local_llm
        .chat_completion(ChatCompletionRequest {
            model: None,
            messages: vec![
                ChatMessage {
                    role: LlmRole::System,
                    content: prompt.system_prompt.clone(),
                },
                ChatMessage {
                    role: LlmRole::User,
                    content: prompt.user_prompt.clone(),
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(4096),
            stream: false,
        })
        .await?;

    Ok(Json(DraftExtractionResponse {
        schema_version: prompt.schema_version,
        prompt,
        llm_response,
        persisted: false,
    }))
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

async fn list_archived_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<novelgraph_core::Project>>, ApiError> {
    Ok(Json(state.store.list_archived_projects().await?))
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

async fn delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<DeleteProjectInput>,
) -> Result<Json<DeleteProjectResult>, ApiError> {
    Ok(Json(
        state
            .store
            .delete_project(&project_id, input.purge_data)
            .await?,
    ))
}

async fn restore_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<novelgraph_core::Project>, ApiError> {
    Ok(Json(state.store.restore_project(&project_id).await?))
}

async fn get_project_workspace(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectWorkspaceSnapshot>, ApiError> {
    let project = state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;
    let novels = state.store.list_novels(&project_id).await?;
    let active_novel = novels.first().cloned();
    let chapters = match &active_novel {
        Some(novel) => state.store.list_chapters(&project_id, &novel.id).await?,
        None => Vec::new(),
    };
    let latest_analysis_job = match &active_novel {
        Some(novel) => {
            state
                .store
                .get_latest_analysis_job_for_novel(&project_id, &novel.id)
                .await?
        }
        None => None,
    };
    let latest_job_events = match &latest_analysis_job {
        Some(job) => state.store.list_job_events(&project_id, &job.id).await?,
        None => Vec::new(),
    };

    Ok(Json(ProjectWorkspaceSnapshot {
        project,
        novels,
        active_novel,
        chapters,
        latest_analysis_job,
        latest_job_events,
    }))
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

async fn get_analysis_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<novelgraph_core::AnalysisJob>, ApiError> {
    let job = state
        .store
        .get_analysis_job(&project_id, &job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))?;

    Ok(Json(job))
}

async fn cancel_analysis_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<novelgraph_core::AnalysisJob>, ApiError> {
    Ok(Json(
        state
            .store
            .cancel_analysis_job(&project_id, &job_id)
            .await?,
    ))
}

async fn get_translation_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<novelgraph_core::TranslationJob>, ApiError> {
    let job = state
        .store
        .get_translation_job(&project_id, &job_id)
        .await?
        .ok_or(ApiError::not_found("translation_job"))?;

    Ok(Json(job))
}

async fn cancel_translation_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<novelgraph_core::TranslationJob>, ApiError> {
    Ok(Json(
        state
            .store
            .cancel_translation_job(&project_id, &job_id)
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

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "invalid_job_transition",
            message: message.into(),
        }
    }
}

impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::InvalidInput(message) => Self::bad_request(message),
            StorageError::InvalidJobTransition(message) => Self::conflict(message),
            StorageError::NotFound(resource) => Self::not_found(resource),
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "storage_error",
                message: "storage operation failed".to_string(),
            },
        }
    }
}

impl From<AiError> for ApiError {
    fn from(error: AiError) -> Self {
        match error {
            AiError::InvalidRequest(message) => Self::bad_request(message),
            AiError::InvalidConfig(message) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "local_llm_config_error",
                message,
            },
            AiError::Request(_) => Self {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "local_llm_unreachable",
                message: "local LLM server is unreachable".to_string(),
            },
            AiError::HttpStatus { status, message } => Self {
                status: StatusCode::BAD_GATEWAY,
                code: "local_llm_http_error",
                message: format!("local LLM returned HTTP {status}: {message}"),
            },
        }
    }
}

impl From<LocalRuntimeError> for ApiError {
    fn from(error: LocalRuntimeError) -> Self {
        match error {
            LocalRuntimeError::SelectionCancelled
            | LocalRuntimeError::UnknownPreset(_)
            | LocalRuntimeError::MissingModel(_)
            | LocalRuntimeError::ManagedModelOutsideRepo => Self::bad_request(error.to_string()),
            LocalRuntimeError::DownloadAlreadyRunning => Self {
                status: StatusCode::CONFLICT,
                code: "local_llm_download_busy",
                message: error.to_string(),
            },
            LocalRuntimeError::InvalidBaseUrl(_) | LocalRuntimeError::StartFailed(_) => Self {
                status: StatusCode::FAILED_DEPENDENCY,
                code: "local_llm_runtime_unavailable",
                message: error.to_string(),
            },
            LocalRuntimeError::Io(_)
            | LocalRuntimeError::Request(_)
            | LocalRuntimeError::Serde(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "local_llm_runtime_error",
                message: error.to_string(),
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

#[derive(Debug, Serialize)]
pub struct DraftExtractionResponse {
    pub schema_version: &'static str,
    pub prompt: DraftExtractionPrompt,
    pub llm_response: ChatCompletionResponse,
    pub persisted: bool,
}
