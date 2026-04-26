use std::collections::HashMap;

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
    build_character_fields_prompt, build_character_identity_prompt,
    build_character_mentions_prompt, build_draft_extraction_prompt, build_import_preview,
    ActivateManagedLocalModelInput, AnalysisChapterRun, AnalysisChapterState, AnalysisRunSnapshot,
    AnalysisRunStepInput, AppConfig, Chapter, CreateProjectInput, CreateTranslationJobInput,
    DeleteProjectInput, DeleteProjectResult, DraftExtractionInput, DraftExtractionPrompt,
    LocalLlmRuntimeSnapshot, NovelImportInput, ProjectWorkspaceSnapshot, StoryCharacterMention,
    StoryEvidenceSpan, StoryExtractionDocument, StoryExtractionFieldPayload,
    StoryExtractionFieldValuePayload, StoryExtractionRecordPayload, StoryExtractionRecordView,
    API_VERSION, APP_VERSION, CHARACTER_EXTRACTION_SCHEMA_VERSION, STORAGE_SCHEMA_VERSION,
};
use novelgraph_storage::{SqliteStore, StorageError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use tower_http::trace::TraceLayer;

const CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS: usize = 2400;
const CHARACTER_EXTRACTION_CHUNK_MIN_CHARS: usize = 900;
const CHARACTER_IDENTITY_MAX_TOKENS: u32 = 512;
const CHARACTER_MENTIONS_MAX_TOKENS: u32 = 1024;
const CHARACTER_FIELDS_MAX_TOKENS: u32 = 16384;

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
            "/api/projects/{project_id}/analysis/jobs/{job_id}/run",
            get(get_analysis_run),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/run/step",
            post(run_next_analysis_chapter),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/run/reset",
            post(reset_analysis_run),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/pause",
            post(pause_analysis_run),
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
    let character_records = match &latest_analysis_job {
        Some(job) => {
            state
                .store
                .list_story_extraction_records(&project_id, &job.id, "character")
                .await?
        }
        None => Vec::new(),
    };

    Ok(Json(ProjectWorkspaceSnapshot {
        project,
        novels,
        active_novel,
        chapters,
        latest_analysis_job,
        latest_job_events,
        character_records,
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

async fn get_analysis_run(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    Ok(Json(
        build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
    ))
}

async fn reset_analysis_run(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    state.store.reset_analysis_run(&project_id, &job_id).await?;

    Ok(Json(
        build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
    ))
}

async fn pause_analysis_run(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    let reason = "Tạm dừng bởi người dùng.";
    state
        .store
        .pause_analysis_job(&project_id, &job_id, reason, None, false)
        .await?;

    Ok(Json(
        build_analysis_run_snapshot(&state.store, &project_id, &job_id, Some(reason.to_string()))
            .await?,
    ))
}

async fn run_next_analysis_chapter(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
    Json(input): Json<AnalysisRunStepInput>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    let chapter_range = chapter_range_from_input(&input)?;
    if input.force {
        if let Some(range) = chapter_range {
            state
                .store
                .reset_analysis_run_range(&project_id, &job_id, range.from, range.to)
                .await?;
        } else {
            state.store.reset_analysis_run(&project_id, &job_id).await?;
        }
    }

    let current_job = state
        .store
        .get_analysis_job(&project_id, &job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))?;
    if current_job.status == "completed" {
        return Ok(Json(
            build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
        ));
    }

    let job = state
        .store
        .mark_analysis_job_running(&project_id, &job_id)
        .await?;
    let novel_id = job
        .novel_id
        .as_deref()
        .ok_or_else(|| ApiError::bad_request("analysis job is not attached to a novel"))?;
    let novel = state
        .store
        .get_novel(&project_id, novel_id)
        .await?
        .ok_or(ApiError::not_found("novel"))?;
    let chapters = state.store.list_chapters(&project_id, novel_id).await?;
    let runs = state
        .store
        .list_analysis_chapter_runs(&project_id, &job_id)
        .await?;

    if next_analysis_chapter(&chapters, &runs, chapter_range).is_none() {
        finish_analysis_range_or_job(
            &state.store,
            &project_id,
            &job_id,
            &chapters,
            &runs,
            chapter_range,
        )
        .await?;
        return Ok(Json(
            build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
        ));
    }

    let health = state.local_llm.health().await?;
    if !health.reachable {
        let reason = format!(
            "Local llama.cpp không reachable: {}",
            health
                .status_text
                .unwrap_or_else(|| "request failed".to_string())
        );
        state
            .store
            .pause_analysis_job(
                &project_id,
                &job_id,
                &reason,
                Some("local_llm_unreachable"),
                true,
            )
            .await?;

        return Ok(Json(
            build_analysis_run_snapshot(&state.store, &project_id, &job_id, Some(reason)).await?,
        ));
    }

    let chapter = next_analysis_chapter(&chapters, &runs, chapter_range)
        .cloned()
        .ok_or_else(|| ApiError::bad_request("no chapter is available for analysis"))?;
    let chapter_run = state
        .store
        .start_analysis_chapter_run(&project_id, &job_id, novel_id, &chapter)
        .await?;

    let chunks = split_chapter_for_character_extraction(&chapter.content);
    let mut working_document = StoryExtractionDocument {
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION.to_string(),
        chapter_num: chapter.chapter_num,
        records: Vec::new(),
    };
    let mut chunk_outputs = Vec::with_capacity(chunks.len());

    for (index, chunk) in chunks.iter().enumerate() {
        if analysis_job_should_stop(&state.store, &project_id, &job_id).await? {
            return Ok(Json(
                build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
            ));
        }

        let chunk_input = DraftExtractionInput {
            chapter_num: chapter.chapter_num,
            title: Some(chapter.title.clone()),
            source_language: novel.source_language.clone(),
            text: chunk.text.clone(),
            prior_context: Some(format!(
                "Đây là đoạn nhỏ {}/{} của chương hiện tại. Mỗi pass chỉ xử lý dữ liệu có trong đoạn này. Offset mention phải tính từ CHAPTER_TEXT của đoạn này; backend sẽ tự quy đổi về toàn chương.",
                index + 1,
                chunks.len()
            )),
        };

        let identity_prompt = build_character_identity_prompt(&chunk_input);
        let (chunk_identities, identity_response) =
            match call_local_json_array::<CharacterIdentity>(
                &state,
                &identity_prompt,
                CHARACTER_IDENTITY_MAX_TOKENS,
            )
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    let reason = format!(
                        "character identity chunk {}/{} failed: {}",
                        index + 1,
                        chunks.len(),
                        error.message
                    );
                    return fail_analysis_chapter_and_pause(
                        &state,
                        &project_id,
                        &job_id,
                        &chapter_run.chapter_id,
                        "character_identity_pass_failed",
                        reason,
                    )
                    .await;
                }
            };
        let chunk_identities = normalize_character_identities(chunk_identities);
        merge_character_identity_records(&mut working_document, &chunk_identities);
        normalize_character_field_keys(&mut working_document);
        state
            .store
            .replace_story_extraction_records_for_chapter(
                &project_id,
                &job_id,
                &chapter.id,
                CHARACTER_EXTRACTION_SCHEMA_VERSION,
                &working_document,
                "character_identity_chunk",
            )
            .await?;

        let db_records = state
            .store
            .list_story_extraction_records(&project_id, &job_id, "character")
            .await?;
        let current_identities =
            db_identities_for_chunk(&db_records, chapter.chapter_num, &chunk_identities);
        let mut character_passes = Vec::new();

        for identity in current_identities {
            if analysis_job_should_stop(&state.store, &project_id, &job_id).await? {
                return Ok(Json(
                    build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
                ));
            }

            let character_json =
                serde_json::to_string(&identity).unwrap_or_else(|_| "{}".to_string());

            let mentions_prompt = build_character_mentions_prompt(&chunk_input, &character_json);
            let (mentions, mentions_response) =
                match call_local_json_array::<StoryCharacterMention>(
                    &state,
                    &mentions_prompt,
                    CHARACTER_MENTIONS_MAX_TOKENS,
                )
                .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        let reason = format!(
                            "character mentions chunk {}/{} for {} failed: {}",
                            index + 1,
                            chunks.len(),
                            identity.name,
                            error.message
                        );
                        return fail_analysis_chapter_and_pause(
                            &state,
                            &project_id,
                            &job_id,
                            &chapter_run.chapter_id,
                            "character_mentions_pass_failed",
                            reason,
                        )
                        .await;
                    }
                };
            let mut mentions = repair_character_mention_list(mentions, &chunk.text);
            shift_character_mentions(&mut mentions, chunk.start_char);
            merge_character_identity_mentions(&mut working_document, &identity, mentions);
            state
                .store
                .replace_story_extraction_records_for_chapter(
                    &project_id,
                    &job_id,
                    &chapter.id,
                    CHARACTER_EXTRACTION_SCHEMA_VERSION,
                    &working_document,
                    "character_mentions_chunk",
                )
                .await?;

            let fields_prompt = build_character_fields_prompt(&chunk_input, &character_json);
            let (fields, fields_response) =
                match call_local_json_array::<StoryExtractionFieldPayload>(
                    &state,
                    &fields_prompt,
                    CHARACTER_FIELDS_MAX_TOKENS,
                )
                .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        let reason = format!(
                            "character fields chunk {}/{} for {} failed: {}",
                            index + 1,
                            chunks.len(),
                            identity.name,
                            error.message
                        );
                        return fail_analysis_chapter_and_pause(
                            &state,
                            &project_id,
                            &job_id,
                            &chapter_run.chapter_id,
                            "character_fields_pass_failed",
                            reason,
                        )
                        .await;
                    }
                };
            merge_character_identity_fields(&mut working_document, &identity, fields);
            normalize_character_field_keys(&mut working_document);
            state
                .store
                .replace_story_extraction_records_for_chapter(
                    &project_id,
                    &job_id,
                    &chapter.id,
                    CHARACTER_EXTRACTION_SCHEMA_VERSION,
                    &working_document,
                    "character_fields_chunk",
                )
                .await?;

            character_passes.push(json!({
                "name": identity.name,
                "aliases": identity.aliases,
                "mentions_response": mentions_response,
                "fields_response": fields_response,
            }));
        }

        chunk_outputs.push(json!({
            "chunk_index": index + 1,
            "chunk_count": chunks.len(),
            "start_char": chunk.start_char,
            "end_char": chunk.end_char,
            "identity_response": identity_response,
            "character_passes": character_passes,
        }));

        if analysis_job_should_stop(&state.store, &project_id, &job_id).await? {
            return Ok(Json(
                build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
            ));
        }
    }

    let mut character_extraction = working_document;
    normalize_character_field_keys(&mut character_extraction);
    if let Err(error) = validate_character_extraction_document(
        &character_extraction,
        chapter.chapter_num,
        &chapter.content,
    ) {
        let reason = format!(
            "character extraction merged result failed validation: {}",
            error.message
        );
        state
            .store
            .fail_analysis_chapter_run(
                &project_id,
                &job_id,
                &chapter_run.chapter_id,
                "character_extraction_validation_failed",
                &reason,
            )
            .await?;
        state
            .store
            .pause_analysis_job(
                &project_id,
                &job_id,
                &reason,
                Some("character_extraction_validation_failed"),
                true,
            )
            .await?;

        return Ok(Json(
            build_analysis_run_snapshot(&state.store, &project_id, &job_id, Some(reason)).await?,
        ));
    }

    let output_json = json!({
        "schema_version": CHARACTER_EXTRACTION_SCHEMA_VERSION,
        "extraction_mode": "staged_chunked_character",
        "chunk_target_chars": CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS,
        "chunk_count": chunks.len(),
        "chunks": chunk_outputs,
        "persisted": true,
        "persisted_group_key": "character",
        "character_record_count": character_extraction.records.len(),
    })
    .to_string();
    state
        .store
        .complete_analysis_chapter_run_with_story_extraction(
            &project_id,
            &job_id,
            &chapter.id,
            CHARACTER_EXTRACTION_SCHEMA_VERSION,
            &output_json,
            &character_extraction,
        )
        .await?;

    let runs = state
        .store
        .list_analysis_chapter_runs(&project_id, &job_id)
        .await?;
    finish_analysis_range_or_job(
        &state.store,
        &project_id,
        &job_id,
        &chapters,
        &runs,
        chapter_range,
    )
    .await?;

    Ok(Json(
        build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
    ))
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

