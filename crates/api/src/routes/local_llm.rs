use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use novelgraph_ai::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, LlmRole, LocalLlmHealth,
    ModelListResponse,
};
use novelgraph_core::{
    build_draft_extraction_prompt, ActivateManagedLocalModelInput, DraftExtractionInput,
    DraftExtractionPrompt, LocalLlmRuntimeSnapshot,
};
use serde::Serialize;

use crate::{ApiError, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
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
            max_tokens: Some(8192),
            chat_template_kwargs: None,
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

#[derive(Debug, Serialize)]
pub(crate) struct DraftExtractionResponse {
    pub schema_version: &'static str,
    pub prompt: DraftExtractionPrompt,
    pub llm_response: ChatCompletionResponse,
    pub persisted: bool,
}
