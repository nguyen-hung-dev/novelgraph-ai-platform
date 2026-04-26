pub mod config;
pub mod domain;
pub mod error;
pub mod extraction;
pub mod import;
pub mod version;

pub use config::{AppConfig, AppMode};
pub use domain::{
    AnalysisJob, Chapter, ChapterPreview, CreateProjectInput, CreateTranslationJobInput,
    ImportPreview, JobEvent, Novel, NovelImportInput, NovelImportResult, Project, SourceSegment,
    TranslationJob,
};
pub use error::{AppError, AppResult};
pub use extraction::{
    build_draft_extraction_prompt, DraftExtractionInput, DraftExtractionPrompt,
    DRAFT_EXTRACTION_SCHEMA_VERSION,
};
pub use import::{build_import_preview, split_chapters, split_source_segments};
pub use version::{API_VERSION, APP_NAME, APP_VERSION, RELEASE_CHANNEL, STORAGE_SCHEMA_VERSION};