async fn fail_analysis_chapter_and_pause(
    state: &AppState,
    project_id: &str,
    job_id: &str,
    chapter_id: &str,
    error_code: &'static str,
    reason: String,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    state
        .store
        .fail_analysis_chapter_run(project_id, job_id, chapter_id, error_code, &reason)
        .await?;
    state
        .store
        .pause_analysis_job(project_id, job_id, &reason, Some(error_code), true)
        .await?;

    Ok(Json(
        build_analysis_run_snapshot(&state.store, project_id, job_id, Some(reason)).await?,
    ))
}

async fn analysis_job_should_stop(
    store: &SqliteStore,
    project_id: &str,
    job_id: &str,
) -> Result<bool, ApiError> {
    let job = store
        .get_analysis_job(project_id, job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))?;

    Ok(matches!(
        job.status.as_str(),
        "paused" | "cancelled" | "completed" | "failed"
    ))
}

async fn call_local_json_array<T>(
    state: &AppState,
    prompt: &DraftExtractionPrompt,
    max_tokens: u32,
) -> Result<(Vec<T>, ChatCompletionResponse), ApiError>
where
    T: DeserializeOwned,
{
    let response = state
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
            temperature: Some(0.0),
            max_tokens: Some(max_tokens),
            chat_template_kwargs: Some(json!({ "enable_thinking": false })),
            stream: false,
        })
        .await?;
    let items = parse_json_array_response::<T>(&response)?;

    Ok((items, response))
}

