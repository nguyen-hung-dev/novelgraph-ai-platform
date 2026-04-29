use serde::Serialize;

use crate::prompt_registry::{
    render_prompt_template, story_chapter_cloud_extraction_template,
    structured_json_repair_template, STORY_CHAPTER_CLOUD_EXTRACTION_PROMPT_VERSION,
};

pub const CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION: &str =
    STORY_CHAPTER_CLOUD_EXTRACTION_PROMPT_VERSION;
pub const CLOUD_GEMINI_ONE_SHOT_CALL_PROFILE: &str = "cloud_gemini_one_shot";

const DEFAULT_CHAPTER_TITLE: &str = "Untitled chapter";
const DEFAULT_SOURCE_LANGUAGE: &str = "unknown";
const DEFAULT_NOVEL_CONTEXT: &str = "No novel context provided.";
const EMPTY_JSON_ARRAY: &str = "[]";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CloudChapterExtractionPrompt {
    pub schema_version: &'static str,
    pub template_id: &'static str,
    pub template_version: &'static str,
    pub call_profile: &'static str,
    pub response_schema_json: &'static str,
    pub system_prompt: String,
    pub user_prompt: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CloudChapterExtractionInput {
    pub chapter_num: i64,
    pub title: Option<String>,
    pub source_language: Option<String>,
    pub chapter_text: String,
    pub novel_context: Option<String>,
    pub known_alias_surfaces_json: Option<String>,
    pub known_relationships_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructuredJsonRepairPrompt {
    pub template_id: &'static str,
    pub template_version: &'static str,
    pub system_prompt: String,
    pub user_prompt: String,
}

pub fn build_story_chapter_cloud_extraction_prompt(
    input: &CloudChapterExtractionInput,
) -> CloudChapterExtractionPrompt {
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_CHAPTER_TITLE);
    let source_language = input
        .source_language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SOURCE_LANGUAGE);
    let novel_context = input
        .novel_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_NOVEL_CONTEXT);
    let known_alias_surfaces_json = input
        .known_alias_surfaces_json
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(EMPTY_JSON_ARRAY);
    let known_relationships_json = input
        .known_relationships_json
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(EMPTY_JSON_ARRAY);
    let chapter_num = input.chapter_num.to_string();
    let template = story_chapter_cloud_extraction_template();
    let response_schema_json = template
        .response_schema_json
        .expect("cloud chapter extraction template must define response schema");

    CloudChapterExtractionPrompt {
        schema_version: CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION,
        template_id: template.id,
        template_version: template.version,
        call_profile: CLOUD_GEMINI_ONE_SHOT_CALL_PROFILE,
        response_schema_json,
        system_prompt: template.system_template.trim().to_string(),
        user_prompt: render_prompt_template(
            template.user_template.trim(),
            &[
                ("schema_version", CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION),
                ("prompt_id", template.id),
                ("prompt_version", template.version),
                ("call_profile", CLOUD_GEMINI_ONE_SHOT_CALL_PROFILE),
                ("chapter_num", &chapter_num),
                ("title", title),
                ("source_language", source_language),
                ("novel_context", novel_context),
                ("known_alias_surfaces_json", known_alias_surfaces_json),
                ("known_relationships_json", known_relationships_json),
                ("chapter_text", &input.chapter_text),
            ],
        ),
    }
}

pub fn build_structured_json_repair_prompt(
    schema_version: &str,
    response_schema_json: &str,
    invalid_json: &str,
) -> StructuredJsonRepairPrompt {
    let template = structured_json_repair_template();
    StructuredJsonRepairPrompt {
        template_id: template.id,
        template_version: template.version,
        system_prompt: template.system_template.trim().to_string(),
        user_prompt: render_prompt_template(
            template.user_template.trim(),
            &[
                ("schema_version", schema_version),
                ("response_schema_json", response_schema_json),
                ("invalid_json", invalid_json),
            ],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_versioned_cloud_prompt_from_registry_templates() {
        let prompt = build_story_chapter_cloud_extraction_prompt(&CloudChapterExtractionInput {
            chapter_num: 1,
            title: Some("Chương 1".to_string()),
            source_language: Some("vi".to_string()),
            chapter_text: "Hàn Lập gặp tam thúc.".to_string(),
            novel_context: Some("Novel: Phàm Nhân Tu Tiên".to_string()),
            known_alias_surfaces_json: Some("[]".to_string()),
            known_relationships_json: Some("[]".to_string()),
        });

        assert_eq!(prompt.schema_version, "story_chapter_cloud_extraction.v2");
        assert_eq!(prompt.call_profile, "cloud_gemini_one_shot");
        assert!(prompt.user_prompt.contains("Chapter number: 1"));
        assert!(prompt.user_prompt.contains("Known character surfaces"));
        assert!(prompt.user_prompt.contains("Known stable relationships"));
        assert!(prompt
            .user_prompt
            .contains("Field value is a compact display label"));
        assert!(prompt.response_schema_json.contains("\"relationships\""));
    }

    #[test]
    fn builds_structured_json_repair_prompt_from_registry_template() {
        let prompt =
            build_structured_json_repair_prompt("schema.v1", "{\"type\":\"OBJECT\"}", "{bad");

        assert_eq!(prompt.template_version, "structured_json_repair.v1");
        assert!(prompt
            .system_prompt
            .contains("Repair invalid JSON object output"));
        assert!(prompt.user_prompt.contains("schema.v1"));
        assert!(prompt.user_prompt.contains("{bad"));
    }
}
