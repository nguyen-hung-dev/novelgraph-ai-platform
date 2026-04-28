use axum::{
    extract::{Path, State},
    Json, Router,
};
mod errors;
pub mod local_runtime;
mod routes;
mod services;
pub(crate) use errors::ApiError;
use local_runtime::LocalLlmRuntimeManager;
use novelgraph_ai::LlamaCppClient;
use novelgraph_core::{
    build_character_alias_ownership_prompt, build_character_candidate_prompt,
    build_character_field_value_verification_prompt, build_character_fields_prompt,
    build_character_identity_creation_review_prompt,
    build_character_identity_merge_confirmation_prompt, build_character_identity_prompt,
    build_character_occurrence_confirmation_prompt, build_character_relationship_candidate_prompt,
    build_character_relationship_verification_prompt, AnalysisRunSnapshot, AnalysisRunStepInput,
    AppConfig, Chapter, DraftExtractionInput, Novel, StoryCharacterAliasView,
    StoryCharacterMention, StoryEvidenceSpan, StoryExtractionDocument, StoryExtractionFieldPayload,
    StoryExtractionFieldValuePayload, StoryExtractionRecordPayload, StoryExtractionRecordView,
    CHARACTER_EXTRACTION_SCHEMA_VERSION,
};
use novelgraph_storage::SqliteStore;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: SqliteStore,
    pub local_llm: LlamaCppClient,
    pub local_runtime: LocalLlmRuntimeManager,
    realtime_tx: broadcast::Sender<ProjectRealtimeEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectRealtimeEvent {
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
        .merge(routes::health::router())
        .merge(routes::local_llm::router())
        .merge(routes::byok::router())
        .merge(routes::projects::router())
        .merge(routes::realtime::router())
        .merge(routes::novels::router())
        .merge(routes::translation::router())
        .merge(routes::jobs::router())
        .merge(routes::analysis::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub(crate) fn publish_project_event(
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

pub(crate) async fn run_next_analysis_chapter(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
    Json(input): Json<AnalysisRunStepInput>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    Ok(Json(
        services::analysis_pipeline::run_next_analysis_chapter(&state, &project_id, &job_id, input)
            .await?,
    ))
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

#[derive(Debug, Clone)]
struct QuotedAliasSpan {
    start_char: i64,
    end_char: i64,
    text: String,
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

    if clean.is_empty()
        || clean.chars().count() < 2
        || clean.chars().count() > 64
        || raw.contains('\n')
        || raw.contains('\r')
    {
        return false;
    }

    if raw.contains(':')
        || raw.contains(';')
        || raw.contains('[')
        || raw.contains(']')
        || raw.contains('{')
        || raw.contains('}')
    {
        return false;
    }

    if clean.split_whitespace().count() > 7 {
        return false;
    }

    if clean
        .chars()
        .any(|ch| matches!(ch, '(' | ')' | '<' | '>' | '/' | '\\'))
    {
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
    let alias_key = normalized_folded_text_key(alias_text);
    let canonical_key = normalized_folded_text_key(canonical_name);
    !alias_key.is_empty() && !canonical_key.is_empty() && alias_key.contains(&canonical_key)
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