fn parse_json_array_response<T>(response: &ChatCompletionResponse) -> Result<Vec<T>, ApiError>
where
    T: DeserializeOwned,
{
    let content = response
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| ApiError::bad_request("local LLM returned empty JSON array response"))?;
    let json_text = extract_json_array(content)
        .ok_or_else(|| ApiError::bad_request("local LLM did not return a JSON array"))?;

    serde_json::from_str::<Vec<T>>(json_text)
        .map_err(|err| ApiError::bad_request(format!("local LLM JSON array parse failed: {err}")))
}

async fn build_analysis_run_snapshot(
    store: &SqliteStore,
    project_id: &str,
    job_id: &str,
    paused_reason: Option<String>,
) -> Result<AnalysisRunSnapshot, ApiError> {
    let job = store
        .get_analysis_job(project_id, job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))?;
    let novel_id = job
        .novel_id
        .as_deref()
        .ok_or_else(|| ApiError::bad_request("analysis job is not attached to a novel"))?;
    let chapters = store.list_chapters(project_id, novel_id).await?;
    let runs = store.list_analysis_chapter_runs(project_id, job_id).await?;
    let character_records = store
        .list_story_extraction_records(project_id, job_id, "character")
        .await?;
    let run_by_chapter = runs
        .iter()
        .map(|run| (run.chapter_id.as_str(), run))
        .collect::<HashMap<_, _>>();

    let chapter_states = chapters
        .iter()
        .map(|chapter| {
            let run = run_by_chapter.get(chapter.id.as_str()).copied();

            AnalysisChapterState {
                chapter_id: chapter.id.clone(),
                chapter_num: chapter.chapter_num,
                title: chapter.title.clone(),
                status: run
                    .map(|run| run.status.clone())
                    .unwrap_or_else(|| "pending".to_string()),
                run_id: run.map(|run| run.id.clone()),
                attempt: run.map(|run| run.attempt),
                prompt_schema_version: run.and_then(|run| run.prompt_schema_version.clone()),
                error_code: run.and_then(|run| run.error_code.clone()),
                error_message: run.and_then(|run| run.error_message.clone()),
                started_at: run.and_then(|run| run.started_at.clone()),
                finished_at: run.and_then(|run| run.finished_at.clone()),
                updated_at: run.map(|run| run.updated_at.clone()),
            }
        })
        .collect::<Vec<_>>();

    let completed_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "completed")
        .count();
    let running_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "running")
        .count();
    let failed_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "failed")
        .count();
    let pending_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "pending")
        .count();
    let next_chapter_num = chapter_states
        .iter()
        .find(|chapter| chapter.status != "completed")
        .map(|chapter| chapter.chapter_num);
    let paused_reason = paused_reason.or_else(|| {
        if job.status == "paused" {
            job.error_message.clone()
        } else {
            None
        }
    });

    Ok(AnalysisRunSnapshot {
        job,
        total_chapters: chapters.len(),
        completed_chapters,
        running_chapters,
        failed_chapters,
        pending_chapters,
        next_chapter_num,
        paused_reason,
        chapters: chapter_states,
        character_records,
    })
}

