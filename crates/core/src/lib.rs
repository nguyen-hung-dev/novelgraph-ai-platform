pub mod cloud_extraction;
pub mod config;
pub mod domain;
pub mod error;
pub mod extraction;
pub mod import;
pub mod prompt_registry;
pub mod version;

pub use cloud_extraction::{
    build_story_chapter_cloud_extraction_prompt, build_structured_json_repair_prompt,
    CloudChapterExtractionInput, CloudChapterExtractionPrompt, StructuredJsonRepairPrompt,
    CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION, CLOUD_GEMINI_ONE_SHOT_CALL_PROFILE,
};
pub use config::{AppConfig, AppMode};
pub use domain::{
    ActivateManagedLocalModelInput, AnalysisChapterRun, AnalysisChapterState,
    AnalysisExecutionProfile, AnalysisJob, AnalysisRunSnapshot, AnalysisRunStepInput,
    ByokProviderConfigRecord, ByokProviderConfigView, ByokProviderKeyHealth, ByokProviderPreset,
    Chapter, ChapterPreview, CheckByokProviderKeyInput, CreateProjectInput,
    CreateTranslationJobInput, DeleteProjectInput, DeleteProjectResult, ImportPreview, JobEvent,
    LocalLlmDownloadState, LocalLlmModelSelection, LocalLlmPreset, LocalLlmRuntimeSnapshot, Novel,
    NovelImportInput, NovelImportResult, NovelMetadataSuggestion, NovelMetadataSuggestionInput,
    NovelMetadataUpdateInput, Project, ProjectWorkspaceSnapshot, SaveByokProviderConfigInput,
    SaveByokProviderConfigResult, SourceSegment, StoryCharacterAliasView, StoryCharacterMention,
    StoryEvidenceSpan, StoryExtractionDocument, StoryExtractionFieldPayload,
    StoryExtractionFieldValuePayload, StoryExtractionFieldValueView, StoryExtractionFieldView,
    StoryExtractionRecordPayload, StoryExtractionRecordView, TranslationJob,
};
pub use error::{AppError, AppResult};
pub use extraction::{
    build_character_alias_ownership_prompt, build_character_candidate_prompt,
    build_character_extraction_prompt, build_character_field_value_verification_prompt,
    build_character_fields_prompt, build_character_identity_creation_review_prompt,
    build_character_identity_merge_confirmation_prompt, build_character_identity_prompt,
    build_character_mentions_prompt, build_character_occurrence_confirmation_prompt,
    build_character_relationship_candidate_prompt, build_character_relationship_pair_prompt,
    build_character_relationship_verification_prompt, build_draft_extraction_prompt,
    DraftExtractionInput, DraftExtractionPrompt, CHARACTER_EXTRACTION_SCHEMA_VERSION,
    DRAFT_EXTRACTION_SCHEMA_VERSION,
};
pub use import::{
    build_import_preview, build_novel_metadata_suggestion_prompt, detect_basic_source_language,
    split_chapters, split_source_segments,
};
pub use version::{API_VERSION, APP_NAME, APP_VERSION, RELEASE_CHANNEL, STORAGE_SCHEMA_VERSION};
