use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Novel {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub author: Option<String>,
    pub source_language: Option<String>,
    pub genre: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chapter {
    pub id: String,
    pub novel_id: String,
    pub chapter_num: i64,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSegment {
    pub id: String,
    pub novel_id: String,
    pub chapter_id: String,
    pub segment_index: i64,
    pub start_char: i64,
    pub end_char: i64,
    pub segment_kind: String,
    pub text: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalysisJob {
    pub id: String,
    pub project_id: String,
    pub novel_id: Option<String>,
    pub job_type: String,
    pub status: String,
    pub payload_json: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalysisChapterRun {
    pub id: String,
    pub project_id: String,
    pub job_id: String,
    pub novel_id: String,
    pub chapter_id: String,
    pub chapter_num: i64,
    pub status: String,
    pub attempt: i64,
    pub prompt_schema_version: Option<String>,
    pub output_json: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisExecutionProfile {
    LocalSmallStaged,
    CloudGeminiOneShot,
}

impl AnalysisExecutionProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalSmallStaged => "local_small_staged",
            Self::CloudGeminiOneShot => "cloud_gemini_one_shot",
        }
    }
}

impl Default for AnalysisExecutionProfile {
    fn default() -> Self {
        Self::LocalSmallStaged
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisChapterState {
    pub chapter_id: String,
    pub chapter_num: i64,
    pub title: String,
    pub status: String,
    pub run_id: Option<String>,
    pub attempt: Option<i64>,
    pub prompt_schema_version: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub updated_at: Option<String>,
    pub execution_profile: Option<String>,
    pub call_status: Option<String>,
    pub api_call_count: Option<i64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost: Option<f64>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisRunSnapshot {
    pub job: AnalysisJob,
    pub total_chapters: usize,
    pub completed_chapters: usize,
    pub running_chapters: usize,
    pub failed_chapters: usize,
    pub pending_chapters: usize,
    pub next_chapter_num: Option<i64>,
    pub paused_reason: Option<String>,
    pub chapters: Vec<AnalysisChapterState>,
    pub character_aliases: Vec<StoryCharacterAliasView>,
    pub character_records: Vec<StoryExtractionRecordView>,
    pub relationship_records: Vec<StoryExtractionRecordView>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisRunStepInput {
    #[serde(default)]
    pub force: bool,
    pub from_chapter_num: Option<i64>,
    pub to_chapter_num: Option<i64>,
    #[serde(default)]
    pub execution_profile: Option<AnalysisExecutionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryExtractionDocument {
    pub schema_version: String,
    pub chapter_num: i64,
    #[serde(default)]
    pub records: Vec<StoryExtractionRecordPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryExtractionRecordPayload {
    pub group_key: String,
    pub group_label: String,
    pub entity_key: Option<String>,
    pub display_name: String,
    #[serde(default)]
    pub mentions: Vec<StoryCharacterMention>,
    #[serde(default)]
    pub fields: Vec<StoryExtractionFieldPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryCharacterMention {
    pub text: String,
    pub start_char: i64,
    pub end_char: i64,
    pub mention_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryExtractionFieldPayload {
    pub field_key: String,
    pub field_label: String,
    #[serde(default)]
    pub values: Vec<StoryExtractionFieldValuePayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryExtractionFieldValuePayload {
    pub value: String,
    pub confidence: Option<f64>,
    #[serde(default)]
    pub related_character: Option<String>,
    #[serde(default)]
    pub relationship_type: Option<String>,
    #[serde(default)]
    pub relationship_label: Option<String>,
    #[serde(default)]
    pub relationship_direction: Option<String>,
    #[serde(default)]
    pub evidence: Vec<StoryEvidenceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryEvidenceSpan {
    pub chapter_num: i64,
    pub start_char: Option<i64>,
    pub end_char: Option<i64>,
    pub quote: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryCharacterAliasView {
    pub id: String,
    pub project_id: String,
    pub novel_id: String,
    pub job_id: String,
    pub entity_key: String,
    pub display_name: String,
    pub alias_text: String,
    pub alias_key: String,
    pub alias_type: String,
    pub alias_label: String,
    pub confidence: Option<f64>,
    pub first_chapter_num: i64,
    pub evidence: Vec<StoryEvidenceSpan>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryExtractionRecordView {
    pub id: String,
    pub project_id: String,
    pub novel_id: String,
    pub chapter_id: String,
    pub job_id: String,
    pub run_id: String,
    pub chapter_num: i64,
    pub group_key: String,
    pub group_label: String,
    pub entity_key: Option<String>,
    pub display_name: String,
    pub prompt_schema_version: String,
    pub mentions: Vec<StoryCharacterMention>,
    pub fields: Vec<StoryExtractionFieldView>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryExtractionFieldView {
    pub id: String,
    pub record_id: String,
    pub field_key: String,
    pub field_label: String,
    pub values: Vec<StoryExtractionFieldValueView>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryExtractionFieldValueView {
    pub id: String,
    pub field_id: String,
    pub value: String,
    pub confidence: Option<f64>,
    pub related_character: Option<String>,
    pub relationship_type: Option<String>,
    pub relationship_label: Option<String>,
    pub relationship_direction: Option<String>,
    pub evidence: Vec<StoryEvidenceSpan>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationJob {
    pub id: String,
    pub project_id: String,
    pub novel_id: String,
    pub source_language: Option<String>,
    pub target_language: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub payload_json: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByokProviderPreset {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub default_model: String,
    pub models: Vec<String>,
    pub api_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByokProviderConfigRecord {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub display_name: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_format: String,
    pub encrypted_secret_ref: Option<String>,
    pub key_fingerprint: Option<String>,
    pub session_only: bool,
    pub last_checked_at: Option<String>,
    pub last_health_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByokProviderConfigView {
    pub provider: String,
    pub display_name: String,
    pub base_url: String,
    pub model: String,
    pub api_format: String,
    pub has_api_key: bool,
    pub api_key_masked: String,
    pub key_fingerprint: Option<String>,
    pub session_only: bool,
    pub last_checked_at: Option<String>,
    pub last_health_status: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveByokProviderConfigInput {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    #[serde(default)]
    pub session_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveByokProviderConfigResult {
    pub config: ByokProviderConfigView,
    pub saved_api_key: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckByokProviderKeyInput {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByokProviderKeyHealth {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub valid: bool,
    pub status_code: Option<u16>,
    pub message: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobEvent {
    pub id: String,
    pub project_id: String,
    pub job_id: String,
    pub job_kind: String,
    pub sequence: i64,
    pub event_type: String,
    pub payload_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteProjectInput {
    pub purge_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteProjectResult {
    pub project_id: String,
    pub action: String,
    pub data_retained: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTranslationJobInput {
    pub novel_id: String,
    pub source_language: Option<String>,
    pub target_language: String,
    pub provider: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelImportInput {
    pub title: String,
    pub author: Option<String>,
    pub source_language: Option<String>,
    pub genre: Option<String>,
    pub description: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NovelMetadataUpdateInput {
    pub title: Option<String>,
    pub author: Option<String>,
    pub source_language: Option<String>,
    pub genre: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovelMetadataSuggestionInput {
    pub title: Option<String>,
    pub author: Option<String>,
    pub source_language: Option<String>,
    pub genre: Option<String>,
    pub description: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovelMetadataSuggestion {
    pub title: Option<String>,
    pub author: Option<String>,
    pub source_language: Option<String>,
    pub genre: Option<String>,
    pub description: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChapterPreview {
    pub chapter_num: i64,
    pub title: String,
    pub start_char: usize,
    pub end_char: usize,
    pub char_count: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ImportPreview {
    pub title: String,
    pub total_chars: usize,
    pub chapter_count: usize,
    pub chapters: Vec<ChapterPreview>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NovelImportResult {
    pub novel: Novel,
    pub chapters: Vec<Chapter>,
    pub source_segment_count: usize,
    pub analysis_job: AnalysisJob,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectWorkspaceSnapshot {
    pub project: Project,
    pub novels: Vec<Novel>,
    pub active_novel: Option<Novel>,
    pub chapters: Vec<Chapter>,
    pub latest_analysis_job: Option<AnalysisJob>,
    pub latest_analysis_chapters: Vec<AnalysisChapterState>,
    pub latest_job_events: Vec<JobEvent>,
    pub character_aliases: Vec<StoryCharacterAliasView>,
    pub character_records: Vec<StoryExtractionRecordView>,
    pub relationship_records: Vec<StoryExtractionRecordView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalLlmModelSelection {
    pub source_kind: String,
    pub display_name: String,
    pub path: String,
    pub preset_id: Option<String>,
    pub size_bytes: Option<u64>,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalLlmPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub size_label: String,
    pub source_url: String,
    pub installed: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalLlmDownloadState {
    pub preset_id: String,
    pub preset_name: String,
    pub filename: String,
    pub target_path: String,
    pub status: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub auto_activate: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalLlmRuntimeSnapshot {
    pub base_url: String,
    pub default_model_alias: String,
    pub server_binary: String,
    pub models_dir: String,
    pub server_running: bool,
    pub server_pid: Option<u32>,
    pub selected_model: Option<LocalLlmModelSelection>,
    pub downloaded_models: Vec<LocalLlmModelSelection>,
    pub presets: Vec<LocalLlmPreset>,
    pub active_download: Option<LocalLlmDownloadState>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActivateManagedLocalModelInput {
    pub path: String,
}