#[derive(Debug, Clone, Copy)]
struct ChapterRange {
    from: i64,
    to: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterIdentity {
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
}

fn chapter_range_from_input(
    input: &AnalysisRunStepInput,
) -> Result<Option<ChapterRange>, ApiError> {
    match (input.from_chapter_num, input.to_chapter_num) {
        (None, None) => Ok(None),
        (Some(from), None) => validate_chapter_range(from, from).map(Some),
        (Some(from), Some(to)) => validate_chapter_range(from, to).map(Some),
        (None, Some(to)) => validate_chapter_range(to, to).map(Some),
    }
}

fn validate_chapter_range(from: i64, to: i64) -> Result<ChapterRange, ApiError> {
    if from < 1 || to < 1 {
        return Err(ApiError::bad_request(
            "chapter range must start from 1 or greater",
        ));
    }

    if to < from {
        return Err(ApiError::bad_request(
            "to chapter must be greater than or equal to from chapter",
        ));
    }

    Ok(ChapterRange { from, to })
}

async fn finish_analysis_range_or_job(
    store: &SqliteStore,
    project_id: &str,
    job_id: &str,
    chapters: &[Chapter],
    runs: &[AnalysisChapterRun],
    range: Option<ChapterRange>,
) -> Result<(), ApiError> {
    if next_analysis_chapter(chapters, runs, None).is_none() {
        store.complete_analysis_job(project_id, job_id).await?;
        return Ok(());
    }

    if let Some(range) = range {
        if next_analysis_chapter(chapters, runs, Some(range)).is_none() {
            let reason = format!(
                "Đã chạy xong phạm vi chương {} -> {}.",
                range.from, range.to
            );
            store
                .pause_analysis_job(
                    project_id,
                    job_id,
                    &reason,
                    Some("analysis_range_completed"),
                    true,
                )
                .await?;
        }
    }

    Ok(())
}

fn next_analysis_chapter<'a>(
    chapters: &'a [Chapter],
    runs: &[AnalysisChapterRun],
    range: Option<ChapterRange>,
) -> Option<&'a Chapter> {
    let completed_by_chapter = runs
        .iter()
        .filter(|run| run.status == "completed")
        .map(|run| run.chapter_id.as_str())
        .collect::<std::collections::HashSet<_>>();

    chapters
        .iter()
        .filter(|chapter| {
            range
                .map(|range| chapter.chapter_num >= range.from && chapter.chapter_num <= range.to)
                .unwrap_or(true)
        })
        .find(|chapter| !completed_by_chapter.contains(chapter.id.as_str()))
}

