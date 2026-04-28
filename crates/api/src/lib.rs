use std::{collections::HashMap, fs, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
pub mod local_runtime;
use local_runtime::{LocalLlmRuntimeManager, LocalRuntimeError};
use novelgraph_ai::{
    AiError, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, LlamaCppClient, LlmRole,
    LocalLlmHealth, ModelListResponse,
};
use novelgraph_core::{
    build_character_alias_ownership_prompt, build_character_candidate_prompt,
    build_character_field_value_verification_prompt, build_character_fields_prompt,
    build_character_identity_creation_review_prompt,
    build_character_identity_merge_confirmation_prompt, build_character_identity_prompt,
    build_character_occurrence_confirmation_prompt, build_character_relationship_candidate_prompt,
    build_character_relationship_verification_prompt, build_draft_extraction_prompt,
    build_import_preview, build_novel_metadata_suggestion_prompt, detect_basic_source_language,
    ActivateManagedLocalModelInput, AnalysisChapterRun, AnalysisChapterState, AnalysisRunSnapshot,
    AnalysisRunStepInput, AppConfig, ByokProviderConfigRecord, ByokProviderConfigView,
    ByokProviderKeyHealth, ByokProviderPreset, Chapter, CheckByokProviderKeyInput,
    CreateProjectInput, CreateTranslationJobInput, DeleteProjectInput, DeleteProjectResult,
    DraftExtractionInput, DraftExtractionPrompt, LocalLlmRuntimeSnapshot, Novel, NovelImportInput,
    NovelMetadataSuggestion, NovelMetadataUpdateInput, ProjectWorkspaceSnapshot,
    SaveByokProviderConfigInput, SaveByokProviderConfigResult, StoryCharacterAliasView,
    StoryCharacterMention, StoryEvidenceSpan, StoryExtractionDocument, StoryExtractionFieldPayload,
    StoryExtractionFieldValuePayload, StoryExtractionRecordPayload, StoryExtractionRecordView,
    API_VERSION, APP_VERSION, CHARACTER_EXTRACTION_SCHEMA_VERSION, STORAGE_SCHEMA_VERSION,
};
use novelgraph_storage::{SqliteStore, StorageError};
use ring::{
    aead,
    digest::{digest, SHA256},
    rand::{SecureRandom, SystemRandom},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;

const CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS: usize = 2400;
const CHARACTER_EXTRACTION_CHUNK_MIN_CHARS: usize = 900;
const CHARACTER_CANDIDATE_MAX_TOKENS: u32 = 2048;
const CHARACTER_IDENTITY_MAX_TOKENS: u32 = 512;
const CHARACTER_ALIAS_OWNERSHIP_MAX_TOKENS: u32 = 2048;
const CHARACTER_ALIAS_OWNERSHIP_MIN_CONFIDENCE: f64 = 0.78;
const CHARACTER_ALIAS_OWNER_REDIRECT_MIN_SCORE: f64 = 0.55;
const CHARACTER_IDENTITY_MERGE_CONFIRMATION_MAX_TOKENS: u32 = 512;
const CHARACTER_IDENTITY_CREATION_REVIEW_MAX_TOKENS: u32 = 768;
const CHARACTER_IDENTITY_CREATION_REVIEW_MIN_CONFIDENCE: f64 = 0.80;
const CHARACTER_IDENTITY_REJECT_MIN_CONFIDENCE: f64 = 0.78;
const CHARACTER_OCCURRENCE_CONFIRMATION_MAX_TOKENS: u32 = 512;
const CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT: usize = 3;
const CHARACTER_OCCURRENCE_CONFIRMATION_MIN_CONFIDENCE: f64 = 0.5;
const CHARACTER_FIELDS_MAX_TOKENS: u32 = 32768;
const CHARACTER_FIELD_VALUE_MIN_CONFIDENCE: f64 = 0.85;
const CHARACTER_FIELD_VALUE_VERIFICATION_MAX_TOKENS: u32 = 512;
const CHARACTER_FIELD_VALUE_VERIFICATION_MIN_CONFIDENCE: f64 = 0.85;
const CHARACTER_RELATIONSHIP_CANDIDATE_MAX_TOKENS: u32 = 4096;
const CHARACTER_RELATIONSHIP_MIN_CONFIDENCE: f64 = 0.78;
const CHARACTER_RELATIONSHIP_VERIFICATION_MAX_TOKENS: u32 = 768;
const CHARACTER_RELATIONSHIP_VERIFICATION_MIN_CONFIDENCE: f64 = 0.85;
const CHARACTER_CANONICAL_AUTO_MERGE_SCORE: f64 = 0.98;
const CHARACTER_CANONICAL_REVIEW_MIN_SCORE: f64 = 0.70;
const CHARACTER_CANONICAL_REVIEW_SCORE_GAP: f64 = 0.05;
const CHARACTER_CANONICAL_AI_MERGE_MIN_SCORE: f64 = 0.90;
const CHARACTER_CANONICAL_MERGE_MIN_CONFIDENCE: f64 = 0.80;
const CHARACTER_CANONICAL_IGNORE_MIN_CONFIDENCE: f64 = 0.95;
const CHARACTER_CANONICAL_STORED_ALIAS_NAME_MATCH_SCORE: f64 = 0.82;
const CHARACTER_OCCURRENCE_CONTEXT_CHARS: i64 = 150;
const CHARACTER_FIELD_CONTEXT_MAX_ITEMS: usize = 12;
const LOCAL_JSON_REPAIR_INPUT_MAX_CHARS: usize = 16_000;
const NOVEL_METADATA_MAX_TOKENS: u32 = 1024;
const GEMINI_OPENAI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";
const GEMINI_DEFAULT_MODEL: &str = "gemini-2.5-flash";
const SECRET_CIPHERTEXT_PREFIX: &str = "ngenc:v1";

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: SqliteStore,
    pub local_llm: LlamaCppClient,
    pub local_runtime: LocalLlmRuntimeManager,
    realtime_tx: broadcast::Sender<ProjectRealtimeEvent>,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectRealtimeEvent {
    project_id: String,
    event_type: String,
    job_id: Option<String>,
    chapter_id: Option<String>,
    detail: String,
}

fn build_current_novel_analysis_context(novel: &Novel) -> String {
    let title = novel.title.trim();
    let author = novel.author.as_deref().unwrap_or("").trim();
    let source_language = novel.source_language.as_deref().unwrap_or("").trim();
    let genre = novel.genre.as_deref().unwrap_or("").trim();
    let description = novel.description.as_deref().unwrap_or("").trim();

    format!(
        "Current novel metadata:\n- Title: {title}\n- Author: {author}\n- Source language: {source_language}\n- Genre: {genre}\n- Description: {description}\n\nGenre guidance:\n- Dùng Genre và Description để chọn phong cách nhãn, cách hiểu danh xưng, vai trò, quan hệ, năng lực và chi tiết nhân vật phù hợp với bối cảnh truyện hiện tại.\n- Không dùng metadata làm evidence. Mọi nhân vật, alias, field, mention và relationship được output vẫn phải có chứng cứ trực tiếp trong CHAPTER_TEXT hoặc TARGET_CONTEXTS của request hiện tại.\n- Không ép output theo một taxonomy cố định nếu thể loại hoặc văn cảnh yêu cầu cách gọi tự nhiên hơn.\n- Output label tiếng Việt có dấu, rõ nghĩa với người đọc truyện, nhưng field_key vẫn phải dùng ASCII snake_case khi schema yêu cầu.\n- Nếu Genre hoặc Description rỗng, mơ hồ, hoặc xung đột với chương hiện tại, ưu tiên chứng cứ trong chương."
    )
}

pub fn build_router(
    config: AppConfig,
    store: SqliteStore,
    local_llm: LlamaCppClient,
    local_runtime: LocalLlmRuntimeManager,
) -> Router {
    let (realtime_tx, _) = broadcast::channel(256);
    let state = AppState {
        config,
        store,
        local_llm,
        local_runtime,
        realtime_tx,
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
        .route("/api/byok/providers", get(list_byok_providers))
        .route(
            "/api/byok/config",
            get(get_byok_config).post(save_byok_config),
        )
        .route("/api/byok/health-check", post(check_byok_key))
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
            "/api/projects/{project_id}/realtime",
            get(project_realtime_ws),
        )
        .route(
            "/api/projects/{project_id}/novels/import/preview",
            post(preview_novel_import),
        )
        .route(
            "/api/projects/{project_id}/novels/import/metadata-suggest",
            post(suggest_novel_import_metadata),
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
            "/api/projects/{project_id}/novels/{novel_id}/metadata",
            post(update_novel_metadata),
        )
        .route(
            "/api/projects/{project_id}/novels/{novel_id}/metadata/ai-fill",
            post(ai_fill_novel_metadata),
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

fn byok_provider_presets() -> Vec<ByokProviderPreset> {
    vec![
        ByokProviderPreset {
            id: "gemini".to_string(),
            name: "Google Gemini".to_string(),
            base_url: GEMINI_OPENAI_BASE_URL.to_string(),
            default_model: GEMINI_DEFAULT_MODEL.to_string(),
            models: vec![
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-pro".to_string(),
                "gemini-2.0-flash".to_string(),
            ],
            api_format: "openai".to_string(),
        },
        ByokProviderPreset {
            id: "openai-compatible".to_string(),
            name: "OpenAI-compatible".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            default_model: "provider-model-id".to_string(),
            models: Vec::new(),
            api_format: "openai".to_string(),
        },
    ]
}

fn byok_provider_preset(provider: &str) -> Option<ByokProviderPreset> {
    byok_provider_presets()
        .into_iter()
        .find(|preset| preset.id == provider)
}

fn byok_config_view(record: Option<&ByokProviderConfigRecord>) -> ByokProviderConfigView {
    let default_preset = byok_provider_preset("gemini").expect("gemini preset exists");
    let provider = record
        .map(|record| record.provider.clone())
        .unwrap_or(default_preset.id);
    let preset = byok_provider_preset(&provider);

    ByokProviderConfigView {
        provider,
        display_name: record
            .map(|record| record.display_name.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.name.clone()))
            .unwrap_or_else(|| "Google Gemini".to_string()),
        base_url: record
            .and_then(|record| record.base_url.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.base_url.clone()))
            .unwrap_or_else(|| GEMINI_OPENAI_BASE_URL.to_string()),
        model: record
            .and_then(|record| record.model.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.default_model.clone()))
            .unwrap_or_else(|| GEMINI_DEFAULT_MODEL.to_string()),
        api_format: record
            .map(|record| record.api_format.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.api_format.clone()))
            .unwrap_or_else(|| "openai".to_string()),
        has_api_key: record
            .and_then(|record| record.encrypted_secret_ref.as_ref())
            .is_some(),
        api_key_masked: record
            .and_then(|record| record.encrypted_secret_ref.as_ref())
            .map(|_| "********".to_string())
            .unwrap_or_default(),
        key_fingerprint: record.and_then(|record| record.key_fingerprint.clone()),
        session_only: record.map(|record| record.session_only).unwrap_or(false),
        last_checked_at: record.and_then(|record| record.last_checked_at.clone()),
        last_health_status: record.and_then(|record| record.last_health_status.clone()),
        updated_at: record.map(|record| record.updated_at.clone()),
    }
}

fn normalized_provider(provider: &str) -> Result<String, ApiError> {
    let provider = require_request_text(provider, "provider")?;
    if provider
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        Ok(provider)
    } else {
        Err(ApiError::bad_request(
            "provider contains unsupported characters",
        ))
    }
}

fn normalized_url(value: &str) -> Result<String, ApiError> {
    let value = require_request_text(value, "base_url")?;
    let value = value.trim_end_matches('/').to_string();
    if value.starts_with("https://") || value.starts_with("http://") {
        Ok(value)
    } else {
        Err(ApiError::bad_request(
            "base_url must start with http:// or https://",
        ))
    }
}

fn require_request_text(value: &str, field: &str) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApiError::bad_request(format!("{field} is required")));
    }

    Ok(value.to_string())
}

fn optional_api_key(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !value.chars().all(|ch| ch == '*'))
        .map(ToOwned::to_owned)
}

fn secret_fingerprint(secret: &str) -> String {
    let digest = digest(&SHA256, secret.as_bytes());
    STANDARD_NO_PAD.encode(&digest.as_ref()[..9])
}

fn seal_secret(config: &AppConfig, secret: &str) -> Result<String, ApiError> {
    let key_bytes = load_or_create_secret_key(config)?;
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| ApiError::internal("failed to prepare BYOK encryption key"))?;
    let sealing_key = aead::LessSafeKey::new(unbound_key);
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0_u8; 12];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| ApiError::internal("failed to generate BYOK encryption nonce"))?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let mut ciphertext = secret.as_bytes().to_vec();

    sealing_key
        .seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut ciphertext)
        .map_err(|_| ApiError::internal("failed to encrypt BYOK key"))?;

    Ok(format!(
        "{SECRET_CIPHERTEXT_PREFIX}:{}:{}",
        STANDARD_NO_PAD.encode(nonce_bytes),
        STANDARD_NO_PAD.encode(ciphertext)
    ))
}

fn open_secret(config: &AppConfig, value: &str) -> Result<String, ApiError> {
    let mut parts = value.split(':');
    let prefix = parts.next();
    let version = parts.next();
    let nonce = parts.next();
    let ciphertext = parts.next();
    if prefix != Some("ngenc") || version != Some("v1") || parts.next().is_some() {
        return Err(ApiError::internal("stored BYOK key format is unsupported"));
    }

    let nonce = nonce.ok_or_else(|| ApiError::internal("stored BYOK key nonce is missing"))?;
    let ciphertext =
        ciphertext.ok_or_else(|| ApiError::internal("stored BYOK ciphertext is missing"))?;
    let nonce_bytes = STANDARD_NO_PAD
        .decode(nonce)
        .map_err(|_| ApiError::internal("stored BYOK key nonce is invalid"))?;
    let mut nonce_array = [0_u8; 12];
    if nonce_bytes.len() != nonce_array.len() {
        return Err(ApiError::internal(
            "stored BYOK key nonce length is invalid",
        ));
    }
    nonce_array.copy_from_slice(&nonce_bytes);

    let mut ciphertext = STANDARD_NO_PAD
        .decode(ciphertext)
        .map_err(|_| ApiError::internal("stored BYOK ciphertext is invalid"))?;
    let key_bytes = load_or_create_secret_key(config)?;
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| ApiError::internal("failed to prepare BYOK encryption key"))?;
    let opening_key = aead::LessSafeKey::new(unbound_key);
    let plaintext = opening_key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_array),
            aead::Aad::empty(),
            &mut ciphertext,
        )
        .map_err(|_| ApiError::internal("failed to decrypt stored BYOK key"))?;

    String::from_utf8(plaintext.to_vec())
        .map_err(|_| ApiError::internal("stored BYOK key is not valid UTF-8"))
}

fn load_or_create_secret_key(config: &AppConfig) -> Result<[u8; 32], ApiError> {
    if let Some(secret) = &config.secrets_encryption_key {
        let digest = digest(&SHA256, secret.as_bytes());
        let mut key = [0_u8; 32];
        key.copy_from_slice(digest.as_ref());
        return Ok(key);
    }

    if config.secrets_key_path.exists() {
        let encoded = fs::read_to_string(&config.secrets_key_path)
            .map_err(|_| ApiError::internal("failed to read local BYOK encryption key"))?;
        let decoded = STANDARD_NO_PAD
            .decode(encoded.trim())
            .map_err(|_| ApiError::internal("local BYOK encryption key is invalid"))?;
        let mut key = [0_u8; 32];
        if decoded.len() != key.len() {
            return Err(ApiError::internal(
                "local BYOK encryption key length is invalid",
            ));
        }
        key.copy_from_slice(&decoded);
        return Ok(key);
    }

    if let Some(parent) = config.secrets_key_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| ApiError::internal("failed to create local secrets directory"))?;
    }

    let rng = SystemRandom::new();
    let mut key = [0_u8; 32];
    rng.fill(&mut key)
        .map_err(|_| ApiError::internal("failed to generate local BYOK encryption key"))?;
    fs::write(&config.secrets_key_path, STANDARD_NO_PAD.encode(key))
        .map_err(|_| ApiError::internal("failed to write local BYOK encryption key"))?;

    Ok(key)
}

async fn probe_byok_provider_key(
    provider: &str,
    base_url: &str,
    model: &str,
    api_key: &str,
) -> ByokProviderKeyHealth {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return byok_health(
                provider,
                base_url,
                model,
                false,
                None,
                "HTTP client setup failed",
            );
        }
    };
    let url = if provider == "gemini" {
        format!("{base_url}/models/{model}")
    } else {
        format!("{base_url}/models")
    };

    match client.get(url).bearer_auth(api_key).send().await {
        Ok(response) if response.status().is_success() => byok_health(
            provider,
            base_url,
            model,
            true,
            Some(response.status().as_u16()),
            "Provider accepted the API key",
        ),
        Ok(response) if matches!(response.status().as_u16(), 401 | 403) => byok_health(
            provider,
            base_url,
            model,
            false,
            Some(response.status().as_u16()),
            "Provider rejected the API key",
        ),
        Ok(response) => byok_health(
            provider,
            base_url,
            model,
            false,
            Some(response.status().as_u16()),
            "Provider returned a non-success status",
        ),
        Err(_) => byok_health(
            provider,
            base_url,
            model,
            false,
            None,
            "Provider health request failed",
        ),
    }
}

fn byok_health(
    provider: &str,
    base_url: &str,
    model: &str,
    valid: bool,
    status_code: Option<u16>,
    message: &str,
) -> ByokProviderKeyHealth {
    ByokProviderKeyHealth {
        provider: provider.to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        valid,
        status_code,
        message: message.to_string(),
        checked_at: unix_timestamp_label(),
    }
}

fn unix_timestamp_label() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix:{seconds}")
}

async fn project_realtime_ws(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| project_realtime_socket(socket, state, project_id))
}

