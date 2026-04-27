use serde::{Deserialize, Serialize};

pub const DRAFT_EXTRACTION_SCHEMA_VERSION: &str = "draft.chapter_extraction.v0";
pub const CHARACTER_EXTRACTION_SCHEMA_VERSION: &str = "story_character_extraction.v1";

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

Current chapter source text or chapter chunk:
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

pub fn build_character_extraction_prompt(input: &DraftExtractionInput) -> DraftExtractionPrompt {
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
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION,
        system_prompt: character_system_prompt().to_string(),
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

Extract only records that belong to this large group:
- group_key: "character"
- group_label: "Nhân Vật"

Return only one strict JSON object. Do not use Markdown fences, comments, trailing commas, or text outside JSON. If the chapter contains too much information, keep fewer records and fewer fields, but always return complete valid JSON.

Output limits for this temporary slice:
- Maximum 8 character records for the provided text block.
- Maximum 8 fields per character.
- Maximum 2 values per field.
- Maximum 2 evidence entries per value.
- Maximum 40 mentions per character.
- Keep each quote under 120 characters.

Return JSON with this shape:
{{
  "schema_version": "{schema_version}",
  "chapter_num": {chapter_num},
  "records": [
    {{
      "group_key": "character",
      "group_label": "Nhân Vật",
      "entity_key": "ascii_snake_case_character_key",
      "display_name": "Tên hiển thị tốt nhất",
      "mentions": [
        {{
          "text": "Chuỗi xuất hiện trong chương cần highlight",
          "start_char": 0,
          "end_char": 0,
          "mention_type": "name"
        }}
      ],
      "fields": [
        {{
          "field_key": "ascii_snake_case_key_created_from_the_observed_fact",
          "field_label": "Nhãn tiếng Việt có dấu để hiển thị UI",
          "values": [
            {{
              "value": "Giá trị trích xuất từ chương",
              "confidence": 0.0,
              "evidence": [
                {{
                  "chapter_num": {chapter_num},
                  "quote": "trích dẫn ngắn trong chương",
                  "reason": "vì sao evidence này chứng minh giá trị"
                }}
              ]
            }}
          ]
        }}
      ]
    }}
  ]
}}

You may create any small field that is clearly supported by the chapter, such as names, aliases, titles, personality, role, ability, status, relationship, appearance, motive, secret, or other character facts. These are examples only; do not force a field when the chapter does not support it.

Rules:
- Every record must use group_key "character".
- The provided CHAPTER_TEXT block may be only one smaller chunk from the chapter.
- Do not return locations, items, organizations, concepts, events, or relationships as top-level records in this schema.
- Do not copy placeholder values from the JSON shape. Replace them with values supported by the provided text.
- Create field_key and field_label from the actual fact you observe.
- field_key must be stable ASCII snake_case: lowercase English letters, numbers, and underscores only.
- Do not use Vietnamese diacritics or spaces in field_key. Use examples like "other_name", "personality", "family_relation", "current_role", "ability", "appearance", "status", or "goal".
- field_label must be Vietnamese with proper diacritics.
- value should be concise and readable.
- Use evidence from the current chapter only.
- Only mentions require start_char and end_char.
- Do not put start_char or end_char inside field values or evidence.
- mentions must contain the exact visible strings to highlight for the character, such as names, aliases, nicknames, or titles.
- A mention must be the minimal contiguous surface form that identifies the character and can stand on its own as a name, alias, nickname, or title.
- Do not include surrounding determiners, demonstratives, pronouns, possessive phrases, relationship owners, particles, verbs, adjectives, clause context, or explanatory words in mention.text.
- If a longer phrase contains a character reference plus ownership, relationship, or context words, mention.text must keep only the character reference. Put the ownership, relationship, or context into fields when it is supported by the text.
- Do not use generic pronouns or pronoun phrases as highlights. Use them only inside evidence.quote or evidence.reason when needed.
- If two possible mention spans overlap for the same character, prefer the shorter span that still identifies the character.
- For every selected character surface form, return every non-overlapping occurrence inside the provided CHAPTER_TEXT block, not just one representative occurrence.
- If the same visible string appears multiple times for the same character, create one mention object for each occurrence with its own start_char and end_char.
- mention_type should be one of "name", "alias", "nickname", "title", or "other".
- mention start_char and end_char should be exact 0-based character offsets inside the provided CHAPTER_TEXT block.
- mention start_char is inclusive and mention end_char is exclusive.
- Do not guess mention offsets. If you are unsure, omit that mention.
- Do not use start_char 0 and end_char 0 as placeholder values.
- Every mention span must stay inside the provided CHAPTER_TEXT block.
- Count characters from the first character after CHAPTER_TEXT, including spaces, punctuation, Vietnamese characters, and newlines.
- The substring CHAPTER_TEXT[start_char:end_char] should match mention.text as closely as possible.
- Prefer one strong evidence quote over many repeated quotes.
- If a character fact is uncertain, keep it as a field on the character with lower confidence and explain uncertainty in evidence.reason.
- Before finishing, verify that every array and object is closed.
- If no character information appears, return records as an empty array."#,
            schema_version = CHARACTER_EXTRACTION_SCHEMA_VERSION,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_identity_prompt(input: &DraftExtractionInput) -> DraftExtractionPrompt {
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
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION,
        system_prompt: character_system_prompt().to_string(),
        user_prompt: format!(
            r#"Chapter number: {chapter_num}
Chapter title: {title}
Source language: {source_language}

Allowed prior context:
{prior_context}

Current chapter chunk:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Trích xuất nhân vật trong đoạn. Với mỗi nhân vật, trả name và aliases.
Nếu đoạn nói một chuỗi là tên gọi, biệt danh, cách gọi trong cộng đồng, hoặc tên chính thức của cùng người, hãy gộp vào cùng object.
name là tên chính hoặc chuỗi định danh tối thiểu tự đứng được.
aliases là các tên gọi khác của cùng nhân vật, mỗi alias phải có loại rõ ràng.
Loại bỏ từ chỉ định, đại từ, sở hữu, quan hệ chủ sở hữu và ngữ cảnh xung quanh.
Giữ chuỗi alias ngắn nhất tự đứng được.
Không lấy nhóm người chung nếu đoạn không định danh một người cụ thể.
Không đưa tên nhân vật khác, vai quan hệ giữa hai nhân vật, hoặc người thân của nhân vật hiện tại vào aliases.
Nếu đoạn chỉ nói quan hệ giữa nhân vật hiện tại và một nhân vật khác, bỏ qua trong identity pass; quan hệ A-B thuộc pipeline Relationship riêng.
alias_type phải là một trong các giá trị sau:
- "nickname" cho biệt danh, tên gọi quen, tên gọi mô tả gắn với nhân vật.
- "other_alias" cho tên gọi khác chưa thuộc nhóm trên.
Không dùng alias cho quan hệ, họ hàng, xưng hô hoặc vai trò giữa hai nhân vật; các thông tin đó thuộc pipeline Relationship riêng.
alias_label phải là nhãn tiếng Việt có dấu phù hợp với alias_type.
is_primary=true nếu alias là tên gọi phụ quan trọng hoặc được nhắc như cách gọi chính trong đoạn; ngược lại false.
Không đưa lại name vào aliases.

Chỉ trả JSON array object trực tiếp:
[
  {{
    "name": "Tên chính hoặc định danh tối thiểu",
    "aliases": [
      {{
        "text": "Tên gọi khác",
        "alias_type": "nickname",
        "alias_label": "Biệt danh",
        "is_primary": false
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_mentions_prompt(
    input: &DraftExtractionInput,
    character_json: &str,
) -> DraftExtractionPrompt {
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
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION,
        system_prompt: character_system_prompt().to_string(),
        user_prompt: format!(
            r#"Chapter number: {chapter_num}
Chapter title: {title}
Source language: {source_language}

Allowed prior context:
{prior_context}

Character loaded from DB:
{character_json}

Current chapter chunk:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Tìm mọi lần đoạn hiện tại nhắc tới đúng nhân vật trong Character loaded from DB.
Chỉ trả mention cho name hoặc aliases của nhân vật đó.
mention.text phải là chuỗi định danh tối thiểu xuất hiện nguyên văn trong CHAPTER_TEXT.
Không lấy đại từ, từ chỉ định, cụm sở hữu, ngữ cảnh xung quanh hoặc cả mệnh đề.
Mỗi occurrence không overlap phải có một object riêng.
start_char là offset 0-based trong CHAPTER_TEXT, inclusive.
end_char là offset 0-based trong CHAPTER_TEXT, exclusive.
Nếu không chắc offset thì bỏ occurrence đó.

Chỉ trả JSON array trực tiếp:
[
  {{
    "text": "Chuỗi định danh tối thiểu",
    "start_char": 0,
    "end_char": 0,
    "mention_type": "name"
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            character_json = character_json,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_occurrence_confirmation_prompt(
    input: &DraftExtractionInput,
    character_json: &str,
    surface: &str,
) -> DraftExtractionPrompt {
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
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION,
        system_prompt: character_system_prompt().to_string(),
        user_prompt: format!(
            r#"Chapter number: {chapter_num}
Chapter title: {title}
Source language: {source_language}

Allowed prior context:
{prior_context}

Character loaded from DB or current run:
{character_json}

Surface scanned by backend:
{surface}

Occurrence context:
<<<CONTEXT
{context_text}
CONTEXT

Backend đã exact-scan surface trong raw text và tự tính offset. Bạn không được trả offset.
Trong Occurrence context, đúng occurrence đang xét được bọc bằng [[ và ]].
Chỉ xác nhận surface trong context này có đang nhắc đến đúng nhân vật trong Character loaded from DB hay không.
Trả false nếu surface chỉ là danh từ chung, địa danh, vật phẩm, động từ/tính từ, nhóm người chung, hoặc đang nhắc đến người khác.
Trả true nếu surface là tên, biệt danh, danh xưng, vai trò gia đình/xã hội, hoặc cách gọi ngắn của đúng nhân vật trong context này.

Chỉ trả JSON array đúng một object:
[
  {{
    "is_character_mention": true,
    "confidence": 0.0,
    "reason": "Lý do ngắn"
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            character_json = character_json,
            surface = surface,
            context_text = input.text
        ),
    }
}

pub fn build_character_fields_prompt(
    input: &DraftExtractionInput,
    character_json: &str,
) -> DraftExtractionPrompt {
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
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION,
        system_prompt: character_system_prompt().to_string(),
        user_prompt: format!(
            r#"Chapter number: {chapter_num}
Chapter title: {title}
Source language: {source_language}

Allowed prior context:
{prior_context}

TARGET_CHARACTER_JSON:
{character_json}

TARGET TASK:
Bạn chỉ đang trích xuất field cho đúng một nhân vật duy nhất trong TARGET_CHARACTER_JSON.
Không trích xuất field cho bất kỳ nhân vật nào khác trong TARGET_CONTEXTS.

TARGET_CONTEXTS:
<<<TARGET_CONTEXTS
{chapter_text}
TARGET_CONTEXTS

TARGET_CONTEXTS đã được backend chọn từ đoạn hiện tại. Occurrence của target được đánh dấu bằng [[...]].

Trích xuất các field nhỏ chỉ cho nhân vật thật được định danh bởi các occurrence [[...]] trong TARGET_CONTEXTS.
Không trả mention trong bước này.
Không trích xuất nhân vật khác làm record riêng.
TARGET_CHARACTER_JSON là owner duy nhất của mọi field trả về. Nếu fact thuộc người khác, trả [] hoặc bỏ value đó.
Fields pass chỉ được trả một trong các field_key sau:
- "appearance" với field_label "Ngoại hình".
Không trả field về tên, họ tên, tên thật, tên gọi khác, biệt danh, danh xưng, quan hệ, trạng thái, ghi chú, năng lực, vật phẩm, tổ chức, địa điểm, sự kiện hoặc mục tiêu.

Luật owner bắt buộc:
- Nếu context mô tả "một người", "vị khách", "người đó", "ông ta", rồi xác định người đó là [[target]], mô tả đó thuộc target. Được lấy mô tả, nhưng không lấy nhãn quan hệ.
- Nếu [[target]] chỉ là cụm quan hệ, họ hàng, xưng hô, người được nhắc tới, người được chào, người được nghĩ tới, người được nhìn thấy, hoặc đối tượng của hành động/cảm xúc của nhân vật khác, bỏ qua candidate đó.
- Không lấy cảm xúc, trạng thái, lời nói, hành động, thói quen hoặc sự kiện thoáng qua.
- Nếu candidate là quan hệ A-B, bỏ qua vì Relationship là pipeline riêng.
- Nếu candidate là cảm xúc, suy nghĩ, hành động hoặc ý kiến của nhân vật không được đánh dấu [[...]], bỏ qua.
- Nếu candidate thuộc bất kỳ nhân vật không được đánh dấu [[...]], bỏ qua.
- Nếu chỉ có quan hệ, cách gọi, trạng thái hoặc ghi chú về target mà không có ngoại hình rõ ràng của target, trả [].
- Không cố tạo field để lấp output. Không chắc thì trả [].

Field hợp lệ duy nhất:
- appearance: cơ thể, khuôn mặt, trang phục, mô tả nhìn thấy được của target.

field_key phải đúng "appearance", không tự tạo field_key khác.
field_label phải đúng "Ngoại hình".
value phải ngắn gọn và được hỗ trợ bởi TARGET_CONTEXTS.
Mỗi value bắt buộc có evidence.quote rõ trong TARGET_CONTEXTS.
Mỗi evidence.reason phải bắt đầu bằng "Thuộc TARGET vì" và giải thích vì sao quote đó mô tả chính target, không chỉ giải thích ý nghĩa của value.
Nếu thông tin chưa chắc chắn, dùng confidence thấp hơn và giải thích trong evidence.reason.
Tối đa 1 field, tối đa 2 values.

Chỉ trả JSON array trực tiếp:
[
  {{
    "field_key": "appearance",
    "field_label": "Ngoại hình",
    "values": [
      {{
        "value": "Giá trị",
        "confidence": 0.0,
        "evidence": [
          {{
            "chapter_num": {chapter_num},
            "quote": "trích dẫn ngắn trong đoạn",
            "reason": "vì sao quote này chứng minh value"
          }}
        ]
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            character_json = character_json,
            chapter_text = input.text
        ),
    }
}

fn system_prompt() -> &'static str {
    "You extract structured fiction facts from exactly one current chapter. Use source evidence from the current chapter only. Prior context may help disambiguate names, but it must not be cited as evidence. Do not use future chapters. Return valid JSON only."
}

fn character_system_prompt() -> &'static str {
    "You extract only character information from exactly one current chapter. Use source evidence from the current chapter only. Prior context may help disambiguate names, but it must not be cited as evidence. Do not use future chapters. Return valid JSON only."
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