#[derive(Debug, Clone)]
struct CharacterExtractionChunk {
    start_char: i64,
    end_char: i64,
    text: String,
}

fn split_chapter_for_character_extraction(text: &str) -> Vec<CharacterExtractionChunk> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return vec![CharacterExtractionChunk {
            start_char: 0,
            end_char: 0,
            text: String::new(),
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = choose_character_extraction_chunk_end(&chars, start);
        let chunk_text = chars[start..end].iter().collect::<String>();
        if !chunk_text.trim().is_empty() {
            chunks.push(CharacterExtractionChunk {
                start_char: start as i64,
                end_char: end as i64,
                text: chunk_text,
            });
        }
        start = end.max(start + 1);
    }

    if chunks.is_empty() {
        chunks.push(CharacterExtractionChunk {
            start_char: 0,
            end_char: chars.len() as i64,
            text: text.to_string(),
        });
    }

    chunks
}

fn choose_character_extraction_chunk_end(chars: &[char], start: usize) -> usize {
    let remaining = chars.len().saturating_sub(start);
    if remaining <= CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS {
        return chars.len();
    }

    let hard_end = (start + CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS).min(chars.len());
    let min_end = (start + CHARACTER_EXTRACTION_CHUNK_MIN_CHARS).min(hard_end);

    for index in (min_end..hard_end).rev() {
        if chars[index] == '\n' && index + 1 < chars.len() && chars[index + 1] == '\n' {
            return index + 2;
        }
    }

    for index in (min_end..hard_end).rev() {
        if chars[index] == '\n' {
            return index + 1;
        }
    }

    for index in (min_end..hard_end).rev() {
        if matches!(chars[index], '.' | '!' | '?' | '。' | '！' | '？') {
            return index + 1;
        }
    }

    hard_end
}

fn shift_character_mentions(mentions: &mut [StoryCharacterMention], offset: i64) {
    if offset == 0 {
        return;
    }

    for mention in mentions {
        mention.start_char += offset;
        mention.end_char += offset;
    }
}

fn normalize_character_identities(identities: Vec<CharacterIdentity>) -> Vec<CharacterIdentity> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for identity in identities {
        let name = identity.name.trim().to_string();
        if name.is_empty() {
            continue;
        }

        let mut aliases = Vec::new();
        let mut alias_seen = std::collections::HashSet::new();
        for alias in identity.aliases {
            let alias = alias.trim().to_string();
            if alias.is_empty() || normalized_text_key(&alias) == normalized_text_key(&name) {
                continue;
            }
            if alias_seen.insert(normalized_text_key(&alias)) {
                aliases.push(alias);
            }
        }

        let key = normalized_text_key(&name);
        if seen.insert(key) {
            normalized.push(CharacterIdentity { name, aliases });
        }
    }

    normalized
}

fn merge_character_identity_records(
    document: &mut StoryExtractionDocument,
    identities: &[CharacterIdentity],
) {
    for identity in identities {
        let record = identity_to_record(identity);
        if let Some(target) = document
            .records
            .iter_mut()
            .find(|record| payload_record_matches_identity(record, identity))
        {
            merge_character_record(target, record);
        } else {
            document.records.push(record);
        }
    }
}

fn identity_to_record(identity: &CharacterIdentity) -> StoryExtractionRecordPayload {
    let mut fields = Vec::new();
    if !identity.aliases.is_empty() {
        fields.push(StoryExtractionFieldPayload {
            field_key: "aliases".to_string(),
            field_label: "Tên gọi khác".to_string(),
            values: identity
                .aliases
                .iter()
                .map(|alias| StoryExtractionFieldValuePayload {
                    value: alias.clone(),
                    confidence: Some(1.0),
                    evidence: Vec::new(),
                })
                .collect(),
        });
    }

    StoryExtractionRecordPayload {
        group_key: "character".to_string(),
        group_label: "Nhân Vật".to_string(),
        entity_key: Some(normalize_ascii_snake_key(&identity.name)),
        display_name: identity.name.clone(),
        mentions: Vec::new(),
        fields,
    }
}

