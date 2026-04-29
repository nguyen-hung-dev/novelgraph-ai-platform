#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptTemplateSpec {
    pub id: &'static str,
    pub version: &'static str,
    pub purpose: &'static str,
    pub system_template: &'static str,
    pub user_template: &'static str,
    pub response_schema_json: Option<&'static str>,
}

pub const STORY_CHAPTER_CLOUD_EXTRACTION_PROMPT_ID: &str = "story_chapter_cloud_extraction";
pub const STORY_CHAPTER_CLOUD_EXTRACTION_PROMPT_VERSION: &str = "story_chapter_cloud_extraction.v2";
pub const STRUCTURED_JSON_REPAIR_PROMPT_ID: &str = "structured_json_repair";
pub const STRUCTURED_JSON_REPAIR_PROMPT_VERSION: &str = "structured_json_repair.v1";

pub fn story_chapter_cloud_extraction_template() -> PromptTemplateSpec {
    PromptTemplateSpec {
        id: STORY_CHAPTER_CLOUD_EXTRACTION_PROMPT_ID,
        version: STORY_CHAPTER_CLOUD_EXTRACTION_PROMPT_VERSION,
        purpose: "Extract chapter story graph facts for the Gemini cloud one-shot profile.",
        system_template: include_str!("prompts/story_chapter_cloud_extraction_v2/system.md"),
        user_template: include_str!("prompts/story_chapter_cloud_extraction_v2/user.md"),
        response_schema_json: Some(include_str!(
            "prompts/story_chapter_cloud_extraction_v2/response_schema.json"
        )),
    }
}

pub fn structured_json_repair_template() -> PromptTemplateSpec {
    PromptTemplateSpec {
        id: STRUCTURED_JSON_REPAIR_PROMPT_ID,
        version: STRUCTURED_JSON_REPAIR_PROMPT_VERSION,
        purpose: "Repair provider JSON syntax without changing extracted facts.",
        system_template: include_str!("prompts/structured_json_repair_v1/system.md"),
        user_template: include_str!("prompts/structured_json_repair_v1/user.md"),
        response_schema_json: None,
    }
}

pub fn render_prompt_template(template: &str, variables: &[(&str, &str)]) -> String {
    let mut rendered = template.to_string();
    for (name, value) in variables {
        let placeholder = format!("{{{name}}}");
        rendered = rendered.replace(&placeholder, value);
    }
    rendered
}
