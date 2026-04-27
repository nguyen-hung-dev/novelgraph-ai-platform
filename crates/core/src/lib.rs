pub mod config;
pub mod domain;
pub mod error;
pub mod extraction;
pub mod import;
pub mod version;

pub use config::{AppConfig, AppMode};
pub use domain::{
    ActivateManagedLocalModelInput, AnalysisChapterRun, AnalysisChapterState, AnalysisJob,
    AnalysisRunSnapshot, AnalysisRunStepInput, Chapter, ChapterPreview, CreateProjectInput,
    CreateTranslationJobInput, DeleteProjectInput, DeleteProjectResult, ImportPreview, JobEvent,
    LocalLlmDownloadState, LocalLlmModelSelection, LocalLlmPreset, LocalLlmRuntimeSnapshot, Novel,
    NovelImportInput, NovelImportResult, Project, ProjectWorkspaceSnapshot, SourceSegment,
    StoryCharacterMention, StoryEvidenceSpan, StoryExtractionDocument, StoryExtractionFieldPayload,
    StoryExtractionFieldValuePayload, StoryExtractionFieldValueView, StoryExtractionFieldView,
    StoryExtractionRecordPayload, StoryExtractionRecordView, TranslationJob,
};
pub use error::{AppError, AppResult};
pub use extraction::{
    build_character_extraction_prompt, build_character_fields_prompt,
    build_character_identity_prompt, build_character_mentions_prompt,
    build_character_occurrence_confirmation_prompt, build_draft_extraction_prompt,
    DraftExtractionInput, DraftExtractionPrompt, CHARACTER_EXTRACTION_SCHEMA_VERSION,
    DRAFT_EXTRACTION_SCHEMA_VERSION,
};
pub use import::{build_import_preview, split_chapters, split_source_segments};
pub use version::{API_VERSION, APP_NAME, APP_VERSION, RELEASE_CHANNEL, STORAGE_SCHEMA_VERSION};