fn db_identities_for_chunk(
    records: &[StoryExtractionRecordView],
    chapter_num: i64,
    chunk_identities: &[CharacterIdentity],
) -> Vec<CharacterIdentity> {
    let mut identities = Vec::new();

    for record in records {
        if record.chapter_num != chapter_num {
            continue;
        }

        if !chunk_identities
            .iter()
            .any(|identity| view_record_matches_identity(record, identity))
        {
            continue;
        }

        identities.push(CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_record(record),
        });
    }

    identities
}

fn aliases_from_record(record: &StoryExtractionRecordView) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for field in &record.fields {
        let field_key = normalize_ascii_snake_key(&field.field_key);
        if !matches!(
            field_key.as_str(),
            "alias" | "aliases" | "other_name" | "other_names"
        ) {
            continue;
        }

        for value in &field.values {
            let alias = value.value.trim();
            if alias.is_empty() {
                continue;
            }
            if seen.insert(normalized_text_key(alias)) {
                aliases.push(alias.to_string());
            }
        }
    }

    aliases
}

fn payload_record_matches_identity(
    record: &StoryExtractionRecordPayload,
    identity: &CharacterIdentity,
) -> bool {
    let identity_names = character_identity_surface_keys(identity);
    if identity_names.contains(&normalized_text_key(&record.display_name)) {
        return true;
    }

    for field in &record.fields {
        let field_key = normalize_ascii_snake_key(&field.field_key);
        if !matches!(
            field_key.as_str(),
            "alias" | "aliases" | "other_name" | "other_names"
        ) {
            continue;
        }

        for value in &field.values {
            if identity_names.contains(&normalized_text_key(&value.value)) {
                return true;
            }
        }
    }

    false
}

fn view_record_matches_identity(
    record: &StoryExtractionRecordView,
    identity: &CharacterIdentity,
) -> bool {
    let identity_names = character_identity_surface_keys(identity);
    if identity_names.contains(&normalized_text_key(&record.display_name)) {
        return true;
    }

    aliases_from_record(record)
        .iter()
        .any(|alias| identity_names.contains(&normalized_text_key(alias)))
}

fn character_identity_surface_keys(
    identity: &CharacterIdentity,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    keys.insert(normalized_text_key(&identity.name));
    for alias in &identity.aliases {
        keys.insert(normalized_text_key(alias));
    }
    keys
}

fn merge_character_identity_mentions(
    document: &mut StoryExtractionDocument,
    identity: &CharacterIdentity,
    mentions: Vec<StoryCharacterMention>,
) {
    let target = ensure_character_record(document, identity);
    merge_character_mentions(&mut target.mentions, mentions);
}

fn merge_character_identity_fields(
    document: &mut StoryExtractionDocument,
    identity: &CharacterIdentity,
    fields: Vec<StoryExtractionFieldPayload>,
) {
    let target = ensure_character_record(document, identity);
    for field in fields {
        merge_character_field(&mut target.fields, field);
    }
}

fn ensure_character_record<'a>(
    document: &'a mut StoryExtractionDocument,
    identity: &CharacterIdentity,
) -> &'a mut StoryExtractionRecordPayload {
    let key = normalized_text_key(&identity.name);
    if let Some(index) = document.records.iter().position(|record| {
        normalized_text_key(&record.display_name) == key
            || payload_record_matches_identity(record, identity)
    }) {
        return &mut document.records[index];
    }

    document.records.push(identity_to_record(identity));
    document
        .records
        .last_mut()
        .expect("record was just inserted")
}

fn merge_character_record(
    target: &mut StoryExtractionRecordPayload,
    mut source: StoryExtractionRecordPayload,
) {
    if target
        .entity_key
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        target.entity_key = source.entity_key.take();
    }

    merge_character_mentions(&mut target.mentions, source.mentions);

    for source_field in source.fields {
        merge_character_field(&mut target.fields, source_field);
    }
}

fn merge_character_mentions(
    target: &mut Vec<StoryCharacterMention>,
    source: Vec<StoryCharacterMention>,
) {
    let mut seen = target
        .iter()
        .map(|mention| {
            format!(
                "{}:{}:{}",
                mention.start_char,
                mention.end_char,
                mention.text.trim()
            )
        })
        .collect::<std::collections::HashSet<_>>();

    for mention in source {
        let key = format!(
            "{}:{}:{}",
            mention.start_char,
            mention.end_char,
            mention.text.trim()
        );
        if seen.insert(key) {
            target.push(mention);
        }
    }

    target.sort_by(|left, right| {
        left.start_char
            .cmp(&right.start_char)
            .then_with(|| right.end_char.cmp(&left.end_char))
    });
}

