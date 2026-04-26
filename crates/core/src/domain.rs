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
    pub text: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectWorkspaceSnapshot {
    pub project: Project,
    pub novels: Vec<Novel>,
    pub active_novel: Option<Novel>,
    pub chapters: Vec<Chapter>,
    pub latest_analysis_job: Option<AnalysisJob>,
    pub latest_job_events: Vec<JobEvent>,
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