async fn project_realtime_socket(mut socket: WebSocket, state: AppState, project_id: String) {
    let mut receiver = state.realtime_tx.subscribe();
    let connected = ProjectRealtimeEvent {
        project_id: project_id.clone(),
        event_type: "connected".to_string(),
        job_id: None,
        chapter_id: None,
        detail: "project realtime socket connected".to_string(),
    };

    if let Ok(payload) = serde_json::to_string(&connected) {
        if socket.send(Message::Text(payload.into())).await.is_err() {
            return;
        }
    }

    loop {
        match receiver.recv().await {
            Ok(event) if event.project_id == project_id => {
                let payload = match serde_json::to_string(&event) {
                    Ok(payload) => payload,
                    Err(_) => continue,
                };

                if socket.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(broadcast::error::RecvError::Lagged(_)) => {
                let lagged = ProjectRealtimeEvent {
                    project_id: project_id.clone(),
                    event_type: "resync_required".to_string(),
                    job_id: None,
                    chapter_id: None,
                    detail: "client lagged behind realtime event stream".to_string(),
                };
                if let Ok(payload) = serde_json::to_string(&lagged) {
                    if socket.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

fn publish_project_event(
    state: &AppState,
    project_id: &str,
    event_type: &str,
    job_id: Option<&str>,
    chapter_id: Option<&str>,
    detail: &str,
) {
    let _ = state.realtime_tx.send(ProjectRealtimeEvent {
        project_id: project_id.to_string(),
        event_type: event_type.to_string(),
        job_id: job_id.map(str::to_string),
        chapter_id: chapter_id.map(str::to_string),
        detail: detail.to_string(),
    });
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

async fn list_byok_providers() -> Json<Vec<ByokProviderPreset>> {
    Json(byok_provider_presets())
}

async fn get_byok_config(
    State(state): State<AppState>,
) -> Result<Json<ByokProviderConfigView>, ApiError> {
    let record = state.store.get_local_byok_provider_config().await?;
    Ok(Json(byok_config_view(record.as_ref())))
}

async fn save_byok_config(
    State(state): State<AppState>,
    Json(input): Json<SaveByokProviderConfigInput>,
) -> Result<Json<SaveByokProviderConfigResult>, ApiError> {
    let provider = normalized_provider(&input.provider)?;
    let preset = byok_provider_preset(&provider);
    let display_name = preset
        .as_ref()
        .map(|provider| provider.name.as_str())
        .unwrap_or(provider.as_str());
    let api_format = preset
        .as_ref()
        .map(|provider| provider.api_format.as_str())
        .unwrap_or("openai");
    let base_url = normalized_url(&input.base_url)?;
    let model = require_request_text(&input.model, "model")?;
    let api_key = optional_api_key(input.api_key);
    let (encrypted_secret_ref, key_fingerprint, saved_api_key) = if input.session_only {
        (None, None, false)
    } else if let Some(api_key) = api_key.as_deref() {
        (
            Some(seal_secret(&state.config, api_key)?),
            Some(secret_fingerprint(api_key)),
            true,
        )
    } else {
        (None, None, false)
    };

    let record = state
        .store
        .save_local_byok_provider_config(
            &provider,
            display_name,
            &base_url,
            &model,
            api_format,
            encrypted_secret_ref.as_deref(),
            key_fingerprint.as_deref(),
            input.session_only,
        )
        .await?;

    Ok(Json(SaveByokProviderConfigResult {
        config: byok_config_view(Some(&record)),
        saved_api_key,
    }))
}

async fn check_byok_key(
    State(state): State<AppState>,
    Json(input): Json<CheckByokProviderKeyInput>,
) -> Result<Json<ByokProviderKeyHealth>, ApiError> {
    let provider = normalized_provider(&input.provider)?;
    let base_url = normalized_url(&input.base_url)?;
    let model = require_request_text(&input.model, "model")?;
    let api_key = match optional_api_key(input.api_key) {
        Some(api_key) => api_key,
        None => {
            let record = state
                .store
                .get_local_byok_provider_config_for_provider(&provider)
                .await?;
            let encrypted_secret_ref = record
                .as_ref()
                .and_then(|record| record.encrypted_secret_ref.as_deref())
                .ok_or_else(|| ApiError::bad_request("API key is required"))?;
            open_secret(&state.config, encrypted_secret_ref)?
        }
    };

    let health = probe_byok_provider_key(&provider, &base_url, &model, &api_key).await;
    let status = if health.valid { "valid" } else { "invalid" };
    let _ = state
        .store
        .update_local_byok_provider_health(&provider, status)
        .await;

    Ok(Json(health))
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
    let project = state.store.create_project(&input.name).await?;
    publish_project_event(
        &state,
        &project.id,
        "project_created",
        None,
        None,
        "project created",
    );
    Ok(Json(project))
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
    let result = state
        .store
        .delete_project(&project_id, input.purge_data)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "project_deleted",
        None,
        None,
        "project deleted",
    );
    Ok(Json(result))
}

async fn restore_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<novelgraph_core::Project>, ApiError> {
    let project = state.store.restore_project(&project_id).await?;
    publish_project_event(
        &state,
        &project_id,
        "project_restored",
        None,
        None,
        "project restored",
    );
    Ok(Json(project))
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
    let latest_analysis_chapters = match &latest_analysis_job {
        Some(job) => {
            let runs = state
                .store
                .list_analysis_chapter_runs(&project_id, &job.id)
                .await?;
            let run_by_chapter = runs
                .iter()
                .map(|run| (run.chapter_id.as_str(), run))
                .collect::<HashMap<_, _>>();

            chapters
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
                        prompt_schema_version: run
                            .and_then(|run| run.prompt_schema_version.clone()),
                        error_code: run.and_then(|run| run.error_code.clone()),
                        error_message: run.and_then(|run| run.error_message.clone()),
                        started_at: run.and_then(|run| run.started_at.clone()),
                        finished_at: run.and_then(|run| run.finished_at.clone()),
                        updated_at: run.map(|run| run.updated_at.clone()),
                    }
                })
                .collect()
        }
        None => Vec::new(),
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
    let relationship_records = match &latest_analysis_job {
        Some(job) => {
            state
                .store
                .list_story_extraction_records(&project_id, &job.id, "relationship")
                .await?
        }
        None => Vec::new(),
    };
    let character_aliases = match &latest_analysis_job {
        Some(job) => {
            state
                .store
                .list_story_character_aliases(&project_id, &job.id)
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
        latest_analysis_chapters,
        latest_job_events,
        character_aliases,
        character_records,
        relationship_records,
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

async fn suggest_novel_import_metadata(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<NovelImportInput>,
) -> Result<Json<NovelMetadataSuggestion>, ApiError> {
    state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;
    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("novel text is required"));
    }

    Ok(Json(suggest_novel_metadata(&state, input).await?))
}

async fn confirm_novel_import(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<NovelImportInput>,
) -> Result<Json<novelgraph_core::NovelImportResult>, ApiError> {
    let result = state.store.import_novel(&project_id, input).await?;
    publish_project_event(
        &state,
        &project_id,
        "novel_imported",
        None,
        None,
        "novel imported",
    );
    Ok(Json(result))
}

async fn update_novel_metadata(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
    Json(mut input): Json<NovelMetadataUpdateInput>,
) -> Result<Json<novelgraph_core::Novel>, ApiError> {
    if input
        .source_language
        .as_deref()
        .map(str::trim)
        .is_none_or(|value| value.is_empty() || value == "auto")
    {
        let chapters = state.store.list_chapters(&project_id, &novel_id).await?;
        let text = chapters
            .iter()
            .map(|chapter| chapter.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        input.source_language = detect_basic_source_language(&text);
    }

    let novel = state
        .store
        .update_novel_metadata(&project_id, &novel_id, input)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "novel_metadata_updated",
        None,
        None,
        "novel metadata updated",
    );
    Ok(Json(novel))
}

async fn ai_fill_novel_metadata(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
) -> Result<Json<novelgraph_core::Novel>, ApiError> {
    let novel = state
        .store
        .get_novel(&project_id, &novel_id)
        .await?
        .ok_or(ApiError::not_found("novel"))?;
    let chapters = state.store.list_chapters(&project_id, &novel_id).await?;
    let text = chapters
        .iter()
        .map(|chapter| chapter.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.trim().is_empty() {
        return Err(ApiError::bad_request("novel text is required"));
    }

    let suggestion = suggest_novel_metadata(
        &state,
        NovelImportInput {
            title: novel.title.clone(),
            author: novel.author.clone(),
            source_language: novel.source_language.clone(),
            genre: novel.genre.clone(),
            description: novel.description.clone(),
            text,
        },
    )
    .await?;
    let updated = state
        .store
        .update_novel_metadata(
            &project_id,
            &novel_id,
            merge_novel_metadata_suggestion(&novel, suggestion),
        )
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "novel_metadata_updated",
        None,
        None,
        "novel metadata updated by AI",
    );
    Ok(Json(updated))
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

async fn suggest_novel_metadata(
    state: &AppState,
    mut input: NovelImportInput,
) -> Result<NovelMetadataSuggestion, ApiError> {
    if input
        .source_language
        .as_deref()
        .map(str::trim)
        .is_none_or(|value| value.is_empty() || value == "auto")
    {
        input.source_language = detect_basic_source_language(&input.text);
    }

    let prompt = build_novel_metadata_suggestion_prompt(&input);
    let (suggestions, _) =
        call_local_json_array::<NovelMetadataSuggestion>(state, &prompt, NOVEL_METADATA_MAX_TOKENS)
            .await?;
    let mut suggestion = suggestions
        .into_iter()
        .next()
        .unwrap_or(NovelMetadataSuggestion {
            title: None,
            author: None,
            source_language: None,
            genre: None,
            description: None,
            confidence: Some(0.0),
        });
    if suggestion
        .source_language
        .as_deref()
        .is_none_or(str::is_empty)
    {
        suggestion.source_language = input.source_language;
    }

    Ok(normalize_novel_metadata_suggestion(suggestion))
}

fn normalize_novel_metadata_suggestion(
    mut suggestion: NovelMetadataSuggestion,
) -> NovelMetadataSuggestion {
    suggestion.title = optional_metadata_text(suggestion.title);
    suggestion.author = optional_metadata_text(suggestion.author);
    suggestion.source_language =
        optional_metadata_text(suggestion.source_language).filter(|value| value != "auto");
    suggestion.genre = optional_metadata_text(suggestion.genre);
    suggestion.description = optional_metadata_text(suggestion.description);
    suggestion
}

fn merge_novel_metadata_suggestion(
    novel: &novelgraph_core::Novel,
    suggestion: NovelMetadataSuggestion,
) -> NovelMetadataUpdateInput {
    NovelMetadataUpdateInput {
        title: suggestion.title.or_else(|| Some(novel.title.clone())),
        author: suggestion.author.or_else(|| novel.author.clone()),
        source_language: suggestion
            .source_language
            .or_else(|| novel.source_language.clone()),
        genre: suggestion.genre.or_else(|| novel.genre.clone()),
        description: suggestion.description.or_else(|| novel.description.clone()),
    }
}

fn optional_metadata_text(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
    let job = state
        .store
        .create_translation_job(&project_id, input)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "translation_job_created",
        Some(&job.id),
        None,
        "translation job created",
    );
    Ok(Json(job))
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
    publish_project_event(
        &state,
        &project_id,
        "analysis_reset",
        Some(&job_id),
        None,
        "analysis run reset",
    );

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
    publish_project_event(
        &state,
        &project_id,
        "analysis_paused",
        Some(&job_id),
        None,
        "analysis run paused",
    );

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
        publish_project_event(
            &state,
            &project_id,
            "analysis_reset",
            Some(&job_id),
            None,
            "analysis run reset before force run",
        );
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
    publish_project_event(
        &state,
        &project_id,
        "analysis_running",
        Some(&job_id),
        None,
        "analysis job marked running",
    );
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
        publish_project_event(
            &state,
            &project_id,
            "analysis_finished",
            Some(&job_id),
            None,
            "analysis range or job finished",
        );
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
        publish_project_event(
            &state,
            &project_id,
            "analysis_paused",
            Some(&job_id),
            None,
            "local LLM unreachable",
        );

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
    publish_project_event(
        &state,
        &project_id,
        "analysis_chapter_started",
        Some(&job_id),
        Some(&chapter.id),
        "analysis chapter started",
    );

    let chunks = split_chapter_for_character_extraction(&chapter.content);
    let mut working_document = StoryExtractionDocument {
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION.to_string(),
        chapter_num: chapter.chapter_num,
        records: Vec::new(),
    };
    let mut chunk_outputs = Vec::with_capacity(chunks.len());
    let novel_analysis_context = build_current_novel_analysis_context(&novel);

    for (index, chunk) in chunks.iter().enumerate() {
        if analysis_job_should_stop(&state.store, &project_id, &job_id).await? {
            return Ok(Json(
                build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
            ));
        }

        let base_prior_context = format!(
            "{novel_analysis_context}\n\nĐây là đoạn nhỏ {}/{} của chương hiện tại. Mỗi pass chỉ xử lý dữ liệu có trong đoạn này. Offset mention phải tính từ CHAPTER_TEXT của đoạn này; backend sẽ tự quy đổi về toàn chương.",
            index + 1,
            chunks.len()
        );
        let db_records_before_identity = state
            .store
            .list_story_extraction_records(&project_id, &job_id, "character")
            .await?;
        let db_aliases_before_identity = state
            .store
            .list_story_character_aliases(&project_id, &job_id)
            .await?;
        let known_alias_identity_hints =
            known_alias_map_identities_for_chunk(&chunk.text, &db_aliases_before_identity);
        let known_alias_context =
            serde_json::to_string(&known_alias_identity_hints).unwrap_or_else(|_| "[]".to_string());
        let candidate_input = DraftExtractionInput {
            chapter_num: chapter.chapter_num,
            title: Some(chapter.title.clone()),
            source_language: novel.source_language.clone(),
            text: chunk.text.clone(),
            prior_context: Some(format!(
                "{base_prior_context}\n\nKnown character/alias surfaces already present in this chunk from previous chapters:\n{known_alias_context}\n\nNếu một known surface xuất hiện trong CHAPTER_TEXT, ưu tiên giữ node canonical đã biết thay vì tạo node mới cho cùng người."
            )),
        };
        let candidate_prompt = build_character_candidate_prompt(&candidate_input);
        let (candidate_identities, candidate_response) =
            match call_local_json_array::<CharacterCandidate>(
                &state,
                &candidate_prompt,
                CHARACTER_CANDIDATE_MAX_TOKENS,
            )
            .await
            {
                Ok((candidates, response)) => {
                    (normalize_character_candidates(candidates), json!(response))
                }
                Err(error) => (
                    Vec::new(),
                    json!({
                        "mode": "candidate_pass_failed_non_blocking",
                        "error": error.message,
                    }),
                ),
            };
        let candidate_context =
            serde_json::to_string(&candidate_identities).unwrap_or_else(|_| "[]".to_string());
        let chunk_input = DraftExtractionInput {
            chapter_num: chapter.chapter_num,
            title: Some(chapter.title.clone()),
            source_language: novel.source_language.clone(),
            text: chunk.text.clone(),
            prior_context: Some(format!(
                "{base_prior_context}\n\nKnown character/alias surfaces already present in this chunk from previous chapters:\n{known_alias_context}\n\nCandidate coverage checklist từ pass quét nhanh trước identity:\n{candidate_context}\n\nIdentity pass phải dùng checklist này để tránh sót tên/alias có evidence trong đoạn, nhưng vẫn chỉ ghi nhân vật/alias khi CHAPTER_TEXT hiện tại hỗ trợ."
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
        let mut chunk_identities = normalize_character_identities(chunk_identities);
        merge_character_identity_hints(&mut chunk_identities, candidate_identities.clone());
        merge_character_identity_hints(&mut chunk_identities, known_alias_identity_hints.clone());
        filter_substring_only_identities(&mut chunk_identities, &chunk.text);
        let identity_nodes_json =
            serde_json::to_string(&chunk_identities).unwrap_or_else(|_| "[]".to_string());
        let quoted_alias_candidates =
            quoted_alias_candidate_context(&chunk_identities, &chunk.text);
        let quoted_alias_context =
            serde_json::to_string(&quoted_alias_candidates).unwrap_or_else(|_| "[]".to_string());
        let alias_ownership_input = DraftExtractionInput {
            chapter_num: chapter.chapter_num,
            title: Some(chapter.title.clone()),
            source_language: novel.source_language.clone(),
            text: chunk.text.clone(),
            prior_context: Some(format!(
                "{base_prior_context}\n\nIdentity/Candidate pass đã tạo danh sách node nhân vật bên dưới. Alias Ownership pass là bước duy nhất được nhập surface alias/coreference vào owner trước khi resolver xuyên chương chạy.\n\nQuoted surface checklist do backend chỉ quét hình thức ngoặc kép, không tự kết luận owner:\n{quoted_alias_context}\n\nDùng checklist này để xét kỹ các surface trong ngoặc kép, nhưng chỉ trả alias ownership khi CHAPTER_TEXT chứng minh bằng ngữ pháp/ngữ nghĩa."
            )),
        };
        let alias_ownership_prompt =
            build_character_alias_ownership_prompt(&alias_ownership_input, &identity_nodes_json);
        let (alias_ownerships, alias_ownership_response) =
            match call_local_json_array::<CharacterAliasOwnership>(
                &state,
                &alias_ownership_prompt,
                CHARACTER_ALIAS_OWNERSHIP_MAX_TOKENS,
            )
            .await
            {
                Ok((ownerships, response)) => (ownerships, json!(response)),
                Err(error) => (
                    Vec::new(),
                    json!({
                        "mode": "alias_ownership_pass_failed_non_blocking",
                        "error": error.message,
                    }),
                ),
            };
        let alias_ownership_applications = apply_character_alias_ownerships(
            &mut chunk_identities,
            alias_ownerships,
            chapter.chapter_num,
        );
        filter_substring_only_identities(&mut chunk_identities, &chunk.text);
        let (chunk_identities, merge_decision_outputs) =
            resolve_character_identities_across_chapters(
                &state,
                &chunk_input,
                chunk_identities,
                &db_records_before_identity,
                &db_aliases_before_identity,
                &working_document,
            )
            .await;
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
        publish_project_event(
            &state,
            &project_id,
            "story_extraction_updated",
            Some(&job_id),
            Some(&chapter.id),
            "character identity chunk persisted",
        );

        let db_records = state
            .store
            .list_story_extraction_records(&project_id, &job_id, "character")
            .await?;
        let current_identities = hydrate_identities_with_alias_map(
            working_identities_for_chunk(&working_document, &chunk_identities),
            &db_aliases_before_identity,
        );
        let mut character_passes = Vec::new();

        for identity in current_identities {
            if analysis_job_should_stop(&state.store, &project_id, &job_id).await? {
                return Ok(Json(
                    build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
                ));
            }

            let character_json =
                serde_json::to_string(&identity).unwrap_or_else(|_| "{}".to_string());

            let (mentions, mentions_response) = match scan_character_mentions_with_backend(
                &state,
                &chunk_input,
                &identity,
                &character_json,
                &chapter.content,
            )
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    let reason = format!(
                        "character backend mention scan chunk {}/{} for {} failed: {}",
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
                        "character_mentions_backend_scan_failed",
                        reason,
                    )
                    .await;
                }
            };
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
            publish_project_event(
                &state,
                &project_id,
                "story_extraction_updated",
                Some(&job_id),
                Some(&chapter.id),
                "character mentions chunk persisted",
            );

            let field_contexts = build_character_field_contexts(&chunk.text, &identity);
            let mut field_input_for_verification: Option<DraftExtractionInput> = None;
            let (fields, fields_response) = if field_contexts.is_empty() {
                (
                    Vec::new(),
                    json!({
                        "mode": "skipped_no_target_context",
                        "target": identity.name,
                    }),
                )
            } else {
                let field_input = DraftExtractionInput {
                    chapter_num: chunk_input.chapter_num,
                    title: chunk_input.title.clone(),
                    source_language: chunk_input.source_language.clone(),
                    text: field_contexts.join("\n---\n"),
                    prior_context: Some(format!(
                        "{novel_analysis_context}\n\nĐây là TARGET_CONTEXTS đã được backend chọn từ đoạn nhỏ {}/{} của chương hiện tại. Các occurrence của target được đánh dấu bằng [[...]]. Fields pass chỉ được dùng các context này, không dùng toàn chunk.",
                        index + 1,
                        chunks.len()
                    )),
                };
                field_input_for_verification = Some(field_input.clone());
                let fields_prompt = build_character_fields_prompt(&field_input, &character_json);
                let (fields, response) = match call_local_json_array::<StoryExtractionFieldPayload>(
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
                (
                    fields,
                    json!({
                        "mode": "target_marked_contexts",
                        "context_count": field_contexts.len(),
                        "contexts": field_contexts,
                        "response": response,
                    }),
                )
            };
            let normalized_fields = normalize_character_field_payloads(
                fields,
                &identity,
                &db_records,
                &working_document,
            );
            let (fields, field_verification_report) = verify_character_field_payloads(
                &state,
                field_input_for_verification.as_ref(),
                &identity,
                &character_json,
                normalized_fields,
            )
            .await;
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
            publish_project_event(
                &state,
                &project_id,
                "story_extraction_updated",
                Some(&job_id),
                Some(&chapter.id),
                "character fields chunk persisted",
            );

            character_passes.push(json!({
                "name": identity.name,
                "aliases": identity.aliases,
                "mentions_response": mentions_response,
                "fields_response": fields_response,
                "field_verification": field_verification_report,
            }));
        }

        chunk_outputs.push(json!({
            "chunk_index": index + 1,
            "chunk_count": chunks.len(),
            "start_char": chunk.start_char,
            "end_char": chunk.end_char,
            "candidate_response": candidate_response,
            "candidate_identity_hints": candidate_identities,
            "known_alias_identity_hints": known_alias_identity_hints,
            "identity_response": identity_response,
            "alias_ownership_response": alias_ownership_response,
            "alias_ownership_applications": alias_ownership_applications,
            "merge_decisions": merge_decision_outputs,
            "character_passes": character_passes,
        }));

        if analysis_job_should_stop(&state.store, &project_id, &job_id).await? {
            return Ok(Json(
                build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
            ));
        }
    }

    let relationship_outputs = match extract_character_relationships_for_chapter(
        &state,
        &project_id,
        &job_id,
        &chapter,
        &novel,
        &mut working_document,
    )
    .await
    {
        Ok(outputs) => outputs,
        Err(error) => {
            let reason = format!("character relationship pass failed: {}", error.message);
            return fail_analysis_chapter_and_pause(
                &state,
                &project_id,
                &job_id,
                &chapter_run.chapter_id,
                "character_relationship_pass_failed",
                reason,
            )
            .await;
        }
    };

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
        publish_project_event(
            &state,
            &project_id,
            "analysis_paused",
            Some(&job_id),
            Some(&chapter.id),
            "character extraction validation failed",
        );

        return Ok(Json(
            build_analysis_run_snapshot(&state.store, &project_id, &job_id, Some(reason)).await?,
        ));
    }

    let output_json = json!({
        "schema_version": CHARACTER_EXTRACTION_SCHEMA_VERSION,
        "extraction_mode": "staged_chunked_character_backend_mention_scan",
        "chunk_target_chars": CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS,
        "chunk_count": chunks.len(),
        "chunks": chunk_outputs,
        "relationship_passes": relationship_outputs,
        "persisted": true,
        "persisted_group_keys": ["character", "relationship"],
        "character_record_count": character_extraction
            .records
            .iter()
            .filter(|record| record.group_key == "character")
            .count(),
        "relationship_record_count": character_extraction
            .records
            .iter()
            .filter(|record| record.group_key == "relationship")
            .count(),
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
    publish_project_event(
        &state,
        &project_id,
        "analysis_chapter_completed",
        Some(&job_id),
        Some(&chapter.id),
        "analysis chapter completed",
    );

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
    publish_project_event(
        &state,
        &project_id,
        "analysis_finished",
        Some(&job_id),
        None,
        "analysis range or job progress updated",
    );

    Ok(Json(
        build_analysis_run_snapshot(&state.store, &project_id, &job_id, None).await?,
    ))
}

async fn cancel_analysis_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<novelgraph_core::AnalysisJob>, ApiError> {
    let job = state
        .store
        .cancel_analysis_job(&project_id, &job_id)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "analysis_cancelled",
        Some(&job_id),
        None,
        "analysis job cancelled",
    );
    Ok(Json(job))
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
    let job = state
        .store
        .cancel_translation_job(&project_id, &job_id)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "translation_job_cancelled",
        Some(&job_id),
        None,
        "translation job cancelled",
    );
    Ok(Json(job))
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
    publish_project_event(
        state,
        project_id,
        "analysis_paused",
        Some(job_id),
        Some(chapter_id),
        error_code,
    );

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
    match parse_json_array_response::<T>(&response) {
        Ok(items) => Ok((items, response)),
        Err(parse_error) => {
            let repair_response =
                repair_local_json_array_response(state, prompt, &response, max_tokens).await?;
            match parse_json_array_response::<T>(&repair_response) {
                Ok(items) => Ok((items, repair_response)),
                Err(retry_error) => Err(ApiError::bad_request(format!(
                    "{}; repair retry failed: {}",
                    parse_error.message, retry_error.message
                ))),
            }
        }
    }
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

    parse_json_array_text(json_text)
}

fn parse_json_array_text<T>(json_text: &str) -> Result<Vec<T>, ApiError>
where
    T: DeserializeOwned,
{
    match serde_json::from_str::<Vec<T>>(json_text) {
        Ok(items) => Ok(items),
        Err(initial_error) => {
            let sanitized = escape_control_chars_inside_json_strings(json_text);
            if sanitized == json_text {
                return Err(ApiError::bad_request(format!(
                    "local LLM JSON array parse failed: {initial_error}"
                )));
            }

            serde_json::from_str::<Vec<T>>(&sanitized).map_err(|retry_error| {
                ApiError::bad_request(format!(
                    "local LLM JSON array parse failed: {initial_error}; sanitized parse failed: {retry_error}"
                ))
            })
        }
    }
}

fn escape_control_chars_inside_json_strings(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_string = false;
    let mut escaped = false;

    for ch in value.chars() {
        if !in_string {
            output.push(ch);
            if ch == '"' {
                in_string = true;
            }
            continue;
        }

        if escaped {
            output.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                output.push(ch);
                escaped = true;
            }
            '"' => {
                output.push(ch);
                in_string = false;
            }
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => {
                output.push_str(&format!("\\u{:04x}", ch as u32));
            }
            _ => output.push(ch),
        }
    }

    output
}

async fn repair_local_json_array_response(
    state: &AppState,
    prompt: &DraftExtractionPrompt,
    invalid_response: &ChatCompletionResponse,
    max_tokens: u32,
) -> Result<ChatCompletionResponse, ApiError> {
    let invalid_content = response_message_content(invalid_response);
    let repair_input = truncate_chars(&invalid_content, LOCAL_JSON_REPAIR_INPUT_MAX_CHARS);

    state
        .local_llm
        .chat_completion(ChatCompletionRequest {
            model: None,
            messages: vec![
                ChatMessage {
                    role: LlmRole::System,
                    content: "You repair invalid JSON array output. Return valid JSON only. Do not add new facts. Do not explain.".to_string(),
                },
                ChatMessage {
                    role: LlmRole::User,
                    content: format!(
                        "Schema version: {}\n\nThe previous response was intended to be a JSON array but failed parsing. Repair only syntax problems such as raw control characters inside strings, missing escaping, trailing text, or malformed commas. Preserve the same array items and fields as much as possible. If an item cannot be repaired safely, remove that item. Return a JSON array directly.\n\nInvalid response:\n<<<INVALID_JSON\n{}\nINVALID_JSON",
                        prompt.schema_version,
                        repair_input
                    ),
                },
            ],
            temperature: Some(0.0),
            max_tokens: Some(max_tokens),
            chat_template_kwargs: Some(json!({ "enable_thinking": false })),
            stream: false,
        })
        .await
        .map_err(ApiError::from)
}

fn response_message_content(response: &ChatCompletionResponse) -> String {
    response
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .unwrap_or_default()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("\n...[truncated]");
    }
    output
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
    let relationship_records = store
        .list_story_extraction_records(project_id, job_id, "relationship")
        .await?;
    let character_aliases = store
        .list_story_character_aliases(project_id, job_id)
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
        character_aliases,
        character_records,
        relationship_records,
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
    aliases: Vec<CharacterAlias>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterCandidate {
    #[serde(default)]
    surface_text: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    role_label: String,
    #[serde(default)]
    aliases: Vec<CharacterAlias>,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "CharacterAliasWire")]
struct CharacterAlias {
    text: String,
    alias_type: String,
    alias_label: String,
    #[serde(default)]
    is_primary: bool,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum CharacterAliasWire {
    Text(String),
    Object {
        text: String,
        #[serde(default)]
        alias_type: String,
        #[serde(default)]
        alias_label: String,
        #[serde(default)]
        is_primary: bool,
        #[serde(default)]
        evidence: Vec<StoryEvidenceSpan>,
    },
}

impl From<CharacterAliasWire> for CharacterAlias {
    fn from(value: CharacterAliasWire) -> Self {
        match value {
            CharacterAliasWire::Text(text) => CharacterAlias {
                text,
                alias_type: "other_alias".to_string(),
                alias_label: "Tên gọi khác".to_string(),
                is_primary: false,
                evidence: Vec::new(),
            },
            CharacterAliasWire::Object {
                text,
                alias_type,
                alias_label,
                is_primary,
                evidence,
            } => CharacterAlias {
                text,
                alias_type,
                alias_label,
                is_primary,
                evidence,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterAliasOwnership {
    #[serde(default)]
    owner_name: String,
    #[serde(default)]
    alias_text: String,
    #[serde(default)]
    alias_type: String,
    #[serde(default)]
    alias_label: String,
    confidence: Option<f64>,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterOccurrenceConfirmation {
    is_character_mention: bool,
    confidence: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterFieldValueVerification {
    #[serde(default)]
    accepted: bool,
    #[serde(default)]
    semantic_class: String,
    #[serde(default)]
    owner_name: Option<String>,
    confidence: Option<f64>,
    reason: Option<String>,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterRelationshipExtraction {
    relationship_type: String,
    a_to_b_label: String,
    b_to_a_label: String,
    confidence: Option<f64>,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterRelationshipCandidate {
    #[serde(default, alias = "person_a", alias = "character_a", alias = "source")]
    source_name: String,
    #[serde(default, alias = "person_b", alias = "character_b", alias = "target")]
    target_name: String,
    #[serde(default, alias = "kind")]
    relationship_kind: String,
    #[serde(default)]
    relationship_type: String,
    #[serde(
        default,
        alias = "a_to_b_label",
        alias = "source_label",
        alias = "relationship_label"
    )]
    source_to_target_label: String,
    #[serde(default, alias = "b_to_a_label", alias = "target_label")]
    target_to_source_label: String,
    confidence: Option<f64>,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterRelationshipVerification {
    #[serde(default)]
    accepted: bool,
    #[serde(default)]
    relationship_scope: String,
    #[serde(default)]
    owner_direction_ok: bool,
    confidence: Option<f64>,
    reason: Option<String>,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Serialize)]
struct CharacterIdentityMergeCandidate {
    target_key: String,
    display_name: String,
    aliases: Vec<CharacterAlias>,
    score: f64,
    source: String,
    chapter_num: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterIdentityMergeDecision {
    #[serde(default)]
    action: String,
    confidence: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterIdentityCreationDecision {
    #[serde(default)]
    action: String,
    #[serde(default)]
    target_key: Option<String>,
    #[serde(default)]
    target_name: Option<String>,
    confidence: Option<f64>,
    reason: Option<String>,
    #[serde(default)]
    evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Serialize)]
struct ScannedCharacterOccurrence {
    text: String,
    start_char: i64,
    end_char: i64,
    mention_type: String,
    ambiguous: bool,
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

fn normalize_character_identities(identities: Vec<CharacterIdentity>) -> Vec<CharacterIdentity> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for identity in identities {
        let name = clean_character_surface(&identity.name);
        if name.is_empty() {
            continue;
        }

        let key = normalized_text_key(&name);
        if seen.insert(key) {
            normalized.push(CharacterIdentity {
                name,
                aliases: Vec::new(),
            });
        }
    }

    normalized
}

fn normalize_character_candidates(candidates: Vec<CharacterCandidate>) -> Vec<CharacterIdentity> {
    let mut identities = Vec::new();

    for candidate in candidates {
        if !character_candidate_kind_is_allowed(&candidate.kind) {
            continue;
        }

        let name = clean_character_surface(
            [
                candidate.display_name.as_str(),
                candidate.surface_text.as_str(),
            ]
            .into_iter()
            .find(|value| !value.trim().is_empty())
            .unwrap_or(""),
        );
        if name.is_empty() {
            continue;
        }

        identities.push(CharacterIdentity {
            name,
            aliases: Vec::new(),
        });
    }

    normalize_character_identities(identities)
}

fn character_candidate_kind_is_allowed(kind: &str) -> bool {
    let key = normalize_ascii_snake_key(kind);
    key.is_empty()
        || matches!(
            key.as_str(),
            "person" | "character" | "nhan_vat" | "nhanvat"
        )
}

fn merge_character_identity_hints(
    identities: &mut Vec<CharacterIdentity>,
    hints: Vec<CharacterIdentity>,
) {
    let mut existing_name_keys = identities
        .iter()
        .map(|identity| normalized_text_key(&identity.name))
        .collect::<std::collections::HashSet<_>>();

    for hint in hints {
        let name = clean_character_surface(&hint.name);
        let name_key = normalized_text_key(&name);
        if name_key.is_empty() || !existing_name_keys.insert(name_key) {
            continue;
        }

        identities.push(CharacterIdentity {
            name,
            aliases: Vec::new(),
        });
    }
}

fn filter_substring_only_identities(identities: &mut Vec<CharacterIdentity>, chunk_text: &str) {
    if identities.len() < 2 {
        return;
    }

    let identity_names = identities
        .iter()
        .map(|identity| clean_character_surface(&identity.name))
        .collect::<Vec<_>>();
    let mut remove_keys = std::collections::HashSet::new();

    for (index, name) in identity_names.iter().enumerate() {
        let key = normalized_text_key(name);
        if key.is_empty() {
            continue;
        }

        let occurrences = find_surface_occurrences(chunk_text, name, "identity", false);
        if occurrences.is_empty() {
            continue;
        }

        let mut containing_occurrences = Vec::new();
        for (other_index, other_name) in identity_names.iter().enumerate() {
            if index == other_index
                || !character_surface_key_contains_other_surface(other_name, name)
            {
                continue;
            }

            containing_occurrences.extend(find_surface_occurrences(
                chunk_text,
                other_name,
                "identity_container",
                false,
            ));
        }

        if containing_occurrences.is_empty() {
            continue;
        }

        let only_inside_longer_surface = occurrences.iter().all(|occurrence| {
            containing_occurrences.iter().any(|container| {
                container.start_char <= occurrence.start_char
                    && container.end_char >= occurrence.end_char
                    && (container.end_char - container.start_char)
                        > (occurrence.end_char - occurrence.start_char)
            })
        });

        if only_inside_longer_surface {
            remove_keys.insert(key);
        }
    }

    if !remove_keys.is_empty() {
        identities.retain(|identity| !remove_keys.contains(&normalized_text_key(&identity.name)));
    }
}

fn character_surface_key_contains_other_surface(longer: &str, shorter: &str) -> bool {
    let longer_key = normalized_folded_text_key(longer);
    let shorter_key = normalized_folded_text_key(shorter);
    !longer_key.is_empty()
        && !shorter_key.is_empty()
        && longer_key != shorter_key
        && longer_key.contains(&shorter_key)
}

fn known_alias_map_identities_for_chunk(
    chunk_text: &str,
    db_aliases: &[StoryCharacterAliasView],
) -> Vec<CharacterIdentity> {
    let mut identities = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for candidate in character_alias_map_candidates(db_aliases) {
        let mut surfaces = Vec::new();
        surfaces.push(candidate.display_name.clone());
        surfaces.extend(candidate.aliases.iter().map(|alias| alias.text.clone()));

        let appears_in_chunk = surfaces.iter().any(|surface| {
            !surface.trim().is_empty()
                && !find_surface_occurrences(chunk_text, surface, "known_alias", false).is_empty()
        });
        if !appears_in_chunk || !seen.insert(candidate.target_key.clone()) {
            continue;
        }

        identities.push(CharacterIdentity {
            name: candidate.display_name,
            aliases: candidate.aliases,
        });
    }

    identities
}

fn apply_character_alias_ownerships(
    identities: &mut Vec<CharacterIdentity>,
    ownerships: Vec<CharacterAliasOwnership>,
    chapter_num: i64,
) -> Vec<serde_json::Value> {
    let mut applications = Vec::new();
    let mut remove_identity_keys = std::collections::HashSet::new();

    for ownership in ownerships {
        let confidence = ownership.confidence.unwrap_or(0.0);
        if confidence < CHARACTER_ALIAS_OWNERSHIP_MIN_CONFIDENCE {
            continue;
        }

        let owner_name = clean_character_surface(&ownership.owner_name);
        let alias_text = clean_character_surface(&ownership.alias_text);
        let owner_key = normalized_text_key(&owner_name);
        let alias_key = normalized_text_key(&alias_text);
        if owner_key.is_empty()
            || alias_key.is_empty()
            || owner_key == alias_key
            || remove_identity_keys.contains(&owner_key)
        {
            continue;
        }

        let Some(mut owner_index) =
            find_character_identity_index_by_surface(identities, &owner_name)
        else {
            continue;
        };
        if let Some(redirected_owner_index) =
            better_alias_owner_by_surface(identities, owner_index, &alias_text)
        {
            owner_index = redirected_owner_index;
        }
        let target_name = identities[owner_index].name.clone();
        let target_key = normalized_text_key(&target_name);
        if remove_identity_keys.contains(&target_key) {
            continue;
        }

        let alias_type = normalize_character_alias_type(&ownership.alias_type);
        if !is_persistable_character_alias_type(&alias_type) {
            continue;
        }
        let alias_label = normalize_character_alias_label(&alias_type, &ownership.alias_label);
        let evidence = normalize_alias_ownership_evidence(ownership.evidence, chapter_num);
        if !alias_ownership_can_be_applied(identities, owner_index, &alias_text, &evidence) {
            applications.push(json!({
                "mode": "alias_ownership",
                "applied": false,
                "reason": "alias owner is not grounded by evidence",
                "owner_name": target_name,
                "alias_text": alias_text,
                "alias_type": alias_type,
                "alias_label": alias_label,
                "confidence": confidence,
            }));
            continue;
        }

        push_character_alias_if_valid(
            &mut identities[owner_index].aliases,
            CharacterAlias {
                text: alias_text.clone(),
                alias_type: alias_type.clone(),
                alias_label: alias_label.clone(),
                is_primary: confidence >= 0.95,
                evidence,
            },
            &target_name,
        );

        if identities.iter().enumerate().any(|(index, identity)| {
            index != owner_index && normalized_text_key(&identity.name) == alias_key
        }) {
            remove_identity_keys.insert(alias_key.clone());
        }

        applications.push(json!({
            "mode": "alias_ownership",
            "applied": true,
            "owner_name": target_name,
            "alias_text": alias_text,
            "alias_type": alias_type,
            "alias_label": alias_label,
            "confidence": confidence,
        }));
    }

    if !remove_identity_keys.is_empty() {
        identities.retain(|identity| {
            !remove_identity_keys.contains(&normalized_text_key(&identity.name))
        });
    }

    applications
}

#[derive(Debug, Clone)]
struct QuotedAliasSpan {
    start_char: i64,
    end_char: i64,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct QuotedAliasCandidateContext {
    surface: String,
    context: String,
    nearby_identity_names: Vec<String>,
    nearest_identity_before_quote: Option<String>,
}

fn quoted_alias_candidate_context(
    identities: &[CharacterIdentity],
    chunk_text: &str,
) -> Vec<QuotedAliasCandidateContext> {
    let mut candidates = Vec::new();

    for span in quoted_alias_spans(chunk_text) {
        let surface = clean_character_surface(&span.text);
        if surface.is_empty() {
            continue;
        }

        let sentence_start = sentence_start_before(chunk_text, span.start_char);
        let sentence_end = sentence_end_after(chunk_text, span.end_char);
        let context = slice_text_by_char_range(chunk_text, sentence_start, sentence_end);
        let nearby_identity_names = identity_names_in_text(identities, &context);
        if nearby_identity_names.is_empty() {
            continue;
        }

        let before_quote = slice_text_by_char_range(chunk_text, sentence_start, span.start_char);
        let nearest_identity_before_quote =
            nearest_identity_before_alias_quote(identities, &before_quote)
                .map(|identity_index| identities[identity_index].name.clone());

        candidates.push(QuotedAliasCandidateContext {
            surface,
            context,
            nearby_identity_names,
            nearest_identity_before_quote,
        });
    }

    candidates
}

fn identity_names_in_text(identities: &[CharacterIdentity], text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for identity in identities {
        let identity_key = normalized_text_key(&identity.name);
        if identity_key.is_empty() || seen.contains(&identity_key) {
            continue;
        }

        let found = character_identity_surfaces(identity)
            .into_iter()
            .any(|(surface, _)| {
                !find_surface_occurrences(text, &surface, "alias_owner_candidate", false).is_empty()
            });
        if found {
            seen.insert(identity_key);
            names.push(identity.name.clone());
        }
    }

    names
}

fn quoted_alias_spans(text: &str) -> Vec<QuotedAliasSpan> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut spans = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        if !is_quote_char(chars[index]) {
            index += 1;
            continue;
        }

        let quote_start = index;
        let mut quote_end = quote_start + 1;
        while quote_end < chars.len() && !is_quote_char(chars[quote_end]) {
            quote_end += 1;
        }

        if quote_end >= chars.len() {
            break;
        }

        let quoted = chars[(quote_start + 1)..quote_end]
            .iter()
            .collect::<String>();
        let cleaned = clean_character_surface(&quoted);
        if is_stable_character_alias_surface(&quoted, &cleaned) {
            spans.push(QuotedAliasSpan {
                start_char: quote_start as i64,
                end_char: (quote_end + 1) as i64,
                text: quoted,
            });
        }

        index = quote_end + 1;
    }

    spans
}

fn nearest_identity_before_alias_quote(
    identities: &[CharacterIdentity],
    before_quote: &str,
) -> Option<usize> {
    let mut best: Option<(usize, i64)> = None;

    for (identity_index, identity) in identities.iter().enumerate() {
        for (surface, _) in character_identity_surfaces(identity) {
            for occurrence in
                find_surface_occurrences(before_quote, &surface, "alias_owner_candidate", false)
            {
                match best {
                    Some((_, best_end)) if occurrence.end_char <= best_end => {}
                    _ => best = Some((identity_index, occurrence.end_char)),
                }
            }
        }
    }

    best.map(|(identity_index, _)| identity_index)
}

fn sentence_start_before(text: &str, char_index: i64) -> i64 {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = char_index.max(0).min(chars.len() as i64) as usize;
    while index > 0 {
        let previous = chars[index - 1];
        if is_sentence_boundary_for_quote_context(previous) {
            break;
        }
        index -= 1;
    }

    index as i64
}

fn sentence_end_after(text: &str, char_index: i64) -> i64 {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = char_index.max(0).min(chars.len() as i64) as usize;
    while index < chars.len() {
        let current = chars[index];
        index += 1;
        if is_sentence_boundary_for_quote_context(current) {
            break;
        }
    }

    index as i64
}

fn is_sentence_boundary_for_quote_context(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?' | '。' | '！' | '？' | '\n' | '\r')
}

fn slice_text_by_char_range(text: &str, start_char: i64, end_char: i64) -> String {
    let start = start_char.max(0) as usize;
    let end = end_char.max(start_char).max(0) as usize;
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

fn normalize_alias_ownership_evidence(
    evidence: Vec<StoryEvidenceSpan>,
    chapter_num: i64,
) -> Vec<StoryEvidenceSpan> {
    evidence
        .into_iter()
        .map(|span| StoryEvidenceSpan {
            chapter_num,
            start_char: None,
            end_char: None,
            quote: span.quote,
            reason: span.reason,
        })
        .collect()
}

fn alias_ownership_can_be_applied(
    identities: &[CharacterIdentity],
    owner_index: usize,
    alias_text: &str,
    evidence: &[StoryEvidenceSpan],
) -> bool {
    if evidence.is_empty() || !alias_evidence_mentions_surface(evidence, alias_text) {
        return false;
    }

    let owner = &identities[owner_index];
    let owner_surfaces = character_identity_surfaces(owner)
        .into_iter()
        .map(|(surface, _)| surface)
        .collect::<Vec<_>>();
    if evidence_mentions_any_surface(evidence, &owner_surfaces) {
        return true;
    }

    if owner_surfaces
        .iter()
        .any(|surface| character_surfaces_share_distinctive_token(alias_text, surface))
    {
        return true;
    }

    false
}

fn alias_evidence_mentions_surface(evidence: &[StoryEvidenceSpan], surface: &str) -> bool {
    let surface_key = normalized_folded_text_key(surface);
    if surface_key.is_empty() {
        return false;
    }

    evidence.iter().any(|span| {
        span.quote
            .as_deref()
            .is_some_and(|quote| normalized_folded_text_key(quote).contains(&surface_key))
    })
}

fn evidence_mentions_any_surface(evidence: &[StoryEvidenceSpan], surfaces: &[String]) -> bool {
    surfaces
        .iter()
        .any(|surface| alias_evidence_mentions_surface(evidence, surface))
}

fn character_surfaces_share_distinctive_token(left: &str, right: &str) -> bool {
    let left_tokens = distinctive_surface_tokens(left);
    if left_tokens.is_empty() {
        return false;
    }
    let right_tokens = distinctive_surface_tokens(right);
    left_tokens.iter().any(|token| right_tokens.contains(token))
}

fn distinctive_surface_tokens(value: &str) -> std::collections::HashSet<String> {
    normalized_folded_text_key(value)
        .split('_')
        .filter(|token| token.chars().count() >= 2)
        .map(str::to_string)
        .collect()
}

fn find_character_identity_index_by_surface(
    identities: &[CharacterIdentity],
    surface: &str,
) -> Option<usize> {
    let key = normalized_text_key(surface);
    if key.is_empty() {
        return None;
    }

    identities.iter().position(|identity| {
        normalized_text_key(&identity.name) == key
            || identity
                .aliases
                .iter()
                .any(|alias| normalized_text_key(&alias.text) == key)
    })
}

fn better_alias_owner_by_surface(
    identities: &[CharacterIdentity],
    owner_index: usize,
    alias_text: &str,
) -> Option<usize> {
    let alias_key = normalized_text_key(alias_text);
    if alias_key.is_empty() {
        return None;
    }

    if identities.iter().enumerate().any(|(index, identity)| {
        index != owner_index && normalized_text_key(&identity.name) == alias_key
    }) {
        return None;
    }

    let alias_identity = CharacterIdentity {
        name: alias_text.to_string(),
        aliases: Vec::new(),
    };
    let owner_score = character_identity_candidate_score(&alias_identity, &identities[owner_index]);
    let mut best: Option<(usize, f64)> = None;

    for (index, identity) in identities.iter().enumerate() {
        if index == owner_index {
            continue;
        }

        let score = character_identity_candidate_score(&alias_identity, identity);
        if score < CHARACTER_ALIAS_OWNER_REDIRECT_MIN_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => best = Some((index, score)),
            None => best = Some((index, score)),
            _ => {}
        }
    }

    let (best_index, best_score) = best?;
    if best_score > owner_score + CHARACTER_CANONICAL_REVIEW_SCORE_GAP {
        Some(best_index)
    } else {
        None
    }
}

fn normalize_character_alias_type(value: &str) -> String {
    match normalize_ascii_snake_key(value).as_str() {
        "nickname" | "biet_danh" | "stable_nickname" => "nickname".to_string(),
        "title_or_role" | "title" | "role" | "danh_xung" | "chuc_vu" | "vai_tro" => {
            "title_or_role".to_string()
        }
        "relationship_name" | "relationship" | "relation" | "family_relation" => {
            "relationship_name".to_string()
        }
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
        "stable_alias" | "other_alias" | "alias" | "aliases" | "other_name" | "other_names" => {
            "other_alias".to_string()
        }
        _ => "other_alias".to_string(),
    }
}

fn is_persistable_character_alias_type(alias_type: &str) -> bool {
    matches!(
        normalize_character_alias_type(alias_type).as_str(),
        "nickname" | "other_alias"
    )
}

fn normalize_character_alias_label(alias_type: &str, label: &str) -> String {
    let label = label.trim();
    if !label.is_empty() {
        return label.to_string();
    }

    match normalize_character_alias_type(alias_type).as_str() {
        "nickname" => "Biệt danh".to_string(),
        "title_or_role" => "Danh xưng".to_string(),
        "unstable_reference" => "Tham chiếu tạm thời".to_string(),
        _ => "Tên gọi khác".to_string(),
    }
}

async fn extract_character_relationships_for_chapter(
    state: &AppState,
    project_id: &str,
    job_id: &str,
    chapter: &Chapter,
    novel: &Novel,
    document: &mut StoryExtractionDocument,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let identities = character_identities_for_relationships(document);
    let mut outputs = Vec::new();

    if identities.len() < 2 {
        return Ok(outputs);
    }

    let identity_nodes_json =
        serde_json::to_string(&identities).unwrap_or_else(|_| "[]".to_string());
    let novel_analysis_context = build_current_novel_analysis_context(novel);
    let relationship_input = DraftExtractionInput {
        chapter_num: chapter.chapter_num,
        title: Some(chapter.title.clone()),
        source_language: novel.source_language.clone(),
        text: chapter.content.clone(),
        prior_context: Some(format!(
            "{novel_analysis_context}\n\nĐây là relationship candidate pass sau khi character identity, aliases, mentions và fields của chương hiện tại đã hoàn tất. Chỉ trả quan hệ có evidence trong chương hiện tại; aliases chỉ dùng để resolve về canonical character, không phải node riêng."
        )),
    };
    let prompt =
        build_character_relationship_candidate_prompt(&relationship_input, &identity_nodes_json);
    let (candidates, response) = match call_local_json_array::<CharacterRelationshipCandidate>(
        state,
        &prompt,
        CHARACTER_RELATIONSHIP_CANDIDATE_MAX_TOKENS,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            outputs.push(json!({
                "mode": "relationship_candidate_pass_failed_non_blocking",
                "error": error.message,
            }));
            return Ok(outputs);
        }
    };

    let candidate_count = candidates.len();
    let (relationships_by_pair, resolution_warnings) =
        resolve_character_relationship_candidates(candidates, &identities, chapter);
    let mut persisted_record_count = 0usize;
    let mut persisted_relationship_count = 0usize;
    let mut relationship_verification_reports = Vec::new();

    for ((left_index, right_index), relationships) in relationships_by_pair {
        let character_a = &identities[left_index];
        let character_b = &identities[right_index];
        let relationships = normalize_character_relationships(relationships);
        if relationships.is_empty() {
            continue;
        }
        let (relationships, reports) = verify_character_relationships(
            state,
            &relationship_input,
            character_a,
            character_b,
            relationships,
        )
        .await;
        relationship_verification_reports.extend(reports);
        if relationships.is_empty() {
            continue;
        }

        persisted_relationship_count += relationships.len();
        if let Some(record) = relationship_pair_to_record(character_a, character_b, relationships) {
            document.records.push(record);
            persisted_record_count += 1;
        }
    }

    if persisted_record_count > 0 {
        state
            .store
            .replace_story_extraction_records_for_chapter(
                project_id,
                job_id,
                &chapter.id,
                CHARACTER_EXTRACTION_SCHEMA_VERSION,
                document,
                "character_relationship_candidate_pass",
            )
            .await?;
        publish_project_event(
            state,
            project_id,
            "story_extraction_updated",
            Some(job_id),
            Some(&chapter.id),
            "character relationship candidates persisted",
        );
    }

    outputs.push(json!({
        "mode": "relationship_candidate_pass",
        "candidate_count": candidate_count,
        "persisted_record_count": persisted_record_count,
        "persisted_relationship_count": persisted_relationship_count,
        "resolution_warnings": resolution_warnings,
        "verification": relationship_verification_reports,
        "response": response,
    }));

    Ok(outputs)
}

fn resolve_character_relationship_candidates(
    candidates: Vec<CharacterRelationshipCandidate>,
    identities: &[CharacterIdentity],
    chapter: &Chapter,
) -> (
    HashMap<(usize, usize), Vec<CharacterRelationshipExtraction>>,
    Vec<serde_json::Value>,
) {
    let mut relationships_by_pair =
        HashMap::<(usize, usize), Vec<CharacterRelationshipExtraction>>::new();
    let mut warnings = Vec::new();

    for candidate in candidates {
        let source_name = clean_character_surface(&candidate.source_name);
        let target_name = clean_character_surface(&candidate.target_name);
        let relationship_kind = normalize_relationship_candidate_kind(&candidate.relationship_kind);
        if relationship_kind != "stable_relation" {
            warnings.push(json!({
                "code": "relationship_candidate_not_stable",
                "source_name": source_name,
                "target_name": target_name,
                "relationship_kind": relationship_kind,
            }));
            continue;
        }

        let confidence = candidate.confidence.unwrap_or(0.0);
        if confidence < CHARACTER_RELATIONSHIP_MIN_CONFIDENCE {
            warnings.push(json!({
                "code": "relationship_candidate_low_confidence",
                "source_name": source_name,
                "target_name": target_name,
                "confidence": confidence,
            }));
            continue;
        }

        let Some(source_index) = resolve_relationship_identity_index(identities, &source_name)
        else {
            warnings.push(json!({
                "code": "relationship_source_not_resolved",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        };
        let Some(target_index) = resolve_relationship_identity_index(identities, &target_name)
        else {
            warnings.push(json!({
                "code": "relationship_target_not_resolved",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        };

        if source_index == target_index {
            warnings.push(json!({
                "code": "relationship_self_reference_skipped",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        }

        if !relationship_candidate_has_grounded_evidence(&candidate, chapter) {
            warnings.push(json!({
                "code": "relationship_candidate_not_grounded",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        }

        let (left_index, right_index, relationship) = relationship_candidate_to_pair_extraction(
            candidate,
            source_index,
            target_index,
            chapter.chapter_num,
        );
        relationships_by_pair
            .entry((left_index, right_index))
            .or_default()
            .push(relationship);
    }

    (relationships_by_pair, warnings)
}

fn normalize_relationship_candidate_kind(value: &str) -> &'static str {
    match normalize_ascii_snake_key(value).as_str() {
        "stable_relation" | "relationship" | "relation" | "direct_relation" => "stable_relation",
        "temporary_interaction" | "interaction" | "scene_interaction" => "temporary_interaction",
        "event" | "action" | "scene_event" => "event",
        _ => "uncertain",
    }
}

fn resolve_relationship_identity_index(
    identities: &[CharacterIdentity],
    surface: &str,
) -> Option<usize> {
    let key = normalized_text_key(surface);
    if key.is_empty() {
        return None;
    }

    let mut matched_index = None;
    for (index, identity) in identities.iter().enumerate() {
        let matches_identity = character_identity_surfaces(identity)
            .into_iter()
            .any(|(candidate_surface, _)| normalized_text_key(&candidate_surface) == key);
        if !matches_identity {
            continue;
        }

        if matched_index.is_some() {
            return None;
        }

        matched_index = Some(index);
    }

    matched_index
}

fn relationship_candidate_has_grounded_evidence(
    candidate: &CharacterRelationshipCandidate,
    chapter: &Chapter,
) -> bool {
    candidate.evidence.iter().any(|evidence| {
        evidence
            .quote
            .as_deref()
            .is_some_and(|quote| chapter_text_contains_evidence_quote(&chapter.content, quote))
    })
}

fn chapter_text_contains_evidence_quote(chapter_text: &str, quote: &str) -> bool {
    let quote = quote.trim();
    if quote.is_empty() {
        return false;
    }

    if chapter_text.contains(quote) {
        return true;
    }

    let normalized_chapter = collapse_evidence_whitespace(chapter_text);
    let normalized_quote = collapse_evidence_whitespace(quote);
    !normalized_quote.is_empty() && normalized_chapter.contains(&normalized_quote)
}

fn collapse_evidence_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn relationship_candidate_to_pair_extraction(
    candidate: CharacterRelationshipCandidate,
    source_index: usize,
    target_index: usize,
    chapter_num: i64,
) -> (usize, usize, CharacterRelationshipExtraction) {
    let relationship_type = if candidate.relationship_type.trim().is_empty() {
        normalize_ascii_snake_key(&candidate.source_to_target_label)
    } else {
        normalize_ascii_snake_key(&candidate.relationship_type)
    };
    let source_to_target_label = candidate.source_to_target_label.trim().to_string();
    let target_to_source_label = candidate.target_to_source_label.trim().to_string();
    let target_to_source_label = if target_to_source_label.is_empty() {
        source_to_target_label.clone()
    } else {
        target_to_source_label
    };
    let evidence = normalize_relationship_candidate_evidence(candidate.evidence, chapter_num);

    if source_index < target_index {
        (
            source_index,
            target_index,
            CharacterRelationshipExtraction {
                relationship_type,
                a_to_b_label: source_to_target_label,
                b_to_a_label: target_to_source_label,
                confidence: candidate.confidence,
                evidence,
            },
        )
    } else {
        (
            target_index,
            source_index,
            CharacterRelationshipExtraction {
                relationship_type,
                a_to_b_label: target_to_source_label,
                b_to_a_label: source_to_target_label,
                confidence: candidate.confidence,
                evidence,
            },
        )
    }
}

fn normalize_relationship_candidate_evidence(
    evidence: Vec<StoryEvidenceSpan>,
    chapter_num: i64,
) -> Vec<StoryEvidenceSpan> {
    evidence
        .into_iter()
        .filter_map(|mut evidence| {
            let quote = evidence.quote.as_deref()?.trim();
            if quote.is_empty() {
                return None;
            }

            evidence.chapter_num = chapter_num;
            evidence.start_char = None;
            evidence.end_char = None;
            evidence.quote = Some(quote.to_string());
            Some(evidence)
        })
        .collect()
}

fn character_identities_for_relationships(
    document: &StoryExtractionDocument,
) -> Vec<CharacterIdentity> {
    let mut identities = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for record in &document.records {
        if record.group_key != "character" || record.mentions.is_empty() {
            continue;
        }

        let key = normalize_ascii_snake_key(
            record
                .entity_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(&record.display_name),
        );
        if key.is_empty() || !seen.insert(key) {
            continue;
        }

        identities.push(CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_payload_record(record),
        });
    }

    identities
}

fn normalize_character_relationships(
    relationships: Vec<CharacterRelationshipExtraction>,
) -> Vec<CharacterRelationshipExtraction> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for mut relationship in relationships {
        relationship.relationship_type = normalize_ascii_snake_key(&relationship.relationship_type);
        relationship.a_to_b_label = relationship.a_to_b_label.trim().to_string();
        relationship.b_to_a_label = relationship.b_to_a_label.trim().to_string();

        if relationship.relationship_type.is_empty()
            || relationship.a_to_b_label.is_empty()
            || relationship.b_to_a_label.is_empty()
            || !relationship.evidence.iter().any(|evidence| {
                evidence
                    .quote
                    .as_deref()
                    .is_some_and(|quote| !quote.trim().is_empty())
            })
        {
            continue;
        }

        for evidence in &mut relationship.evidence {
            evidence.start_char = None;
            evidence.end_char = None;
        }

        let key = format!(
            "{}:{}:{}",
            relationship.relationship_type,
            normalized_text_key(&relationship.a_to_b_label),
            normalized_text_key(&relationship.b_to_a_label)
        );
        if seen.insert(key) {
            normalized.push(relationship);
        }
    }

    normalized
}

async fn verify_character_relationships(
    state: &AppState,
    relationship_input: &DraftExtractionInput,
    character_a: &CharacterIdentity,
    character_b: &CharacterIdentity,
    relationships: Vec<CharacterRelationshipExtraction>,
) -> (Vec<CharacterRelationshipExtraction>, Vec<serde_json::Value>) {
    let character_a_json = serde_json::to_string(character_a).unwrap_or_else(|_| "{}".to_string());
    let character_b_json = serde_json::to_string(character_b).unwrap_or_else(|_| "{}".to_string());
    let mut verified_relationships = Vec::new();
    let mut reports = Vec::new();

    for relationship in relationships {
        let relationship_json = serde_json::to_string(&json!({
            "relationship_type": &relationship.relationship_type,
            "a_to_b_label": &relationship.a_to_b_label,
            "b_to_a_label": &relationship.b_to_a_label,
            "confidence": relationship.confidence,
            "evidence": &relationship.evidence,
        }))
        .unwrap_or_else(|_| "{}".to_string());
        let prompt = build_character_relationship_verification_prompt(
            relationship_input,
            &character_a_json,
            &character_b_json,
            &relationship_json,
        );
        let verification_result = call_local_json_array::<CharacterRelationshipVerification>(
            state,
            &prompt,
            CHARACTER_RELATIONSHIP_VERIFICATION_MAX_TOKENS,
        )
        .await;

        match verification_result {
            Ok((verifications, response)) => {
                let verification = verifications.into_iter().next();
                let accepted = verification
                    .as_ref()
                    .is_some_and(|verification| relationship_verification_accepts(verification));
                reports.push(json!({
                    "character_a": &character_a.name,
                    "character_b": &character_b.name,
                    "relationship_type": &relationship.relationship_type,
                    "a_to_b_label": &relationship.a_to_b_label,
                    "b_to_a_label": &relationship.b_to_a_label,
                    "accepted": accepted,
                    "verification": verification,
                    "response": response,
                }));

                if accepted {
                    verified_relationships.push(relationship);
                }
            }
            Err(error) => {
                reports.push(json!({
                    "character_a": &character_a.name,
                    "character_b": &character_b.name,
                    "relationship_type": &relationship.relationship_type,
                    "a_to_b_label": &relationship.a_to_b_label,
                    "b_to_a_label": &relationship.b_to_a_label,
                    "accepted": false,
                    "mode": "relationship_verification_failed",
                    "error": error.message,
                }));
            }
        }
    }

    (verified_relationships, reports)
}

fn relationship_verification_accepts(verification: &CharacterRelationshipVerification) -> bool {
    verification.accepted
        && verification.owner_direction_ok
        && verification.confidence.unwrap_or(0.0)
            >= CHARACTER_RELATIONSHIP_VERIFICATION_MIN_CONFIDENCE
        && relationship_scope_is_persistable(&verification.relationship_scope)
}

fn relationship_scope_is_persistable(scope: &str) -> bool {
    matches!(
        normalize_ascii_snake_key(scope).as_str(),
        "kinship" | "organization_hierarchy" | "stable_relationship"
    )
}

fn relationship_pair_to_record(
    character_a: &CharacterIdentity,
    character_b: &CharacterIdentity,
    relationships: Vec<CharacterRelationshipExtraction>,
) -> Option<StoryExtractionRecordPayload> {
    let character_a_key = normalize_ascii_snake_key(&character_a.name);
    let character_b_key = normalize_ascii_snake_key(&character_b.name);
    if character_a_key.is_empty() || character_b_key.is_empty() {
        return None;
    }

    let mut values = Vec::new();
    for relationship in relationships {
        values.push(relationship_field_value(
            &relationship.a_to_b_label,
            &relationship,
            &character_b.name,
        ));
        values.push(relationship_field_value(
            &relationship.b_to_a_label,
            &relationship,
            &character_a.name,
        ));
    }

    if values.is_empty() {
        return None;
    }

    Some(StoryExtractionRecordPayload {
        group_key: "relationship".to_string(),
        group_label: "Quan Hệ".to_string(),
        entity_key: Some(format!(
            "relationship|{}|{}",
            character_a_key, character_b_key
        )),
        display_name: format!("{} ↔ {}", character_a.name, character_b.name),
        mentions: Vec::new(),
        fields: vec![StoryExtractionFieldPayload {
            field_key: "relationship".to_string(),
            field_label: "Quan hệ".to_string(),
            values,
        }],
    })
}

fn relationship_field_value(
    label: &str,
    relationship: &CharacterRelationshipExtraction,
    related_character: &str,
) -> StoryExtractionFieldValuePayload {
    StoryExtractionFieldValuePayload {
        value: label.to_string(),
        confidence: relationship.confidence,
        related_character: Some(related_character.to_string()),
        relationship_type: Some(relationship.relationship_type.clone()),
        relationship_label: Some(label.to_string()),
        relationship_direction: Some("self_to_related".to_string()),
        evidence: relationship.evidence.clone(),
    }
}

async fn scan_character_mentions_with_backend(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: &CharacterIdentity,
    character_json: &str,
    chapter_text: &str,
) -> Result<(Vec<StoryCharacterMention>, serde_json::Value), ApiError> {
    let surfaces = character_identity_surfaces(identity);
    let mut scanned = Vec::new();
    for (surface, mention_type) in &surfaces {
        let ambiguous = is_ambiguous_character_surface(surface);
        scanned.extend(find_surface_occurrences(
            chapter_text,
            surface,
            mention_type,
            ambiguous,
        ));
    }

    let scanned_count = scanned.len();
    let selected = select_non_overlapping_occurrences(scanned);
    let mut mentions = Vec::new();
    let mut occurrence_reports = Vec::new();
    let mut ambiguous_groups = Vec::<(String, Vec<ScannedCharacterOccurrence>)>::new();
    let mut ambiguous_group_indexes = std::collections::HashMap::<String, usize>::new();

    for occurrence in selected {
        if occurrence.ambiguous {
            let group_key = character_occurrence_group_key(&occurrence);
            if let Some(index) = ambiguous_group_indexes.get(&group_key).copied() {
                ambiguous_groups[index].1.push(occurrence);
            } else {
                ambiguous_group_indexes.insert(group_key.clone(), ambiguous_groups.len());
                ambiguous_groups.push((group_key, vec![occurrence]));
            }
        } else {
            occurrence_reports.push(json!({
                "mode": "direct_boundary_scan",
                "occurrence": occurrence.clone(),
                "confirmed": true,
            }));
            mentions.push(scanned_occurrence_to_mention(occurrence));
        }
    }

    for (group_key, occurrences) in ambiguous_groups {
        let occurrence_count = occurrences.len();
        let surface_text = occurrences
            .first()
            .map(|occurrence| occurrence.text.clone())
            .unwrap_or_default();
        let samples = sample_character_occurrences_for_confirmation(&occurrences);
        let mut sample_reports = Vec::new();
        let mut confirmed_samples = Vec::new();
        let mut rejected_sample_count = 0usize;

        for occurrence in &samples {
            let (confirmed, confirmation, response) = confirm_character_occurrence_with_llm(
                state,
                chunk_input,
                character_json,
                chapter_text,
                occurrence,
            )
            .await?;

            sample_reports.push(json!({
                "occurrence": occurrence,
                "confirmed": confirmed,
                "confirmation": confirmation,
                "response": response,
            }));

            if confirmed {
                confirmed_samples.push(occurrence.clone());
            } else {
                rejected_sample_count += 1;
            }
        }

        let accept_all = !samples.is_empty()
            && rejected_sample_count == 0
            && surface_sample_confirmation_can_accept_all(&surface_text);

        if accept_all {
            for occurrence in occurrences {
                mentions.push(scanned_occurrence_to_mention(occurrence));
            }
        } else {
            for occurrence in confirmed_samples {
                mentions.push(scanned_occurrence_to_mention(occurrence));
            }
        }

        let confirmed_sample_count = sample_reports
            .iter()
            .filter(|report| {
                report
                    .get("confirmed")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
            })
            .count();

        occurrence_reports.push(json!({
            "mode": "llm_surface_sample_confirmation",
            "surface_key": group_key,
            "surface_text": surface_text,
            "occurrence_count": occurrence_count,
            "sample_limit": CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT,
            "sample_count": samples.len(),
            "confirmed_sample_count": confirmed_sample_count,
            "rejected_sample_count": rejected_sample_count,
            "decision": if accept_all { "accept_all_occurrences" } else { "accept_confirmed_samples_only" },
            "samples": sample_reports,
        }));
    }

    mentions.sort_by(|left, right| {
        left.start_char
            .cmp(&right.start_char)
            .then_with(|| right.end_char.cmp(&left.end_char))
    });

    let report = json!({
        "mode": "backend_surface_scan_with_sampled_llm_confirmation",
        "surface_count": surfaces.len(),
        "scanned_occurrence_count": scanned_count,
        "confirmed_mention_count": mentions.len(),
        "confirmation_sample_limit": CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT,
        "occurrences": occurrence_reports,
    });

    Ok((mentions, report))
}

async fn confirm_character_occurrence_with_llm(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    character_json: &str,
    chapter_text: &str,
    occurrence: &ScannedCharacterOccurrence,
) -> Result<
    (
        bool,
        Option<CharacterOccurrenceConfirmation>,
        serde_json::Value,
    ),
    ApiError,
> {
    let context = character_occurrence_context(
        chapter_text,
        occurrence.start_char,
        occurrence.end_char,
        CHARACTER_OCCURRENCE_CONTEXT_CHARS,
    );
    let parent_prior_context = chunk_input.prior_context.as_deref().unwrap_or("").trim();
    let confirmation_prior_context = if parent_prior_context.is_empty() {
        "Backend đã exact-scan surface bằng boundary ký tự trước khi hỏi xác nhận.".to_string()
    } else {
        format!(
            "{parent_prior_context}\n\nBackend đã exact-scan surface bằng boundary ký tự trước khi hỏi xác nhận."
        )
    };
    let confirmation_input = DraftExtractionInput {
        chapter_num: chunk_input.chapter_num,
        title: chunk_input.title.clone(),
        source_language: chunk_input.source_language.clone(),
        text: context,
        prior_context: Some(confirmation_prior_context),
    };
    let prompt = build_character_occurrence_confirmation_prompt(
        &confirmation_input,
        character_json,
        &occurrence.text,
    );
    let (confirmations, response) = call_local_json_array::<CharacterOccurrenceConfirmation>(
        state,
        &prompt,
        CHARACTER_OCCURRENCE_CONFIRMATION_MAX_TOKENS,
    )
    .await?;
    let confirmation = confirmations.into_iter().next();
    let confirmed = confirmation.as_ref().is_some_and(|item| {
        item.is_character_mention
            && item.confidence.unwrap_or(1.0) >= CHARACTER_OCCURRENCE_CONFIRMATION_MIN_CONFIDENCE
    });

    Ok((confirmed, confirmation, json!(response)))
}

fn character_occurrence_group_key(occurrence: &ScannedCharacterOccurrence) -> String {
    format!(
        "{}:{}",
        occurrence.mention_type,
        normalized_text_key(&occurrence.text)
    )
}

fn sample_character_occurrences_for_confirmation(
    occurrences: &[ScannedCharacterOccurrence],
) -> Vec<ScannedCharacterOccurrence> {
    if occurrences.len() <= CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT {
        return occurrences.to_vec();
    }

    let last_index = occurrences.len() - 1;
    let sample_slots = CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT.saturating_sub(1);
    let mut samples = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sample_index in 0..CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT {
        let occurrence_index = if sample_slots == 0 {
            0
        } else {
            sample_index * last_index / sample_slots
        };

        if seen.insert(occurrence_index) {
            samples.push(occurrences[occurrence_index].clone());
        }
    }

    samples
}

fn surface_sample_confirmation_can_accept_all(surface: &str) -> bool {
    let tokens = surface.split_whitespace().collect::<Vec<_>>();
    let char_count = surface.chars().filter(|ch| ch.is_alphanumeric()).count();

    char_count >= 5 && tokens.len() >= 2
}

fn scanned_occurrence_to_mention(occurrence: ScannedCharacterOccurrence) -> StoryCharacterMention {
    StoryCharacterMention {
        text: occurrence.text,
        start_char: occurrence.start_char,
        end_char: occurrence.end_char,
        mention_type: Some(occurrence.mention_type),
    }
}

fn character_identity_surfaces(identity: &CharacterIdentity) -> Vec<(String, String)> {
    let mut surfaces = Vec::new();
    let mut seen = std::collections::HashSet::new();
    push_character_surface(
        &mut surfaces,
        &mut seen,
        clean_character_surface(&identity.name),
        "name",
    );

    for alias in &identity.aliases {
        push_character_surface(
            &mut surfaces,
            &mut seen,
            clean_character_surface(&alias.text),
            "alias",
        );
    }

    surfaces
}

fn push_character_surface(
    surfaces: &mut Vec<(String, String)>,
    seen: &mut std::collections::HashSet<String>,
    surface: String,
    mention_type: &str,
) {
    if surface.is_empty() || surface.chars().count() < 2 || surface.chars().count() > 64 {
        return;
    }

    let key = normalized_text_key(&surface);
    if key.is_empty() || !seen.insert(key) {
        return;
    }

    surfaces.push((surface, mention_type.to_string()));
}

fn clean_character_surface(value: &str) -> String {
    let mut surface = value.trim().trim_matches(is_quote_char).to_string();
    surface = surface.split_whitespace().collect::<Vec<_>>().join(" ");

    for suffix in [" này", " đó", " kia"] {
        if surface.ends_with(suffix) {
            surface.truncate(surface.len() - suffix.len());
            surface = surface.trim().to_string();
        }
    }

    if surface.starts_with("vị ") && surface.split_whitespace().count() > 2 {
        surface = surface.trim_start_matches("vị ").trim().to_string();
    }

    surface.trim().trim_matches(is_quote_char).to_string()
}

fn is_quote_char(ch: char) -> bool {
    matches!(ch, '"' | '\'' | '`' | '“' | '”' | '‘' | '’')
}

fn is_ambiguous_character_surface(surface: &str) -> bool {
    let tokens = surface.split_whitespace().collect::<Vec<_>>();
    let uppercase_token_count = tokens
        .iter()
        .filter(|token| token.chars().next().is_some_and(char::is_uppercase))
        .count();

    uppercase_token_count == 0 || (tokens.len() <= 2 && uppercase_token_count < 2)
}

fn find_surface_occurrences(
    chapter_text: &str,
    surface: &str,
    mention_type: &str,
    ambiguous: bool,
) -> Vec<ScannedCharacterOccurrence> {
    let mut occurrences = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for variant in surface_scan_variants(surface) {
        for (byte_start, matched_text) in chapter_text.match_indices(&variant) {
            let byte_end = byte_start + matched_text.len();
            if !has_character_surface_boundary(chapter_text, byte_start, byte_end) {
                continue;
            }

            let start_char = chapter_text[..byte_start].chars().count() as i64;
            let end_char = start_char + matched_text.chars().count() as i64;
            let key = format!("{start_char}:{end_char}");
            if !seen.insert(key) {
                continue;
            }

            occurrences.push(ScannedCharacterOccurrence {
                text: matched_text.to_string(),
                start_char,
                end_char,
                mention_type: mention_type.to_string(),
                ambiguous,
            });
        }
    }

    occurrences
}

fn surface_scan_variants(surface: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let mut seen = std::collections::HashSet::new();
    push_surface_variant(&mut variants, &mut seen, surface.to_string());

    if let Some(first) = surface.chars().next() {
        let rest = &surface[first.len_utf8()..];
        let lower_first = first.to_lowercase().collect::<String>();
        let upper_first = first.to_uppercase().collect::<String>();
        push_surface_variant(&mut variants, &mut seen, format!("{lower_first}{rest}"));
        push_surface_variant(&mut variants, &mut seen, format!("{upper_first}{rest}"));
    }

    variants
}

fn push_surface_variant(
    variants: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    value: String,
) {
    if !value.is_empty() && seen.insert(value.clone()) {
        variants.push(value);
    }
}

fn has_character_surface_boundary(text: &str, byte_start: usize, byte_end: usize) -> bool {
    let before = text[..byte_start].chars().next_back();
    let after = text[byte_end..].chars().next();

    !before.is_some_and(is_character_surface_word_char)
        && !after.is_some_and(is_character_surface_word_char)
}

fn is_character_surface_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn select_non_overlapping_occurrences(
    mut occurrences: Vec<ScannedCharacterOccurrence>,
) -> Vec<ScannedCharacterOccurrence> {
    occurrences.sort_by(|left, right| {
        let left_len = left.end_char - left.start_char;
        let right_len = right.end_char - right.start_char;
        right_len
            .cmp(&left_len)
            .then_with(|| left.start_char.cmp(&right.start_char))
            .then_with(|| left.text.cmp(&right.text))
    });

    let mut selected = Vec::<ScannedCharacterOccurrence>::new();
    for occurrence in occurrences {
        if selected
            .iter()
            .any(|selected| character_mentions_overlap(selected, &occurrence))
        {
            continue;
        }
        selected.push(occurrence);
    }

    selected.sort_by(|left, right| {
        left.start_char
            .cmp(&right.start_char)
            .then_with(|| right.end_char.cmp(&left.end_char))
    });
    selected
}

fn character_mentions_overlap(
    left: &ScannedCharacterOccurrence,
    right: &ScannedCharacterOccurrence,
) -> bool {
    left.start_char < right.end_char && right.start_char < left.end_char
}

fn character_occurrence_context(
    chapter_text: &str,
    start_char: i64,
    end_char: i64,
    radius: i64,
) -> String {
    let chars = chapter_text.chars().collect::<Vec<_>>();
    let context_start = start_char.saturating_sub(radius).max(0) as usize;
    let context_end = (end_char + radius).max(0).min(chars.len() as i64) as usize;
    let mention_start = start_char.max(0) as usize;
    let mention_end = end_char.max(0).min(chars.len() as i64) as usize;

    let mut context = String::new();
    for (index, ch) in chars
        .iter()
        .enumerate()
        .take(context_end)
        .skip(context_start)
    {
        if index == mention_start {
            context.push_str("[[");
        }
        context.push(*ch);
        if index + 1 == mention_end {
            context.push_str("]]");
        }
    }
    context
}

fn build_character_field_contexts(chunk_text: &str, identity: &CharacterIdentity) -> Vec<String> {
    let mut contexts = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sentence in split_character_field_sentences(chunk_text) {
        if contexts.len() >= CHARACTER_FIELD_CONTEXT_MAX_ITEMS {
            break;
        }
        let Some(context) = mark_character_field_context(&sentence, identity) else {
            continue;
        };
        let key = normalized_text_key(&context);
        if !key.is_empty() && seen.insert(key) {
            contexts.push(context);
        }
    }

    contexts
}

fn split_character_field_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if is_character_field_sentence_boundary(ch) {
            push_character_field_sentence(&mut sentences, &mut current);
        }
    }
    push_character_field_sentence(&mut sentences, &mut current);

    sentences
}

fn push_character_field_sentence(sentences: &mut Vec<String>, current: &mut String) {
    let sentence = current.trim();
    if !sentence.is_empty() {
        sentences.push(sentence.to_string());
    }
    current.clear();
}

fn is_character_field_sentence_boundary(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?' | '。' | '！' | '？' | '\n' | '\r')
}

fn mark_character_field_context(sentence: &str, identity: &CharacterIdentity) -> Option<String> {
    let mut occurrences = Vec::new();
    for (surface, _) in character_identity_surfaces(identity) {
        occurrences.extend(find_surface_occurrences(
            sentence,
            &surface,
            "field_context",
            false,
        ));
    }

    let occurrences = select_non_overlapping_occurrences(occurrences);
    if occurrences.is_empty() {
        return None;
    }

    Some(insert_character_field_markers(sentence, &occurrences))
}

fn insert_character_field_markers(
    text: &str,
    occurrences: &[ScannedCharacterOccurrence],
) -> String {
    let starts = occurrences
        .iter()
        .map(|occurrence| occurrence.start_char as usize)
        .collect::<std::collections::HashSet<_>>();
    let ends = occurrences
        .iter()
        .map(|occurrence| occurrence.end_char as usize)
        .collect::<std::collections::HashSet<_>>();
    let mut marked = String::new();

    for (index, ch) in text.chars().enumerate() {
        if starts.contains(&index) {
            marked.push_str("[[");
        }
        marked.push(ch);
        if ends.contains(&(index + 1)) {
            marked.push_str("]]");
        }
    }

    marked
}

fn merge_character_identity_records(
    document: &mut StoryExtractionDocument,
    identities: &[CharacterIdentity],
) {
    for identity in identities {
        let mut record = identity_to_record(identity, document.chapter_num);
        let exact_index = document.records.iter().position(|record| {
            record.group_key == "character"
                && normalized_text_key(&record.display_name) == normalized_text_key(&identity.name)
        });
        let matched_index = exact_index.or_else(|| {
            document
                .records
                .iter()
                .position(|record| payload_record_matches_identity(record, identity))
        });

        if let Some(index) = matched_index {
            let target = &mut document.records[index];
            target.group_key = "character".to_string();
            target.group_label = "Nhân Vật".to_string();
            target.display_name = identity.name.clone();
            target.entity_key = record.entity_key.take();
            merge_character_record(target, record);
        } else {
            document.records.push(record);
        }
    }

    dedupe_character_records_by_identity(document);
}

fn dedupe_character_records_by_identity(document: &mut StoryExtractionDocument) {
    let mut index = 0;
    while index < document.records.len() {
        if document.records[index].group_key != "character" {
            index += 1;
            continue;
        }

        let identity = CharacterIdentity {
            name: document.records[index].display_name.clone(),
            aliases: aliases_from_payload_record(&document.records[index]),
        };
        let duplicate_index = document
            .records
            .iter()
            .enumerate()
            .skip(index + 1)
            .find(|(_, record)| {
                record.group_key == "character"
                    && payload_record_matches_identity(record, &identity)
            })
            .map(|(duplicate_index, _)| duplicate_index);

        if let Some(duplicate_index) = duplicate_index {
            let duplicate = document.records.remove(duplicate_index);
            merge_character_record(&mut document.records[index], duplicate);
        } else {
            index += 1;
        }
    }
}

async fn resolve_character_identities_across_chapters(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identities: Vec<CharacterIdentity>,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
) -> (Vec<CharacterIdentity>, Vec<serde_json::Value>) {
    let mut resolved = Vec::new();
    let mut merge_decision_outputs = Vec::new();

    for identity in identities {
        let (canonical, merge_decision_output) = resolve_character_identity_across_chapters(
            state,
            chunk_input,
            identity,
            db_records,
            db_aliases,
            working_document,
        )
        .await;
        if let Some(output) = merge_decision_output {
            merge_decision_outputs.push(output);
        }
        if let Some(canonical) = canonical {
            merge_character_identity_into_list(&mut resolved, canonical);
        }
    }

    (resolved, merge_decision_outputs)
}

async fn resolve_character_identity_across_chapters(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
) -> (Option<CharacterIdentity>, Option<serde_json::Value>) {
    let known_name_keys = known_character_name_keys(db_records, working_document);

    if let Some(record) = find_exact_db_character_record(&identity, db_records) {
        return (
            Some(identity_from_db_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(record) = find_exact_working_character_record(&identity, working_document) {
        return (
            Some(identity_from_payload_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(candidate) =
        find_exact_alias_map_character_identity(&identity, db_aliases, &known_name_keys)
    {
        return (Some(candidate), None);
    }

    if let Some(candidate) =
        find_high_confidence_alias_map_character_identity(&identity, db_aliases, &known_name_keys)
    {
        return (Some(candidate), None);
    }

    if let Some(record) = find_high_confidence_db_character_record(&identity, db_records) {
        return (
            Some(identity_from_db_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(record) = find_high_confidence_working_character_record(&identity, working_document)
    {
        return (
            Some(identity_from_payload_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(candidate) =
        find_character_merge_review_candidate(&identity, db_records, db_aliases, working_document)
    {
        let (decision, response) =
            confirm_character_identity_merge(state, chunk_input, &identity, &candidate).await;
        let action = normalize_character_identity_merge_action(&decision.action);
        let confidence = decision.confidence.unwrap_or(0.0);
        let observed_name = identity.name.clone();
        let candidate_name = candidate.display_name.clone();

        if action == "merge_existing"
            && candidate.score >= CHARACTER_CANONICAL_AI_MERGE_MIN_SCORE
            && confidence >= CHARACTER_CANONICAL_MERGE_MIN_CONFIDENCE
        {
            let mut canonical = CharacterIdentity {
                name: candidate.display_name.clone(),
                aliases: candidate.aliases.clone(),
            };
            merge_observed_identity_aliases(&mut canonical, Some(&identity), &known_name_keys);
            return (
                Some(canonical),
                Some(json!({
                    "mode": "ai_merge_confirmation",
                    "applied": "merge_existing",
                    "observed_identity": observed_name,
                    "candidate_identity": candidate_name,
                    "candidate_score": candidate.score,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        if action == "ignore" && confidence >= CHARACTER_CANONICAL_IGNORE_MIN_CONFIDENCE {
            return (
                None,
                Some(json!({
                    "mode": "ai_merge_confirmation",
                    "applied": "ignore",
                    "observed_identity": observed_name,
                    "candidate_identity": candidate_name,
                    "candidate_score": candidate.score,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        if action == "create_new"
            && observed_identity_is_known_name_phrase(&identity, db_records, working_document)
        {
            return (
                None,
                Some(json!({
                    "mode": "ai_merge_confirmation",
                    "applied": "ignore_known_name_phrase",
                    "observed_identity": observed_name,
                    "candidate_identity": candidate_name,
                    "candidate_score": candidate.score,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        let sanitized = sanitize_new_character_identity(identity, db_records, working_document);
        return (
            Some(sanitized),
            Some(json!({
                "mode": "ai_merge_confirmation",
                "applied": "create_new",
                "observed_identity": observed_name,
                "candidate_identity": candidate_name,
                "candidate_score": candidate.score,
                "decision": decision,
                "response": response,
            })),
        );
    }

    if let Some(review_candidates) = character_identity_creation_review_candidates(
        &identity,
        db_records,
        db_aliases,
        working_document,
        &chunk_input.text,
    ) {
        let (decision, response) =
            confirm_character_identity_creation(state, chunk_input, &identity, &review_candidates)
                .await;
        let action = normalize_character_identity_creation_action(&decision.action);
        let confidence = decision.confidence.unwrap_or(0.0);
        let observed_name = identity.name.clone();
        let candidate = find_creation_review_target_candidate(&decision, &review_candidates);

        if action == "merge_existing"
            && confidence >= CHARACTER_IDENTITY_CREATION_REVIEW_MIN_CONFIDENCE
        {
            if let Some(candidate) = candidate {
                let mut canonical = identity_from_alias_map_candidate(
                    candidate.clone(),
                    Some(&identity),
                    &known_name_keys,
                );
                merge_observed_identity_aliases(&mut canonical, Some(&identity), &known_name_keys);
                return (
                    Some(canonical),
                    Some(json!({
                        "mode": "identity_creation_review",
                        "applied": "merge_existing",
                        "observed_identity": observed_name,
                        "candidate_identity": candidate.display_name,
                        "decision": decision,
                        "response": response,
                    })),
                );
            }
        }

        if action == "reject" && confidence >= CHARACTER_IDENTITY_REJECT_MIN_CONFIDENCE {
            return (
                None,
                Some(json!({
                    "mode": "identity_creation_review",
                    "applied": "reject",
                    "observed_identity": observed_name,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        if observed_identity_is_known_name_phrase(&identity, db_records, working_document) {
            return (
                None,
                Some(json!({
                    "mode": "identity_creation_review",
                    "applied": "reject_known_name_phrase",
                    "observed_identity": observed_name,
                    "decision": decision,
                    "response": response,
                })),
            );
        }
    } else if observed_identity_is_known_name_phrase(&identity, db_records, working_document) {
        return (
            None,
            Some(json!({
                "mode": "identity_creation_review",
                "applied": "reject_known_name_phrase",
                "observed_identity": identity.name,
            })),
        );
    }

    (
        Some(sanitize_new_character_identity(
            identity,
            db_records,
            working_document,
        )),
        None,
    )
}

fn find_exact_db_character_record<'a>(
    identity: &CharacterIdentity,
    db_records: &'a [StoryExtractionRecordView],
) -> Option<&'a StoryExtractionRecordView> {
    let name_key = normalized_text_key(&identity.name);
    db_records
        .iter()
        .find(|record| normalized_text_key(&record.display_name) == name_key)
}

fn find_exact_working_character_record<'a>(
    identity: &CharacterIdentity,
    working_document: &'a StoryExtractionDocument,
) -> Option<&'a StoryExtractionRecordPayload> {
    let name_key = normalized_text_key(&identity.name);
    working_document
        .records
        .iter()
        .find(|record| normalized_text_key(&record.display_name) == name_key)
}

fn find_exact_alias_map_character_identity(
    identity: &CharacterIdentity,
    db_aliases: &[StoryCharacterAliasView],
    known_name_keys: &std::collections::HashSet<String>,
) -> Option<CharacterIdentity> {
    let mut matched_candidate: Option<CharacterIdentityMergeCandidate> = None;
    for candidate in character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        if character_identity_candidate_score(identity, &candidate_identity) < 1.0 {
            continue;
        }

        if matched_candidate
            .as_ref()
            .is_some_and(|matched| matched.target_key != candidate.target_key)
        {
            return None;
        }

        matched_candidate = Some(candidate);
    }

    matched_candidate.map(|candidate| {
        identity_from_alias_map_candidate(candidate, Some(identity), known_name_keys)
    })
}

fn find_high_confidence_db_character_record<'a>(
    identity: &CharacterIdentity,
    db_records: &'a [StoryExtractionRecordView],
) -> Option<&'a StoryExtractionRecordView> {
    let mut best: Option<(&StoryExtractionRecordView, f64)> = None;
    let mut best_tie_count = 0;

    for record in db_records {
        let candidate = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate);
        if score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            Some((_, best_score)) if (score - best_score).abs() < f64::EPSILON => {
                best_tie_count += 1;
            }
            None => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            _ => {}
        }
    }

    if best_tie_count == 1 {
        best.map(|(record, _)| record)
    } else {
        None
    }
}

fn find_high_confidence_working_character_record<'a>(
    identity: &CharacterIdentity,
    working_document: &'a StoryExtractionDocument,
) -> Option<&'a StoryExtractionRecordPayload> {
    let mut best: Option<(&StoryExtractionRecordPayload, f64)> = None;
    let mut best_tie_count = 0;

    for record in &working_document.records {
        if record.group_key != "character" {
            continue;
        }

        let candidate = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_payload_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate);
        if score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            Some((_, best_score)) if (score - best_score).abs() < f64::EPSILON => {
                best_tie_count += 1;
            }
            None => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            _ => {}
        }
    }

    if best_tie_count == 1 {
        best.map(|(record, _)| record)
    } else {
        None
    }
}

fn find_high_confidence_alias_map_character_identity(
    identity: &CharacterIdentity,
    db_aliases: &[StoryCharacterAliasView],
    known_name_keys: &std::collections::HashSet<String>,
) -> Option<CharacterIdentity> {
    let mut best: Option<(CharacterIdentityMergeCandidate, f64)> = None;
    let mut best_tie_count = 0;

    for candidate in character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => {
                best = Some((candidate, score));
                best_tie_count = 1;
            }
            Some((_, best_score)) if (score - best_score).abs() < f64::EPSILON => {
                best_tie_count += 1;
            }
            None => {
                best = Some((candidate, score));
                best_tie_count = 1;
            }
            _ => {}
        }
    }

    if best_tie_count == 1 {
        best.map(|(candidate, _)| {
            identity_from_alias_map_candidate(candidate, Some(identity), known_name_keys)
        })
    } else {
        None
    }
}

fn character_identity_creation_review_candidates(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
    chunk_text: &str,
) -> Option<Vec<CharacterIdentityMergeCandidate>> {
    let mut candidates = std::collections::HashMap::new();

    for mut candidate in character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if !identity_creation_candidate_is_relevant(
            identity,
            &candidate_identity,
            chunk_text,
            score,
        ) {
            continue;
        }
        candidate.score = score;
        push_character_merge_review_candidate(&mut candidates, candidate);
    }

    for record in db_records {
        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if !identity_creation_candidate_is_relevant(
            identity,
            &candidate_identity,
            chunk_text,
            score,
        ) {
            continue;
        }

        push_character_merge_review_candidate(
            &mut candidates,
            CharacterIdentityMergeCandidate {
                target_key: record
                    .entity_key
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                display_name: candidate_identity.name,
                aliases: candidate_identity.aliases,
                score,
                source: "db_creation_review".to_string(),
                chapter_num: Some(record.chapter_num),
            },
        );
    }

    for record in &working_document.records {
        if record.group_key != "character" {
            continue;
        }

        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_payload_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if !identity_creation_candidate_is_relevant(
            identity,
            &candidate_identity,
            chunk_text,
            score,
        ) {
            continue;
        }

        push_character_merge_review_candidate(
            &mut candidates,
            CharacterIdentityMergeCandidate {
                target_key: record
                    .entity_key
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                display_name: candidate_identity.name,
                aliases: candidate_identity.aliases,
                score,
                source: "working_creation_review".to_string(),
                chapter_num: Some(working_document.chapter_num),
            },
        );
    }

    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.chapter_num.cmp(&right.chapter_num))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    candidates.truncate(12);
    Some(candidates)
}

fn identity_creation_candidate_is_relevant(
    observed: &CharacterIdentity,
    candidate: &CharacterIdentity,
    chunk_text: &str,
    score: f64,
) -> bool {
    score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
        || character_identity_any_surface_appears(candidate, chunk_text)
        || observed_identity_contains_candidate_surface(observed, candidate)
}

fn character_identity_any_surface_appears(identity: &CharacterIdentity, text: &str) -> bool {
    character_identity_surfaces(identity)
        .into_iter()
        .any(|(surface, _)| {
            !find_surface_occurrences(text, &surface, "identity_review", false).is_empty()
        })
}

fn observed_identity_contains_candidate_surface(
    observed: &CharacterIdentity,
    candidate: &CharacterIdentity,
) -> bool {
    let observed_key = normalized_folded_text_key(&observed.name);
    if observed_key.is_empty() {
        return false;
    }

    character_identity_surfaces(candidate)
        .into_iter()
        .any(|(surface, _)| {
            let candidate_key = normalized_folded_text_key(&surface);
            !candidate_key.is_empty()
                && candidate_key != observed_key
                && (observed_key.starts_with(&(candidate_key.clone() + "_"))
                    || observed_key.ends_with(&("_".to_string() + &candidate_key)))
        })
}

fn observed_identity_is_known_name_phrase(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> bool {
    let observed_key = normalized_folded_text_key(&identity.name);
    if observed_key.is_empty() {
        return false;
    }

    let mut known_surfaces = Vec::new();
    known_surfaces.extend(db_records.iter().map(|record| record.display_name.clone()));
    known_surfaces.extend(
        working_document
            .records
            .iter()
            .filter(|record| record.group_key == "character")
            .map(|record| record.display_name.clone()),
    );

    known_surfaces.into_iter().any(|surface| {
        let known_key = normalized_folded_text_key(&surface);
        !known_key.is_empty()
            && known_key != observed_key
            && (observed_key.starts_with(&(known_key.clone() + "_"))
                || observed_key.ends_with(&("_".to_string() + &known_key)))
    })
}

fn find_character_merge_review_candidate(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
) -> Option<CharacterIdentityMergeCandidate> {
    let mut candidates = std::collections::HashMap::new();

    for mut candidate in character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
            && score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE
        {
            candidate.score = score;
            push_character_merge_review_candidate(&mut candidates, candidate);
        }
    }

    for record in db_records {
        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
            && score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE
        {
            push_character_merge_review_candidate(
                &mut candidates,
                CharacterIdentityMergeCandidate {
                    target_key: record
                        .entity_key
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                    display_name: candidate_identity.name,
                    aliases: candidate_identity.aliases,
                    score,
                    source: "db".to_string(),
                    chapter_num: Some(record.chapter_num),
                },
            );
        }
    }

    for record in &working_document.records {
        if record.group_key != "character" {
            continue;
        }

        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_payload_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
            && score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE
        {
            push_character_merge_review_candidate(
                &mut candidates,
                CharacterIdentityMergeCandidate {
                    target_key: record
                        .entity_key
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                    display_name: candidate_identity.name,
                    aliases: candidate_identity.aliases,
                    score,
                    source: "working_document".to_string(),
                    chapter_num: Some(working_document.chapter_num),
                },
            );
        }
    }

    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });

    let best = candidates.first()?;
    if candidates
        .get(1)
        .is_some_and(|next| best.score - next.score < CHARACTER_CANONICAL_REVIEW_SCORE_GAP)
    {
        return None;
    }

    Some(best.clone())
}

fn character_alias_map_candidates(
    db_aliases: &[StoryCharacterAliasView],
) -> Vec<CharacterIdentityMergeCandidate> {
    let mut candidates =
        std::collections::HashMap::<String, CharacterIdentityMergeCandidate>::new();

    for alias in db_aliases {
        let target_key = alias.entity_key.trim();
        if target_key.is_empty() {
            continue;
        }

        let entry = candidates.entry(target_key.to_string()).or_insert_with(|| {
            CharacterIdentityMergeCandidate {
                target_key: target_key.to_string(),
                display_name: alias.display_name.clone(),
                aliases: Vec::new(),
                score: 0.0,
                source: "alias_map".to_string(),
                chapter_num: Some(alias.first_chapter_num),
            }
        });

        if alias.alias_type == "canonical_name" {
            entry.display_name = alias.alias_text.clone();
        } else {
            push_character_alias_if_valid(
                &mut entry.aliases,
                CharacterAlias {
                    text: alias.alias_text.clone(),
                    alias_type: alias.alias_type.clone(),
                    alias_label: alias.alias_label.clone(),
                    is_primary: alias.confidence.unwrap_or(0.0) >= 1.0,
                    evidence: alias.evidence.clone(),
                },
                &entry.display_name,
            );
        }

        if entry
            .chapter_num
            .is_none_or(|chapter_num| alias.first_chapter_num < chapter_num)
        {
            entry.chapter_num = Some(alias.first_chapter_num);
        }
    }

    candidates.into_values().collect()
}

fn identity_from_alias_map_candidate(
    candidate: CharacterIdentityMergeCandidate,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) -> CharacterIdentity {
    let mut identity = CharacterIdentity {
        name: candidate.display_name,
        aliases: candidate.aliases,
    };
    merge_observed_identity_aliases(&mut identity, observed_identity, known_name_keys);
    identity
}

fn push_character_merge_review_candidate(
    candidates: &mut std::collections::HashMap<String, CharacterIdentityMergeCandidate>,
    candidate: CharacterIdentityMergeCandidate,
) {
    if let Some(existing) = candidates.get_mut(&candidate.target_key) {
        if candidate.score > existing.score {
            existing.score = candidate.score;
        }
        if existing.chapter_num.is_none_or(|chapter_num| {
            candidate
                .chapter_num
                .is_some_and(|candidate_chapter_num| candidate_chapter_num < chapter_num)
        }) {
            existing.chapter_num = candidate.chapter_num;
        }
        for alias in candidate.aliases {
            push_character_alias_if_valid(&mut existing.aliases, alias, &existing.display_name);
        }
        return;
    }

    candidates.insert(candidate.target_key.clone(), candidate);
}

async fn confirm_character_identity_merge(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: &CharacterIdentity,
    candidate: &CharacterIdentityMergeCandidate,
) -> (CharacterIdentityMergeDecision, serde_json::Value) {
    let observed_identity_json =
        serde_json::to_string(identity).unwrap_or_else(|_| "{}".to_string());
    let candidate_identity_json =
        serde_json::to_string(candidate).unwrap_or_else(|_| "{}".to_string());
    let prompt = build_character_identity_merge_confirmation_prompt(
        chunk_input,
        &observed_identity_json,
        &candidate_identity_json,
    );

    match call_local_json_array::<CharacterIdentityMergeDecision>(
        state,
        &prompt,
        CHARACTER_IDENTITY_MERGE_CONFIRMATION_MAX_TOKENS,
    )
    .await
    {
        Ok((decisions, response)) => (
            decisions.into_iter().next().unwrap_or_else(|| {
                character_identity_merge_decision("create_new", 0.0, "LLM trả mảng rỗng.")
            }),
            json!(response),
        ),
        Err(error) => (
            character_identity_merge_decision(
                "create_new",
                0.0,
                "Không parse được JSON xác nhận merge; giữ nhân vật riêng để tránh nhập sai.",
            ),
            json!({
                "mode": "merge_confirmation_failed_non_blocking",
                "error": error.message,
            }),
        ),
    }
}

async fn confirm_character_identity_creation(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: &CharacterIdentity,
    candidates: &[CharacterIdentityMergeCandidate],
) -> (CharacterIdentityCreationDecision, serde_json::Value) {
    let observed_identity_json =
        serde_json::to_string(identity).unwrap_or_else(|_| "{}".to_string());
    let candidates_json = serde_json::to_string(candidates).unwrap_or_else(|_| "[]".to_string());
    let prompt = build_character_identity_creation_review_prompt(
        chunk_input,
        &observed_identity_json,
        &candidates_json,
    );

    match call_local_json_array::<CharacterIdentityCreationDecision>(
        state,
        &prompt,
        CHARACTER_IDENTITY_CREATION_REVIEW_MAX_TOKENS,
    )
    .await
    {
        Ok((decisions, response)) => (
            decisions.into_iter().next().unwrap_or_else(|| {
                character_identity_creation_decision(
                    "create_new",
                    None,
                    None,
                    0.0,
                    "LLM trả mảng rỗng.",
                )
            }),
            json!(response),
        ),
        Err(error) => (
            character_identity_creation_decision(
                "create_new",
                None,
                None,
                0.0,
                "Không parse được JSON kiểm tra nhân vật mới; giữ nhân vật riêng để tránh nhập sai.",
            ),
            json!({
                "mode": "identity_creation_review_failed_non_blocking",
                "error": error.message,
            }),
        ),
    }
}

fn character_identity_creation_decision(
    action: &str,
    target_key: Option<String>,
    target_name: Option<String>,
    confidence: f64,
    reason: &str,
) -> CharacterIdentityCreationDecision {
    CharacterIdentityCreationDecision {
        action: action.to_string(),
        target_key,
        target_name,
        confidence: Some(confidence),
        reason: Some(reason.to_string()),
        evidence: Vec::new(),
    }
}

fn character_identity_merge_decision(
    action: &str,
    confidence: f64,
    reason: &str,
) -> CharacterIdentityMergeDecision {
    CharacterIdentityMergeDecision {
        action: action.to_string(),
        confidence: Some(confidence),
        reason: Some(reason.to_string()),
    }
}

fn normalize_character_identity_merge_action(action: &str) -> &'static str {
    match normalize_ascii_snake_key(action).as_str() {
        "merge_existing" | "merge" | "merge_into_existing" => "merge_existing",
        "ignore" | "skip" => "ignore",
        _ => "create_new",
    }
}

fn normalize_character_identity_creation_action(action: &str) -> &'static str {
    match normalize_ascii_snake_key(action).as_str() {
        "merge_existing" | "merge" | "merge_into_existing" => "merge_existing",
        "reject" | "ignore" | "skip" => "reject",
        _ => "create_new",
    }
}

fn find_creation_review_target_candidate<'a>(
    decision: &CharacterIdentityCreationDecision,
    candidates: &'a [CharacterIdentityMergeCandidate],
) -> Option<&'a CharacterIdentityMergeCandidate> {
    if let Some(target_key) = decision
        .target_key
        .as_deref()
        .map(normalize_ascii_snake_key)
        .filter(|value| !value.is_empty())
    {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| normalize_ascii_snake_key(&candidate.target_key) == target_key)
        {
            return Some(candidate);
        }
    }

    let target_name = decision
        .target_name
        .as_deref()
        .map(normalized_text_key)
        .filter(|value| !value.is_empty())?;

    candidates.iter().find(|candidate| {
        normalized_text_key(&candidate.display_name) == target_name
            || candidate
                .aliases
                .iter()
                .any(|alias| normalized_text_key(&alias.text) == target_name)
    })
}

fn character_identity_candidate_score(
    identity: &CharacterIdentity,
    candidate: &CharacterIdentity,
) -> f64 {
    let identity_surfaces = character_resolution_surface_items(identity);
    let candidate_surfaces = character_resolution_surface_items(candidate);
    if identity_surfaces.is_empty() || candidate_surfaces.is_empty() {
        return 0.0;
    }

    let mut best_score: f64 = 0.0;
    for left in &identity_surfaces {
        for right in &candidate_surfaces {
            if left.key == right.key {
                if !left.is_canonical || !right.is_alias {
                    return 1.0;
                }

                best_score = best_score.max(CHARACTER_CANONICAL_STORED_ALIAS_NAME_MATCH_SCORE);
                continue;
            }

            let mut score = character_surface_similarity_score(&left.text, &right.text);
            if left.is_canonical && right.is_alias {
                score = score.min(CHARACTER_CANONICAL_STORED_ALIAS_NAME_MATCH_SCORE);
            }
            if score > best_score {
                best_score = score;
            }
        }
    }

    best_score
}

#[derive(Debug, Clone)]
struct CharacterResolutionSurface {
    text: String,
    key: String,
    is_canonical: bool,
    is_alias: bool,
}

fn character_resolution_surface_items(
    identity: &CharacterIdentity,
) -> Vec<CharacterResolutionSurface> {
    let mut surfaces = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let name = clean_character_surface(&identity.name);
    let name_key = normalized_folded_text_key(&name);
    if is_strong_character_resolution_key(&name_key) && seen.insert(name_key.clone()) {
        surfaces.push(CharacterResolutionSurface {
            text: name,
            key: name_key,
            is_canonical: true,
            is_alias: false,
        });
    }

    for alias in &identity.aliases {
        let alias_type = normalize_character_alias_type(&alias.alias_type);
        if !is_persistable_character_alias_type(&alias_type) {
            continue;
        }

        let surface = clean_character_surface(&alias.text);
        let key = normalized_folded_text_key(&surface);
        if is_strong_character_resolution_key(&key) && seen.insert(key.clone()) {
            surfaces.push(CharacterResolutionSurface {
                text: surface,
                key,
                is_canonical: false,
                is_alias: true,
            });
        }
    }

    surfaces
}

fn character_surface_similarity_score(left: &str, right: &str) -> f64 {
    let left_key = normalized_folded_text_key(left);
    let right_key = normalized_folded_text_key(right);
    if !is_strong_character_resolution_key(&left_key)
        || !is_strong_character_resolution_key(&right_key)
    {
        return 0.0;
    }
    if left_key == right_key {
        return 1.0;
    }

    let edit_score = levenshtein_similarity_score(&left_key, &right_key);
    let token_score = token_dice_score(&left_key, &right_key);
    let substring_score = character_substring_similarity_score(&left_key, &right_key);

    edit_score.max(token_score * 0.9).max(substring_score)
}

fn character_substring_similarity_score(left_key: &str, right_key: &str) -> f64 {
    if left_key == right_key {
        return 1.0;
    }

    let left_token_count = character_resolution_token_count(left_key);
    let right_token_count = character_resolution_token_count(right_key);
    let min_len = left_key.chars().count().min(right_key.chars().count());
    if min_len < 7 || left_token_count < 2 || right_token_count < 2 {
        return 0.0;
    }

    if left_key.contains(right_key) || right_key.contains(left_key) {
        return 0.93;
    }

    0.0
}

fn token_dice_score(left_key: &str, right_key: &str) -> f64 {
    let left_tokens = left_key
        .split('_')
        .filter(|token| !token.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let right_tokens = right_key
        .split('_')
        .filter(|token| !token.is_empty())
        .collect::<std::collections::HashSet<_>>();

    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let shared_count = left_tokens.intersection(&right_tokens).count();
    (2.0 * shared_count as f64) / (left_tokens.len() + right_tokens.len()) as f64
}

fn levenshtein_similarity_score(left: &str, right: &str) -> f64 {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let max_len = left_chars.len().max(right_chars.len());
    if max_len == 0 {
        return 0.0;
    }

    let distance = levenshtein_distance(&left_chars, &right_chars);
    1.0 - (distance as f64 / max_len as f64)
}

fn levenshtein_distance(left: &[char], right: &[char]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right.iter().enumerate() {
            let insert_cost = current[right_index] + 1;
            let delete_cost = previous[right_index + 1] + 1;
            let replace_cost = previous[right_index] + usize::from(left_char != right_char);
            current[right_index + 1] = insert_cost.min(delete_cost).min(replace_cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

fn is_strong_character_resolution_key(key: &str) -> bool {
    let char_count = key.chars().filter(|ch| *ch != '_').count();
    if char_count < 4 {
        return false;
    }

    character_resolution_token_count(key) >= 2 || char_count >= 6
}

fn character_resolution_token_count(key: &str) -> usize {
    key.split('_').filter(|token| !token.is_empty()).count()
}

fn identity_from_db_record(
    record: &StoryExtractionRecordView,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) -> CharacterIdentity {
    let mut identity = CharacterIdentity {
        name: record.display_name.clone(),
        aliases: aliases_from_record(record),
    };
    merge_observed_identity_aliases(&mut identity, observed_identity, known_name_keys);
    identity
}

fn identity_from_payload_record(
    record: &StoryExtractionRecordPayload,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) -> CharacterIdentity {
    let mut identity = CharacterIdentity {
        name: record.display_name.clone(),
        aliases: aliases_from_payload_record(record),
    };
    merge_observed_identity_aliases(&mut identity, observed_identity, known_name_keys);
    identity
}

fn merge_observed_identity_aliases(
    target: &mut CharacterIdentity,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) {
    let Some(observed_identity) = observed_identity else {
        return;
    };

    if normalized_text_key(&target.name) != normalized_text_key(&observed_identity.name) {
        let observed_name_key = normalized_text_key(&observed_identity.name);
        if !known_name_keys.contains(&observed_name_key) {
            push_character_alias_if_valid(
                &mut target.aliases,
                CharacterAlias {
                    text: observed_identity.name.clone(),
                    alias_type: "other_alias".to_string(),
                    alias_label: "Tên gọi khác".to_string(),
                    is_primary: false,
                    evidence: Vec::new(),
                },
                &target.name,
            );
        }
    }

    for alias in &observed_identity.aliases {
        if !is_persistable_character_alias_type(&alias.alias_type) {
            continue;
        }
        if known_name_keys.contains(&normalized_text_key(&alias.text))
            && normalized_text_key(&alias.text) != normalized_text_key(&target.name)
        {
            continue;
        }
        push_character_alias_if_valid(&mut target.aliases, alias.clone(), &target.name);
    }
}

fn sanitize_new_character_identity(
    identity: CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> CharacterIdentity {
    let blocked_alias_keys = known_character_name_keys(db_records, working_document)
        .into_iter()
        .filter(|key| *key != normalized_text_key(&identity.name))
        .collect::<std::collections::HashSet<_>>();

    let mut sanitized = CharacterIdentity {
        name: identity.name,
        aliases: Vec::new(),
    };

    for alias in identity.aliases {
        let alias_type = normalize_character_alias_type(&alias.alias_type);
        if !is_persistable_character_alias_type(&alias_type) {
            continue;
        }
        if blocked_alias_keys.contains(&normalized_text_key(&alias.text)) {
            continue;
        }
        push_character_alias_if_valid(&mut sanitized.aliases, alias, &sanitized.name);
    }

    sanitized
}

fn known_character_name_keys(
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();

    for record in db_records {
        keys.insert(normalized_text_key(&record.display_name));
    }
    for record in &working_document.records {
        keys.insert(normalized_text_key(&record.display_name));
    }

    keys.retain(|key| !key.is_empty());
    keys
}

fn merge_character_identity_into_list(
    identities: &mut Vec<CharacterIdentity>,
    source: CharacterIdentity,
) {
    if let Some(target) = identities
        .iter_mut()
        .find(|identity| normalized_text_key(&identity.name) == normalized_text_key(&source.name))
    {
        for alias in source.aliases {
            push_character_alias_if_valid(&mut target.aliases, alias, &target.name);
        }
        return;
    }

    identities.push(source);
}

fn push_character_alias_if_valid(
    aliases: &mut Vec<CharacterAlias>,
    alias: CharacterAlias,
    canonical_name: &str,
) {
    let raw_text = alias.text.trim().to_string();
    let text = clean_character_surface(&alias.text);
    if text.is_empty()
        || normalized_text_key(&text) == normalized_text_key(canonical_name)
        || !is_stable_character_alias_surface(&raw_text, &text)
    {
        return;
    }

    let alias_type = normalize_character_alias_type(&alias.alias_type);
    if !is_persistable_character_alias_type(&alias_type) {
        return;
    }
    if character_alias_surface_contains_canonical_name(&text, canonical_name) {
        return;
    }
    if !character_surface_has_uppercase_token(&text)
        && !alias_has_direct_quoted_evidence(&alias, &text)
    {
        return;
    }

    if let Some(existing) = aliases
        .iter_mut()
        .find(|existing| normalized_text_key(&existing.text) == normalized_text_key(&text))
    {
        if existing.evidence.is_empty() && !alias.evidence.is_empty() {
            existing.evidence = alias.evidence;
        }
        existing.is_primary = existing.is_primary || alias.is_primary;
        return;
    }

    aliases.push(CharacterAlias {
        text,
        alias_label: normalize_character_alias_label(&alias_type, &alias.alias_label),
        alias_type,
        is_primary: alias.is_primary,
        evidence: alias.evidence,
    });
}

fn is_stable_character_alias_surface(raw_text: &str, clean_text: &str) -> bool {
    let raw = raw_text.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean = clean_text.split_whitespace().collect::<Vec<_>>().join(" ");
    if raw.is_empty() || clean.is_empty() {
        return false;
    }

    let raw_key = normalized_text_key(&raw);
    let clean_key = normalized_text_key(&clean);
    if raw_key.is_empty() || clean_key.is_empty() {
        return false;
    }

    let tokens = clean.split_whitespace().collect::<Vec<_>>();
    let token_count = tokens.len();
    let char_count = clean.chars().filter(|ch| ch.is_alphanumeric()).count();
    let has_uppercase_token = character_surface_has_uppercase_token(&clean);

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

fn character_surface_has_uppercase_token(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|token| token.chars().next().is_some_and(char::is_uppercase))
}

fn character_alias_surface_contains_canonical_name(alias_text: &str, canonical_name: &str) -> bool {
    let alias_key = normalized_text_key(alias_text);
    let canonical_key = normalized_text_key(canonical_name);
    !alias_key.is_empty()
        && !canonical_key.is_empty()
        && alias_key != canonical_key
        && alias_key.contains(&canonical_key)
}

fn alias_has_direct_quoted_evidence(alias: &CharacterAlias, alias_text: &str) -> bool {
    let alias_key = normalized_text_key(alias_text);
    if alias_key.is_empty() {
        return false;
    }

    alias.evidence.iter().any(|evidence| {
        evidence.quote.as_deref().is_some_and(|quote| {
            quoted_alias_spans(quote)
                .iter()
                .any(|span| normalized_text_key(&clean_character_surface(&span.text)) == alias_key)
        })
    })
}

fn identity_to_record(
    identity: &CharacterIdentity,
    chapter_num: i64,
) -> StoryExtractionRecordPayload {
    let mut fields: Vec<StoryExtractionFieldPayload> = Vec::new();
    for alias in &identity.aliases {
        let field_key = normalize_character_alias_type(&alias.alias_type);
        let field_label = normalize_character_alias_label(&field_key, &alias.alias_label);
        let evidence = alias
            .evidence
            .iter()
            .filter(|evidence| evidence.chapter_num == chapter_num)
            .cloned()
            .collect::<Vec<_>>();
        let value = StoryExtractionFieldValuePayload {
            value: alias.text.clone(),
            confidence: Some(if alias.is_primary { 1.0 } else { 0.95 }),
            related_character: None,
            relationship_type: None,
            relationship_label: None,
            relationship_direction: None,
            evidence,
        };

        if let Some(field) = fields
            .iter_mut()
            .find(|field| normalize_ascii_snake_key(&field.field_key) == field_key)
        {
            field.values.push(value);
        } else {
            fields.push(StoryExtractionFieldPayload {
                field_key,
                field_label,
                values: vec![value],
            });
        }
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

fn working_identities_for_chunk(
    document: &StoryExtractionDocument,
    chunk_identities: &[CharacterIdentity],
) -> Vec<CharacterIdentity> {
    let mut identities = Vec::new();

    for record in &document.records {
        if !chunk_identities
            .iter()
            .any(|identity| payload_record_matches_identity(record, identity))
        {
            continue;
        }

        identities.push(CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_payload_record(record),
        });
    }

    identities
}

fn hydrate_identities_with_alias_map(
    mut identities: Vec<CharacterIdentity>,
    db_aliases: &[StoryCharacterAliasView],
) -> Vec<CharacterIdentity> {
    if identities.is_empty() || db_aliases.is_empty() {
        return identities;
    }

    let alias_candidates = character_alias_map_candidates(db_aliases);
    for identity in &mut identities {
        let identity_key = normalize_ascii_snake_key(&identity.name);
        let Some(candidate) = alias_candidates.iter().find(|candidate| {
            candidate.target_key == identity_key
                || normalized_text_key(&candidate.display_name)
                    == normalized_text_key(&identity.name)
                || candidate.aliases.iter().any(|alias| {
                    normalized_text_key(&alias.text) == normalized_text_key(&identity.name)
                })
        }) else {
            continue;
        };

        if normalized_text_key(&candidate.display_name) != normalized_text_key(&identity.name) {
            push_character_alias_if_valid(
                &mut identity.aliases,
                CharacterAlias {
                    text: candidate.display_name.clone(),
                    alias_type: "other_alias".to_string(),
                    alias_label: "Tên gọi khác".to_string(),
                    is_primary: false,
                    evidence: Vec::new(),
                },
                &identity.name,
            );
        }

        for alias in &candidate.aliases {
            push_character_alias_if_valid(&mut identity.aliases, alias.clone(), &identity.name);
        }
    }

    identities
}

fn aliases_from_record(record: &StoryExtractionRecordView) -> Vec<CharacterAlias> {
    let mut aliases = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for field in &record.fields {
        let field_key = normalize_ascii_snake_key(&field.field_key);
        if !is_character_alias_field_key(&field_key) {
            continue;
        }

        for value in &field.values {
            let alias = value.value.trim();
            if alias.is_empty() {
                continue;
            }
            if seen.insert(normalized_text_key(alias)) {
                aliases.push(CharacterAlias {
                    text: alias.to_string(),
                    alias_type: normalize_character_alias_type(&field_key),
                    alias_label: field.field_label.clone(),
                    is_primary: value.confidence.unwrap_or(0.0) >= 1.0,
                    evidence: value.evidence.clone(),
                });
            }
        }
    }

    aliases
}

fn aliases_from_payload_record(record: &StoryExtractionRecordPayload) -> Vec<CharacterAlias> {
    let mut aliases = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for field in &record.fields {
        let field_key = normalize_ascii_snake_key(&field.field_key);
        if !is_character_alias_field_key(&field_key) {
            continue;
        }

        for value in &field.values {
            let alias = value.value.trim();
            if alias.is_empty() {
                continue;
            }
            if seen.insert(normalized_text_key(alias)) {
                aliases.push(CharacterAlias {
                    text: alias.to_string(),
                    alias_type: normalize_character_alias_type(&field_key),
                    alias_label: field.field_label.clone(),
                    is_primary: value.confidence.unwrap_or(0.0) >= 1.0,
                    evidence: value.evidence.clone(),
                });
            }
        }
    }

    aliases
}

fn is_character_alias_field_key(field_key: &str) -> bool {
    matches!(
        field_key,
        "alias" | "aliases" | "other_alias" | "other_name" | "other_names" | "nickname"
    )
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
        if !is_character_alias_field_key(&field_key) {
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
        .any(|alias| identity_names.contains(&normalized_text_key(&alias.text)))
}

fn character_identity_surface_keys(
    identity: &CharacterIdentity,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    keys.insert(normalized_text_key(&identity.name));
    for alias in &identity.aliases {
        keys.insert(normalized_text_key(&alias.text));
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

fn normalize_character_field_payloads(
    fields: Vec<StoryExtractionFieldPayload>,
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> Vec<StoryExtractionFieldPayload> {
    let mut normalized_fields = Vec::new();
    let identity_value_keys = character_identity_value_keys(identity);
    let other_character_value_keys =
        other_character_value_keys(identity, db_records, working_document);

    for mut field in fields {
        let Some((field_key, field_label)) = normalize_character_minimal_field(&field.field_key)
        else {
            continue;
        };
        field.field_key = field_key.to_string();
        field.field_label = field_label.to_string();

        let mut values = Vec::new();
        for mut value in field.values {
            value.value = value.value.trim().to_string();
            if value.value.is_empty() {
                continue;
            }

            if identity_value_keys.contains(&normalized_text_key(&value.value)) {
                continue;
            }
            if other_character_value_keys.contains(&normalized_text_key(&value.value)) {
                continue;
            }
            if !field_value_has_quoted_evidence(&value) {
                continue;
            }
            if !field_value_has_target_marker(&value) {
                continue;
            }
            if !field_value_has_minimum_confidence(&value) {
                continue;
            }
            if !field_value_is_grounded_in_quoted_evidence(&value) {
                continue;
            }
            strip_character_field_markers_from_evidence(&mut value);
            value.related_character = None;
            value.relationship_type = None;
            value.relationship_label = None;
            value.relationship_direction = None;

            values.push(value);
        }

        if values.is_empty() {
            continue;
        }

        field.values = values;
        normalized_fields.push(field);
    }

    normalized_fields
}

async fn verify_character_field_payloads(
    state: &AppState,
    field_input: Option<&DraftExtractionInput>,
    identity: &CharacterIdentity,
    character_json: &str,
    fields: Vec<StoryExtractionFieldPayload>,
) -> (Vec<StoryExtractionFieldPayload>, serde_json::Value) {
    let Some(field_input) = field_input else {
        return (
            fields,
            json!({
                "mode": "skipped_no_target_context",
            }),
        );
    };

    let mut verified_fields = Vec::new();
    let mut reports = Vec::new();

    for mut field in fields {
        let mut verified_values = Vec::new();
        for value in field.values {
            let field_value_json = serde_json::to_string(&json!({
                "field_key": &field.field_key,
                "field_label": &field.field_label,
                "value": &value.value,
                "confidence": value.confidence,
                "evidence": &value.evidence,
            }))
            .unwrap_or_else(|_| "{}".to_string());
            let prompt = build_character_field_value_verification_prompt(
                field_input,
                character_json,
                &field_value_json,
            );
            let verification_result = call_local_json_array::<CharacterFieldValueVerification>(
                state,
                &prompt,
                CHARACTER_FIELD_VALUE_VERIFICATION_MAX_TOKENS,
            )
            .await;

            match verification_result {
                Ok((verifications, response)) => {
                    let verification = verifications.into_iter().next();
                    let accepted = verification.as_ref().is_some_and(|verification| {
                        field_value_verification_accepts(identity, verification)
                    });
                    reports.push(json!({
                        "field_key": &field.field_key,
                        "field_label": &field.field_label,
                        "value": &value.value,
                        "accepted": accepted,
                        "verification": verification,
                        "response": response,
                    }));

                    if accepted {
                        verified_values.push(value);
                    }
                }
                Err(error) => {
                    reports.push(json!({
                        "field_key": &field.field_key,
                        "field_label": &field.field_label,
                        "value": &value.value,
                        "accepted": false,
                        "mode": "field_value_verification_failed",
                        "error": error.message,
                    }));
                }
            }
        }

        if verified_values.is_empty() {
            continue;
        }
        field.values = verified_values;
        verified_fields.push(field);
    }

    (
        verified_fields,
        json!({
            "mode": "field_value_verification",
            "checks": reports,
        }),
    )
}

fn field_value_verification_accepts(
    identity: &CharacterIdentity,
    verification: &CharacterFieldValueVerification,
) -> bool {
    verification.accepted
        && verification.confidence.unwrap_or(0.0)
            >= CHARACTER_FIELD_VALUE_VERIFICATION_MIN_CONFIDENCE
        && field_value_semantic_class_is_allowed(&verification.semantic_class)
        && field_value_verification_owner_matches_identity(identity, verification)
}

fn field_value_semantic_class_is_allowed(semantic_class: &str) -> bool {
    matches!(
        normalize_ascii_snake_key(semantic_class).as_str(),
        "physical_appearance" | "clothing" | "age_or_build" | "appearance"
    )
}

fn field_value_verification_owner_matches_identity(
    identity: &CharacterIdentity,
    verification: &CharacterFieldValueVerification,
) -> bool {
    let Some(owner_name) = verification
        .owner_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return true;
    };

    character_identity_surfaces(identity)
        .into_iter()
        .any(|(surface, _)| normalized_text_key(&surface) == normalized_text_key(owner_name))
}

fn strip_character_field_markers_from_evidence(value: &mut StoryExtractionFieldValuePayload) {
    for evidence in &mut value.evidence {
        if let Some(quote) = &mut evidence.quote {
            if quote.contains("[[") || quote.contains("]]") {
                *quote = quote.replace("[[", "").replace("]]", "");
            }
        }
    }
}

fn normalize_character_minimal_field(field_key: &str) -> Option<(&'static str, &'static str)> {
    match normalize_ascii_snake_key(field_key).as_str() {
        "appearance" => Some(("appearance", "Ngoại hình")),
        _ => None,
    }
}

fn character_identity_value_keys(
    identity: &CharacterIdentity,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    keys.insert(normalized_text_key(&identity.name));

    for alias in &identity.aliases {
        keys.insert(normalized_text_key(&alias.text));
    }

    keys.retain(|key| !key.is_empty());
    keys
}

fn other_character_value_keys(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();

    for record in db_records {
        if view_record_matches_identity(record, identity) {
            continue;
        }
        keys.insert(normalized_text_key(&record.display_name));
        for alias in aliases_from_record(record) {
            keys.insert(normalized_text_key(&alias.text));
        }
    }

    for record in &working_document.records {
        if payload_record_matches_identity(record, identity)
            || normalized_text_key(&record.display_name) == normalized_text_key(&identity.name)
        {
            continue;
        }
        keys.insert(normalized_text_key(&record.display_name));
        for alias in aliases_from_payload_record(record) {
            keys.insert(normalized_text_key(&alias.text));
        }
    }

    keys.retain(|key| !key.is_empty());
    keys
}

fn field_value_has_quoted_evidence(value: &StoryExtractionFieldValuePayload) -> bool {
    value.evidence.iter().any(|evidence| {
        evidence
            .quote
            .as_deref()
            .is_some_and(|quote| !quote.trim().is_empty())
    })
}

fn field_value_has_target_marker(value: &StoryExtractionFieldValuePayload) -> bool {
    value.evidence.iter().any(|evidence| {
        evidence.quote.as_deref().is_some_and(|quote| {
            let quote = quote.trim();
            quote.contains("[[") && quote.contains("]]")
        })
    })
}

fn field_value_has_minimum_confidence(value: &StoryExtractionFieldValuePayload) -> bool {
    value
        .confidence
        .is_some_and(|confidence| confidence >= CHARACTER_FIELD_VALUE_MIN_CONFIDENCE)
}

fn field_value_is_grounded_in_quoted_evidence(value: &StoryExtractionFieldValuePayload) -> bool {
    let value_tokens = normalized_field_value_tokens(&value.value);
    if value_tokens.is_empty() {
        return false;
    }

    value.evidence.iter().any(|evidence| {
        let Some(quote) = evidence.quote.as_deref() else {
            return false;
        };
        let quote_tokens = normalized_field_value_tokens(quote);
        if quote_tokens.is_empty() {
            return false;
        }

        let overlap_count = value_tokens
            .iter()
            .filter(|token| quote_tokens.contains(*token))
            .count();

        if value_tokens.len() <= 2 {
            overlap_count == value_tokens.len()
        } else {
            overlap_count * 5 >= value_tokens.len() * 3
        }
    })
}

fn normalized_field_value_tokens(value: &str) -> std::collections::HashSet<String> {
    normalized_folded_text_key(value)
        .split('_')
        .filter(|token| token.chars().count() >= 2)
        .map(str::to_string)
        .collect()
}

fn field_value_has_relationship_metadata(value: &StoryExtractionFieldValuePayload) -> bool {
    value
        .related_character
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        || value
            .relationship_type
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        || value
            .relationship_label
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        || value
            .relationship_direction
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
}

fn is_relationship_field_key(field_key: &str) -> bool {
    matches!(
        field_key,
        "relationship"
            | "relationships"
            | "relation"
            | "relations"
            | "family_relation"
            | "social_relation"
            | "character_relationship"
            | "character_relationships"
    )
}

fn is_valid_relationship_field_value(value: &StoryExtractionFieldValuePayload) -> bool {
    value
        .related_character
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        && value
            .relationship_type
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        && value
            .relationship_label
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        && value
            .relationship_direction
            .as_deref()
            .is_some_and(|direction| matches!(direction, "self_to_related" | "related_to_self"))
        && value.evidence.iter().any(|evidence| {
            evidence
                .quote
                .as_deref()
                .is_some_and(|quote| !quote.trim().is_empty())
        })
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

    document
        .records
        .push(identity_to_record(identity, document.chapter_num));
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

    let mut values = Vec::new();
    for source_value in source.values {
        if let Some(source_value) =
            merge_duplicate_character_field_value(target, &source_key, source_value)
        {
            values.push(source_value);
        }
    }

    if values.is_empty() {
        return;
    }

    target.push(StoryExtractionFieldPayload {
        field_key: source_key,
        field_label: source.field_label,
        values,
    });
}

fn merge_duplicate_character_field_value(
    target: &mut [StoryExtractionFieldPayload],
    source_key: &str,
    source_value: StoryExtractionFieldValuePayload,
) -> Option<StoryExtractionFieldValuePayload> {
    if field_value_has_relationship_metadata(&source_value) || is_relationship_field_key(source_key)
    {
        return Some(source_value);
    }

    let source_value_key = normalized_text_key(&source_value.value);
    if source_value_key.is_empty() {
        return None;
    }

    for field in target {
        let target_key = normalize_ascii_snake_key(&field.field_key);
        if target_key == source_key || is_relationship_field_key(&target_key) {
            continue;
        }

        for target_value in &mut field.values {
            if normalized_text_key(&target_value.value) != source_value_key {
                continue;
            }

            merge_character_field_value_metadata(target_value, &source_value);
            merge_story_evidence(&mut target_value.evidence, source_value.evidence);
            return None;
        }
    }

    Some(source_value)
}

fn merge_character_field_values(
    target: &mut Vec<StoryExtractionFieldValuePayload>,
    source: Vec<StoryExtractionFieldValuePayload>,
) {
    let mut value_index_by_key = target
        .iter()
        .enumerate()
        .map(|(index, value)| (character_field_value_key(value), index))
        .collect::<HashMap<_, _>>();

    for source_value in source {
        let key = character_field_value_key(&source_value);
        if let Some(index) = value_index_by_key.get(&key).copied() {
            merge_character_field_value_metadata(&mut target[index], &source_value);
            merge_story_evidence(&mut target[index].evidence, source_value.evidence);
        } else {
            let index = target.len();
            value_index_by_key.insert(key, index);
            target.push(source_value);
        }
    }
}

fn character_field_value_key(value: &StoryExtractionFieldValuePayload) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        normalized_text_key(&value.value),
        value
            .related_character
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default(),
        value
            .relationship_type
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default(),
        value
            .relationship_label
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default(),
        value
            .relationship_direction
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default()
    )
}

fn merge_character_field_value_metadata(
    target: &mut StoryExtractionFieldValuePayload,
    source: &StoryExtractionFieldValuePayload,
) {
    if target.related_character.is_none() {
        target.related_character = source.related_character.clone();
    }
    if target.relationship_type.is_none() {
        target.relationship_type = source.relationship_type.clone();
    }
    if target.relationship_label.is_none() {
        target.relationship_label = source.relationship_label.clone();
    }
    if target.relationship_direction.is_none() {
        target.relationship_direction = source.relationship_direction.clone();
    }
    if target.confidence.is_none() {
        target.confidence = source.confidence;
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

fn normalized_folded_text_key(value: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = true;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            last_was_separator = false;
        } else if let Some(ascii) = fold_key_char(ch) {
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

fn validate_character_record(
    record: &StoryExtractionRecordPayload,
    chapter_num: i64,
    chapter_text: &str,
) -> Result<(), ApiError> {
    let group_key = record.group_key.trim();
    if !matches!(group_key, "character" | "relationship") {
        return Err(ApiError::bad_request(
            "story extraction records must use group_key character or relationship",
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

    if group_key == "character" {
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

            if is_relationship_field_key(&normalize_ascii_snake_key(&field.field_key))
                && !is_valid_relationship_field_value(value)
            {
                return Err(ApiError::bad_request(
                    "character relationship fields require related_character, relationship_type, relationship_label, relationship_direction, and quoted evidence",
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

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
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