fn merge_character_field(
    target: &mut Vec<StoryExtractionFieldPayload>,
    source: StoryExtractionFieldPayload,
) {
    let source_key = normalize_ascii_snake_key(&source.field_key);
    if let Some(target_field) = target
        .iter_mut()
        .find(|field| normalize_ascii_snake_key(&field.field_key) == source_key)
    {
        merge_character_field_values(&mut target_field.values, source.values);
        return;
    }

    target.push(StoryExtractionFieldPayload {
        field_key: source_key,
        field_label: source.field_label,
        values: source.values,
    });
}

fn merge_character_field_values(
    target: &mut Vec<StoryExtractionFieldValuePayload>,
    source: Vec<StoryExtractionFieldValuePayload>,
) {
    let mut value_index_by_key = target
        .iter()
        .enumerate()
        .map(|(index, value)| (normalized_text_key(&value.value), index))
        .collect::<HashMap<_, _>>();

    for source_value in source {
        let key = normalized_text_key(&source_value.value);
        if let Some(index) = value_index_by_key.get(&key).copied() {
            merge_story_evidence(&mut target[index].evidence, source_value.evidence);
        } else {
            let index = target.len();
            value_index_by_key.insert(key, index);
            target.push(source_value);
        }
    }
}

fn merge_story_evidence(target: &mut Vec<StoryEvidenceSpan>, source: Vec<StoryEvidenceSpan>) {
    let mut seen = target
        .iter()
        .map(story_evidence_key)
        .collect::<std::collections::HashSet<_>>();

    for evidence in source {
        let key = story_evidence_key(&evidence);
        if seen.insert(key) {
            target.push(evidence);
        }
    }
}

fn story_evidence_key(evidence: &StoryEvidenceSpan) -> String {
    format!(
        "{}:{}:{}:{}",
        evidence.start_char.unwrap_or(-1),
        evidence.end_char.unwrap_or(-1),
        evidence.quote.as_deref().unwrap_or_default().trim(),
        evidence.reason.as_deref().unwrap_or_default().trim()
    )
}

fn normalized_text_key(value: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = true;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
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

fn extract_json_array(content: &str) -> Option<&str> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;

    if end <= start {
        return None;
    }

    Some(&content[start..=end])
}

fn validate_character_extraction_document(
    document: &StoryExtractionDocument,
    chapter_num: i64,
    chapter_text: &str,
) -> Result<(), ApiError> {
    if document.schema_version.trim() != CHARACTER_EXTRACTION_SCHEMA_VERSION {
        return Err(ApiError::bad_request(format!(
            "character extraction schema mismatch: expected {}, got {}",
            CHARACTER_EXTRACTION_SCHEMA_VERSION, document.schema_version
        )));
    }

    if document.chapter_num != chapter_num {
        return Err(ApiError::bad_request(
            "character extraction chapter_num does not match the running chapter",
        ));
    }

    for record in &document.records {
        validate_character_record(record, chapter_num, chapter_text)?;
    }

    Ok(())
}

fn repair_character_mention_list(
    mentions: Vec<StoryCharacterMention>,
    chapter_text: &str,
) -> Vec<StoryCharacterMention> {
    let chapter_len = chapter_text.chars().count() as i64;
    let mut repaired_mentions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for mut mention in mentions {
        mention.text = mention.text.trim().to_string();
        if mention.text.is_empty() {
            continue;
        }

        if !is_valid_mention_span(&mention, chapter_len)
            || !mention_span_matches_text(chapter_text, &mention)
        {
            if let Some((start_char, end_char)) =
                find_best_mention_span(chapter_text, &mention.text, Some(mention.start_char))
            {
                mention.start_char = start_char;
                mention.end_char = end_char;
            }
        }

        if !is_valid_mention_span(&mention, chapter_len)
            || !mention_span_matches_text(chapter_text, &mention)
        {
            continue;
        }

        let key = format!(
            "{}:{}:{}",
            mention.start_char, mention.end_char, mention.text
        );
        if seen.insert(key) {
            repaired_mentions.push(mention);
        }
    }

    repaired_mentions.sort_by(|left, right| {
        left.start_char
            .cmp(&right.start_char)
            .then_with(|| right.end_char.cmp(&left.end_char))
    });
    repaired_mentions
}

