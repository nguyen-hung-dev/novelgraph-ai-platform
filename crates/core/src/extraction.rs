use serde::{Deserialize, Serialize};

pub const DRAFT_EXTRACTION_SCHEMA_VERSION: &str = "draft.chapter_extraction.v0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DraftExtractionInput {
    pub chapter_num: i64,
    pub title: Option<String>,
    pub source_language: Option<String>,
    pub text: String,
    pub prior_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DraftExtractionPrompt {
    pub schema_version: &'static str,
    pub system_prompt: String,
    pub user_prompt: String,
}

pub fn build_draft_extraction_prompt(input: &DraftExtractionInput) -> DraftExtractionPrompt {
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Untitled chapter");
    let source_language = input
        .source_language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let prior_context = input
        .prior_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("No prior context is provided.");

    DraftExtractionPrompt {
        schema_version: DRAFT_EXTRACTION_SCHEMA_VERSION,
        system_prompt: system_prompt().to_string(),
        user_prompt: format!(
            r#"Schema version: {schema_version}
Chapter number: {chapter_num}
Chapter title: {title}
Source language: {source_language}

Allowed prior context:
{prior_context}

Current chapter source text:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Return only valid JSON with this shape:
{{
  "schema_version": "{schema_version}",
  "characters": [],
  "locations": [],
  "organizations": [],
  "items": [],
  "concepts": [],
  "relationships": [],
  "events": [],
  "spatial_relations": [],
  "review_items": []
}}

For every factual item, include:
- "confidence": number between 0 and 1
- "evidence": array of objects with "chapter_num", "start_char", "end_char", "quote", and "reason"

If a fact is inferred or uncertain, put it into "review_items" and explain why."#,
            schema_version = DRAFT_EXTRACTION_SCHEMA_VERSION,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            chapter_text = input.text
        ),
    }
}

fn system_prompt() -> &'static str {
    "You extract structured fiction facts from exactly one current chapter. Use source evidence from the current chapter only. Prior context may help disambiguate names, but it must not be cited as evidence. Do not use future chapters. Return valid JSON only."
}

#[cfg(test)]
mod tests {
    use super::{
        build_draft_extraction_prompt, DraftExtractionInput, DRAFT_EXTRACTION_SCHEMA_VERSION,
    };

    #[test]
    fn builds_prompt_with_schema_and_current_chapter_boundary() {
        let prompt = build_draft_extraction_prompt(&DraftExtractionInput {
            chapter_num: 3,
            title: Some("Gặp lại".to_string()),
            source_language: Some("vi".to_string()),
            text: "Nhân vật bước vào thung lũng.".to_string(),
            prior_context: None,
        });

        assert_eq!(prompt.schema_version, DRAFT_EXTRACTION_SCHEMA_VERSION);
        assert!(prompt.user_prompt.contains("Chapter number: 3"));
        assert!(prompt.user_prompt.contains("<<<CHAPTER_TEXT"));
        assert!(prompt.user_prompt.contains("\"review_items\""));
    }
}