fn is_valid_mention_span(mention: &StoryCharacterMention, chapter_len: i64) -> bool {
    mention.start_char >= 0
        && mention.end_char > mention.start_char
        && mention.end_char <= chapter_len
}

fn mention_span_matches_text(chapter_text: &str, mention: &StoryCharacterMention) -> bool {
    text_at_char_span(chapter_text, mention.start_char, mention.end_char)
        .as_deref()
        .is_some_and(|value| value == mention.text)
}

fn text_at_char_span(text: &str, start_char: i64, end_char: i64) -> Option<String> {
    if start_char < 0 || end_char <= start_char {
        return None;
    }

    Some(
        text.chars()
            .skip(start_char as usize)
            .take((end_char - start_char) as usize)
            .collect(),
    )
}

fn find_best_mention_span(
    chapter_text: &str,
    mention_text: &str,
    preferred_start: Option<i64>,
) -> Option<(i64, i64)> {
    let mention_text = mention_text.trim();
    if mention_text.is_empty() {
        return None;
    }

    let mention_len = mention_text.chars().count() as i64;
    let preferred_start = preferred_start.unwrap_or(0);
    chapter_text
        .match_indices(mention_text)
        .map(|(byte_start, _)| {
            let start_char = chapter_text[..byte_start].chars().count() as i64;
            let end_char = start_char + mention_len;
            let distance = (start_char - preferred_start).abs();
            (distance, start_char, end_char)
        })
        .min_by_key(|(distance, _, _)| *distance)
        .map(|(_, start_char, end_char)| (start_char, end_char))
}

fn validate_character_record(
    record: &StoryExtractionRecordPayload,
    chapter_num: i64,
    chapter_text: &str,
) -> Result<(), ApiError> {
    if record.group_key.trim() != "character" {
        return Err(ApiError::bad_request(
            "character extraction records must use group_key character",
        ));
    }

    if record.group_label.trim().is_empty() {
        return Err(ApiError::bad_request(
            "character extraction group_label is required",
        ));
    }

    if record.display_name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "character extraction display_name is required",
        ));
    }

    for mention in &record.mentions {
        if mention.text.trim().is_empty() {
            return Err(ApiError::bad_request(
                "character extraction mention text is required",
            ));
        }

        let chapter_len = chapter_text.chars().count() as i64;
        if mention.start_char < 0
            || mention.end_char <= mention.start_char
            || mention.end_char > chapter_len
        {
            return Err(ApiError::bad_request(
                "character extraction mention span is outside chapter bounds",
            ));
        }
    }

    for field in &record.fields {
        if field.field_key.trim().is_empty() {
            return Err(ApiError::bad_request(
                "character extraction field_key is required",
            ));
        }

        if field.field_label.trim().is_empty() {
            return Err(ApiError::bad_request(
                "character extraction field_label is required",
            ));
        }

        for value in &field.values {
            if value.value.trim().is_empty() {
                return Err(ApiError::bad_request(
                    "character extraction value is required",
                ));
            }

            if let Some(confidence) = value.confidence {
                if !(0.0..=1.0).contains(&confidence) {
                    return Err(ApiError::bad_request(
                        "character extraction confidence must be between 0 and 1",
                    ));
                }
            }

            for evidence in &value.evidence {
                if evidence.chapter_num != chapter_num {
                    return Err(ApiError::bad_request(
                        "character extraction evidence chapter_num does not match the running chapter",
                    ));
                }

                if let (Some(start), Some(end)) = (evidence.start_char, evidence.end_char) {
                    let chapter_len = chapter_text.chars().count() as i64;
                    if start < 0 || end < start || end > chapter_len {
                        return Err(ApiError::bad_request(
                            "character extraction evidence span is outside chapter bounds",
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

fn normalize_character_field_keys(document: &mut StoryExtractionDocument) {
    for record in &mut document.records {
        for field in &mut record.fields {
            field.field_key = normalize_ascii_snake_key(&field.field_key);
        }
    }
}

fn normalize_ascii_snake_key(value: &str) -> String {
    let value = value.trim();
    if is_ascii_snake_key(value) {
        return value.to_string();
    }

    let mut normalized = String::new();
    let mut last_was_separator = true;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if let Some(ascii) = fold_key_char(ch) {
            normalized.push(ascii);
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }

    while normalized.ends_with('_') {
        normalized.pop();
    }

    if normalized.is_empty() {
        return "field".to_string();
    }

    if !normalized
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_lowercase())
    {
        normalized.insert_str(0, "field_");
    }

    normalized
}

fn is_ascii_snake_key(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_lowercase()
        || bytes.last().is_some_and(|byte| *byte == b'_')
        || value.contains("__")
    {
        return false;
    }

    bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

fn fold_key_char(ch: char) -> Option<char> {
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
