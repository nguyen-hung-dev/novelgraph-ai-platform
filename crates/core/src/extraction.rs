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

Trích xuất node nhân vật trong đoạn. Với mỗi nhân vật, trả name và aliases=[].
Pass này chỉ được phát hiện node nhân vật, không quyết định alias/coreference.
Nếu đoạn nói một chuỗi là tên gọi, biệt danh, cách gọi trong cộng đồng, hoặc tên chính thức của cùng người, vẫn trả các surface đó như node riêng khi chưa chắc owner; pass Alias Ownership sẽ gộp sau.
name là tên chính hoặc chuỗi định danh tối thiểu tự đứng được.
Loại bỏ từ chỉ định, đại từ, sở hữu, quan hệ chủ sở hữu và ngữ cảnh xung quanh.
Không lấy ngôi xưng, đại từ hồi chỉ, cụm chỉ định, cụm sở hữu, cách gọi tạm thời trong một câu, hoặc surface chỉ có tác dụng nối ngữ pháp về người đã nhắc trước đó.
Một node nhân vật phải là surface có thể đứng độc lập như tên riêng, biệt danh, tên gọi ổn định, danh xưng định danh một người cụ thể, hoặc tên chính thức trong truyện.
Không lấy nhóm người chung nếu đoạn không định danh một người cụ thể.
Không đưa tên nhân vật khác, vai quan hệ giữa hai nhân vật, hoặc người thân của nhân vật hiện tại vào aliases.
Nếu đoạn chỉ nói quan hệ giữa nhân vật hiện tại và một nhân vật khác, bỏ qua trong identity pass; quan hệ A-B thuộc pipeline Relationship riêng.
Nếu một câu có nhiều người cùng xuất hiện, không merge alias ở pass này.
aliases phải luôn là [] vì alias/coreference thuộc pass Alias Ownership riêng.

Chỉ trả JSON array object trực tiếp:
[
  {{
    "name": "Tên chính hoặc định danh tối thiểu",
    "aliases": []
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

pub fn build_character_candidate_prompt(input: &DraftExtractionInput) -> DraftExtractionPrompt {
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

Quét nhanh ứng viên surface nhân vật trong đoạn hiện tại. Đây là checklist phủ sóng cho identity pass kế tiếp, chưa phải dữ liệu cuối cùng.

Chỉ dùng bằng chứng trong CHAPTER_TEXT. Không dùng kiến thức ngoài chương, không dùng chương tương lai.
Ưu tiên mọi người có tên riêng, biệt danh, tên chính thức, tên gọi cố định hoặc danh xưng cố định gắn với đúng một người trong đoạn.
Không trả đại từ, danh từ chung, nhóm người chung hoặc cụm mô tả không định danh một người cụ thể.
Không trả ngôi xưng, đại từ hồi chỉ, cụm chỉ định, cụm sở hữu, cách gọi tạm thời trong một câu, hoặc surface chỉ có tác dụng nối ngữ pháp về người đã nhắc trước đó.
Candidate phải là surface có thể đứng độc lập như tên riêng, biệt danh, tên gọi ổn định, danh xưng định danh một người cụ thể, hoặc tên chính thức trong truyện.
Không quyết định alias/coreference ở pass này.
Nếu một chuỗi có thể là alias của người khác, vẫn trả như candidate riêng; pass Alias Ownership sẽ xác định owner sau.
Mỗi candidate nên có evidence quote ngắn từ CHAPTER_TEXT. Offset evidence có thể bỏ trống nếu không chắc.
aliases phải luôn là [].

Chỉ trả JSON array trực tiếp:
[
  {{
    "surface_text": "Chuỗi định danh xuất hiện trong đoạn",
    "display_name": "Tên chính nếu đoạn xác định được, nếu không dùng surface_text",
    "kind": "person",
    "role_label": "mô tả rất ngắn nếu có",
    "aliases": [],
    "evidence": [
      {{
        "chapter_num": {chapter_num},
        "quote": "trích dẫn ngắn trong đoạn",
        "reason": "vì sao đây là ứng viên nhân vật"
      }}
    ],
    "confidence": 0.0
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

pub fn build_character_alias_ownership_prompt(
    input: &DraftExtractionInput,
    identities_json: &str,
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

IDENTITY_NODES_JSON:
{identities_json}

Current chapter chunk:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Vai trò của bạn: bộ xác định alias ownership và coreference theo câu cho nhân vật truyện.

Pipeline boundary:
- Identity/Candidate pass chỉ tạo node nhân vật.
- Pass này là nơi duy nhất được quyết định một surface có tính tên gọi bền vững là tên gọi khác của node nào trong đoạn hiện tại.
- Không tạo nhân vật mới.
- Không trích xuất relationship A-B, ngoại hình, năng lực, trạng thái, hành động hoặc field khác.
- Không tạo mention offsets.

Nhiệm vụ:
Đọc CHAPTER_TEXT và IDENTITY_NODES_JSON, rồi trả các cặp owner_name -> alias_text khi quote trong đoạn chứng minh alias_text là tên gọi khác của đúng owner_name.
owner_name phải khớp một name đã có trong IDENTITY_NODES_JSON.
alias_text phải là chuỗi định danh ngắn nhất tự đứng được trong CHAPTER_TEXT hoặc là name của một node khác trong IDENTITY_NODES_JSON cần nhập vào owner.
alias_text phải có thể dùng lại trên UI như một tên gọi bền vững của nhân vật ngoài câu hiện tại.
Không trả surface chỉ là ngôi xưng, đại từ hồi chỉ, cụm chỉ định, cụm sở hữu, vai ngữ pháp trong câu, hoặc cách tham chiếu tạm thời dù nó đang trỏ đúng về owner.
Nếu một surface chỉ giúp bạn hiểu ai đang được nhắc đến trong câu nhưng không phải tên gọi/biệt danh/danh xưng định danh bền vững, dùng nó để suy luận nội bộ rồi bỏ qua, không trả JSON.

Luật coreference theo câu:
- Chỉ gộp khi evidence trong cùng câu hoặc ngữ cảnh rất gần chứng minh hai surface là cùng một người.
- Nếu câu có nhiều chủ thể, không gán alias cho người đầu tiên chỉ vì họ xuất hiện trước.
- Nếu câu có cấu trúc đặt tên, giới thiệu tên thật, gán biệt danh, hoặc mô tả cách cộng đồng gọi một danh ngữ cụ thể, xác định chủ thể của alias bằng ngữ pháp/ngữ nghĩa của câu thay vì vị trí xuất hiện đầu tiên.
- Với surface trong ngoặc kép, coi đó là candidate cần xét kỹ chứ không tự động là alias. Chỉ gộp nếu câu chứng minh surface đó là cách gọi bền vững của đúng một node nhân vật.
- Khi alias đứng sau một cụm liệt kê nhiều người, chỉ gộp nếu quote cho thấy alias bổ nghĩa cho đúng một người; nếu alias có thể chỉ cả nhóm hoặc mơ hồ giữa nhiều người, bỏ qua.
- Nếu alias bổ nghĩa cho danh ngữ gần nhất trong câu và danh ngữ đó là một node trong IDENTITY_NODES_JSON, gán alias cho node đó, không gán cho node xa hơn.
- Không biến quan hệ A-B thành alias của A. Ví dụ một người thân, người quen, thuộc hạ, cấp trên của A là node riêng nếu không có evidence nói đó là cùng người với A.
- Không gộp chỉ vì giống họ, giống một token, giống danh xưng xã hội, giống chức vụ hoặc cùng xuất hiện.
- Không gộp các surface không bền vững chỉ vì tham chiếu ngữ pháp rõ ràng. Rõ chủ thể không đồng nghĩa với hợp lệ làm alias.
- Nếu quote mô tả hành động, trạng thái, cảm xúc, sở hữu, thân thể hoặc đồ vật của owner thông qua một từ hồi chỉ/ngôi xưng, không lấy từ hồi chỉ/ngôi xưng đó làm alias.

alias_type chỉ dùng:
- "nickname" cho biệt danh, tên gọi quen, tên mô tả gắn với người.
- "other_alias" cho tên gọi khác hoặc tên chính thức khác.
Không dùng alias_type khác cho output hợp lệ.
Nếu surface là đại từ, tham chiếu tạm thời, cụm mô tả, cụm sở hữu, cụm sự kiện, nhóm người chung, hoặc vai ngữ pháp trong câu thì không trả object đó.
Không trả alias_text nếu alias_text chứa nguyên tên owner cộng thêm từ khác, vì đó thường là cụm quan hệ/mệnh đề chứ không phải tên gọi độc lập.
Không trả alias_text dài theo mô tả sự kiện hoặc hành động. Alias hợp lệ phải đủ ngắn để có thể dùng lại như một tên gọi trên UI.
alias_label phải là tiếng Việt có dấu.
confidence phải >= 0.0 và <= 1.0.

Nếu không có alias ownership đủ chắc, trả [].
Chỉ trả JSON array trực tiếp:
[
  {{
    "owner_name": "name chính xác từ IDENTITY_NODES_JSON",
    "alias_text": "surface alias ngắn nhất trong CHAPTER_TEXT",
    "alias_type": "nickname",
    "alias_label": "Biệt danh",
    "confidence": 0.0,
    "evidence": [
      {{
        "chapter_num": {chapter_num},
        "quote": "một câu hoặc vế câu ngắn chứng minh owner và alias là cùng người",
        "reason": "vì sao alias_text thuộc owner_name theo ngữ pháp/ngữ nghĩa của quote"
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            identities_json = identities_json,
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
Không trả field về tên, họ tên, tên thật, tên gọi khác, biệt danh, danh xưng, quan hệ, trạng thái, ghi chú, năng lực, vật phẩm, tổ chức, địa điểm, sự kiện, mục tiêu, hành động, cảm xúc hoặc thái độ.

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
- appearance: chỉ lấy thuộc tính thị giác cụ thể và tương đối tĩnh của target: dáng người, cơ thể, khuôn mặt, da, tóc, mắt, tuổi tác nhìn thấy được, trang phục, phụ kiện, vết sẹo, dấu hiệu bề ngoài hoặc mô tả nhìn thấy được.

Luật loại trừ cho appearance:
- Không lấy hành động, hoạt động, tư thế tạm thời, thói quen, sự kiện đang diễn ra hoặc việc nhân vật đang làm, kể cả khi quote có target.
- Không lấy âm thanh, giọng nói, tiếng động, triệu chứng thoáng qua, biểu hiện cơ thể nhất thời hoặc phản ứng sinh lý.
- Không lấy cảm xúc, tinh thần, suy nghĩ, ý chí, nỗi sợ, sự tự tin, thái độ, vẻ mặt, thần sắc, ánh nhìn, biểu cảm, đánh giá xã hội hoặc nhận xét tính cách.
- Không lấy quan hệ, vai trò, chức vụ, cách gọi, danh xưng, địa vị hoặc nhóm tuổi nếu không kèm thuộc tính bề ngoài cụ thể.
- Không trả value phủ định, value nói thiếu dữ liệu, hoặc nhận xét rằng context không có mô tả ngoại hình.
- Nếu candidate trả lời câu hỏi "đang làm gì", "đang cảm thấy gì", "thái độ gì", "quan hệ gì" hoặc "xảy ra chuyện gì", candidate đó không phải appearance.
- Chỉ giữ candidate trả lời được câu hỏi "trông như thế nào", "mặc gì", "dáng vóc/khuôn mặt/da/tóc/mắt/dấu hiệu bề ngoài ra sao".
- Nếu quote vừa có target vừa có hành động nhưng không có thuộc tính bề ngoài cụ thể, trả [].

Bài kiểm tra bắt buộc trước khi trả appearance:
1. Quote có mô tả trực tiếp thuộc tính bề ngoài của target không? Nếu không, trả [].
2. Value có còn đúng nếu bỏ toàn bộ động từ, hành động, cảm xúc, thái độ, âm thanh và sự kiện khỏi quote không? Nếu không, trả [].
3. Value có mô tả đặc điểm có thể nhìn thấy trên thân thể, khuôn mặt, da, tóc, mắt, trang phục, phụ kiện hoặc dấu vết bề ngoài không? Nếu không, trả [].
4. Value có thể thay đổi ngay sau vài giây hoặc vài phút vì hành động/cảm xúc/tình huống không? Nếu có, trả [].
5. Value có phải biểu cảm gương mặt, thần thái, phản ứng, triệu chứng hoặc thái độ nhất thời không? Nếu có, trả [].

field_key phải đúng "appearance", không tự tạo field_key khác.
field_label phải đúng "Ngoại hình".
value phải là cụm danh từ hoặc cụm tính từ ngắn về bề ngoài, không phải câu hoàn chỉnh, không phải hành động, không phải trạng thái.
Mỗi value bắt buộc có evidence.quote rõ trong TARGET_CONTEXTS.
Mỗi evidence.quote bắt buộc phải chứa occurrence [[...]] của target. Nếu quote không chứa marker [[...]], bỏ value đó.
Mỗi evidence.reason phải bắt đầu bằng "Thuộc TARGET vì" và giải thích vì sao quote đó mô tả bề ngoài nhìn thấy được của chính target, không chỉ giải thích ý nghĩa của value.
Chỉ trả value khi bạn tự tin cao. Nếu thông tin chưa chắc chắn, quote không chứng minh trực tiếp target, hoặc candidate chỉ là suy diễn, hãy bỏ value thay vì trả confidence thấp.
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

pub fn build_character_field_value_verification_prompt(
    input: &DraftExtractionInput,
    character_json: &str,
    field_value_json: &str,
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

FIELD_VALUE_JSON:
{field_value_json}

TARGET_CONTEXTS:
<<<TARGET_CONTEXTS
{chapter_text}
TARGET_CONTEXTS

Vai trò của bạn: bộ kiểm định field nhân vật trước khi ghi DB. Bạn chỉ quyết định FIELD_VALUE_JSON có được phép lưu cho đúng TARGET_CHARACTER_JSON hay không.

Luật bắt buộc:
- Chỉ dùng TARGET_CONTEXTS và evidence trong FIELD_VALUE_JSON.
- Prior context chỉ giúp hiểu thể loại/bối cảnh, không được dùng làm evidence.
- TARGET_CHARACTER_JSON là owner duy nhất được phép nhận field.
- Nếu value thuộc nhân vật khác, trả accepted false và semantic_class "wrong_owner".
- Nếu quote chỉ nhắc target như người nhìn, người nghe, người nghĩ tới, người được gọi, người được chào, hoặc đối tượng của hành động nhưng value mô tả người khác, trả accepted false.
- Nếu quote không trực tiếp chứng minh value, trả accepted false.

Field hiện tại chỉ cho phép appearance.
Chỉ accepted true khi value là đặc điểm bề ngoài tương đối ổn định hoặc trực quan của chính target:
- physical_appearance: cơ thể, dáng người, khuôn mặt, da, tóc, mắt, vết sẹo, dấu hiệu bề ngoài.
- clothing: trang phục, phụ kiện, vật đang mặc/đeo.
- age_or_build: tuổi tác nhìn thấy được, vóc dáng.

Phải accepted false với các semantic_class sau:
- temporary_state: mệt mỏi, bị thương thoáng qua, trạng thái hiện tại, không động thân, phản ứng nhất thời.
- action_or_posture: khom người, đứng, đi, chạy, leo, hút thuốc, ngủ, nói, nhìn, ôm, cắn răng, đang làm gì.
- emotion_or_attitude: ngạo mạn, kính ý, khinh khỉnh, lạnh lùng, sợ hãi, vui buồn, thần sắc, vẻ mặt, ánh nhìn, thái độ.
- relationship_or_role: quan hệ, chức vụ, danh xưng, vai trò xã hội, cách gọi.
- wrong_owner: value mô tả nhân vật khác.
- no_direct_evidence: quote không chứng minh value.
- uncertain: không chắc.

accepted chỉ được true khi:
1. owner là đúng target.
2. semantic_class là physical_appearance, clothing hoặc age_or_build.
3. evidence quote chứng minh trực tiếp value.
4. confidence >= 0.85.

Nếu có bất kỳ nghi ngờ nào, accepted false.
owner_name là tên nhân vật thật sự được mô tả nếu xác định được; nếu không thì null.
confidence là độ chắc của quyết định accepted/reject, từ 0.0 đến 1.0.

Chỉ trả JSON array trực tiếp với đúng một object:
[
  {{
    "accepted": false,
    "semantic_class": "uncertain",
    "owner_name": null,
    "confidence": 0.0,
    "reason": "lý do ngắn dựa trên evidence hiện tại",
    "evidence": [
      {{
        "chapter_num": {chapter_num},
        "quote": "trích dẫn ngắn trong TARGET_CONTEXTS",
        "reason": "vì sao quyết định này đúng"
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            character_json = character_json,
            field_value_json = field_value_json,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_relationship_pair_prompt(
    input: &DraftExtractionInput,
    character_a_json: &str,
    character_b_json: &str,
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

CHARACTER_A_JSON:
{character_a_json}

CHARACTER_B_JSON:
{character_b_json}

Current full chapter:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Vai trò của bạn: nhà phân tích quan hệ nhân vật cho tiểu thuyết nhiều thể loại. Bạn phải đọc chương như một biên tập viên story bible, tự nhận diện bối cảnh thể loại và hệ thống xã hội của chương trước khi đặt nhãn quan hệ.

Nhiệm vụ: xác định quan hệ trực tiếp giữa đúng hai nhân vật canonical A và B trong chương hiện tại.
name và aliases trong CHARACTER_A_JSON/CHARACTER_B_JSON chỉ là surface để nhận diện cùng một nhân vật trong CHAPTER_TEXT.
Không tạo quan hệ giữa alias với nhân vật khác. Alias không phải node riêng.
Không xét quan hệ với nhân vật ngoài A và B.
Không dùng kiến thức ngoài chương hiện tại, không dùng future chapter, không suy diễn từ prior context.

Quy trình suy luận nội bộ bắt buộc, nhưng không được in ra ngoài JSON:
1. Tự nhận diện bối cảnh thể loại, tầng lớp xã hội và quy ước xưng hô của chương từ chính CHAPTER_TEXT.
2. Dựa trên bối cảnh đó, chọn quan hệ đúng với evidence trong chương, không mặc định vào bất kỳ nhóm quan hệ dựng sẵn nào.
3. Ưu tiên quan hệ cụ thể nhất được chứng minh bởi quote, nhưng không bịa nếu quote chỉ cho thấy tương tác thoáng qua.
4. Nếu có nhiều lớp quan hệ, chọn lớp quan hệ cốt lõi nhất giữa A và B trong chương này.

Chỉ trả quan hệ khi CHAPTER_TEXT có bằng chứng rõ ràng rằng A và B có quan hệ với nhau.
Nếu chương chỉ nhắc A và B riêng rẽ nhưng không nói quan hệ giữa họ, trả [].
Nếu chỉ có hai nhân vật cùng xuất hiện/cùng nói chuyện/cùng đi qua một cảnh mà không nêu quan hệ cụ thể, trả [].
Nếu quan hệ chỉ là phỏng đoán, cảm giác mơ hồ hoặc cần kiến thức ngoài chương, trả [].

Không được dồn mọi thứ về một kiểu nhãn quen thuộc chỉ vì có cách xưng hô giống quan hệ đó. Nếu cách xưng hô chỉ là lối gọi xã hội, biệt danh, kính ngữ hoặc cách gọi trong cộng đồng, hãy chọn nhãn đúng theo evidence hoặc trả [] nếu không rõ.
Không dùng nhãn quá chung như "có quan hệ", "liên quan", "tương tác", "xuất hiện cùng" nếu có thể chọn nhãn cụ thể hơn từ evidence.

Mỗi quan hệ phải có:
- relationship_type: ASCII snake_case do bạn tạo từ quan hệ quan sát được.
- a_to_b_label: nhãn tiếng Việt có dấu mô tả A gọi/quan hệ với B.
- b_to_a_label: nhãn tiếng Việt có dấu mô tả B gọi/quan hệ với A.
- confidence: 0..1.
- evidence: quote ngắn từ CHAPTER_TEXT chứng minh quan hệ A-B.

Nếu quan hệ hai chiều dùng cùng một nhãn, a_to_b_label và b_to_a_label có thể giống nhau.
Nếu chỉ biết quan hệ chung mà không biết chiều cụ thể, dùng cùng một nhãn tổng quát cho cả hai chiều.
Không trả hành động, cảm xúc, ngoại hình, chức vụ độc lập, sự kiện hoặc lời thoại làm quan hệ.
Không trả quan hệ nếu evidence.quote không nhắc hoặc không nối được cả A và B thông qua name/alias/ngữ cảnh rõ ràng.
relationship_type phải là semantic key của quan hệ thật, ví dụ tự tạo từ ý nghĩa evidence; không copy nguyên văn một alias hoặc danh xưng nếu đó không phải loại quan hệ.
a_to_b_label và b_to_a_label phải là nhãn đọc được trên UI, ngắn, cụ thể, có dấu, và phản ánh đúng chiều quan hệ.

Chỉ trả JSON array trực tiếp:
[
  {{
    "relationship_type": "ascii_snake_case_relation",
    "a_to_b_label": "Nhãn quan hệ từ A đến B",
    "b_to_a_label": "Nhãn quan hệ từ B đến A",
    "confidence": 0.0,
    "evidence": [
      {{
        "chapter_num": {chapter_num},
        "quote": "trích dẫn ngắn trong chương",
        "reason": "vì sao quote này chứng minh quan hệ giữa A và B"
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            character_a_json = character_a_json,
            character_b_json = character_b_json,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_relationship_candidate_prompt(
    input: &DraftExtractionInput,
    character_nodes_json: &str,
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

CHARACTER_NODES_JSON:
{character_nodes_json}

Current full chapter:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Vai trò của bạn: bộ đọc quan hệ nhân vật cho story graph. Bạn chỉ làm việc với các nhân vật đã có trong CHARACTER_NODES_JSON.

Nhiệm vụ:
- Tìm các quan hệ trực tiếp giữa hai nhân vật trong CHARACTER_NODES_JSON khi CHAPTER_TEXT có bằng chứng hiện tại.
- name và aliases chỉ là surface nhận diện canonical character; alias không phải node riêng.
- source_name và target_name phải khớp name canonical trong CHARACTER_NODES_JSON, không dùng alias làm endpoint.
- Không tạo nhân vật mới, không tạo quan hệ với nhân vật ngoài CHARACTER_NODES_JSON.
- Không dùng kiến thức ngoài chương hiện tại, không dùng chương tương lai, không dùng prior context làm evidence.

Luật evidence:
- Chỉ trả quan hệ khi quote trong CHAPTER_TEXT nối được hai nhân vật bằng tên, alias, hoặc ngữ cảnh câu gần rõ ràng.
- Nếu hai nhân vật chỉ cùng xuất hiện nhưng không có quan hệ cụ thể, bỏ qua.
- Nếu quan hệ là phỏng đoán, suy diễn xã hội, hoặc cần thông tin ngoài quote, bỏ qua.
- Nếu relation chỉ là alias/cách gọi của cùng một người, bỏ qua vì alias ownership xử lý ở pipeline khác.
- Nếu text cho thấy quan hệ thay đổi trong chương, trả quan hệ mới nhất có evidence rõ; có thể ghi chú previous trong reason nhưng không thêm field khác.
- Chỉ trả quan hệ có tính ổn định trong story graph. Hành động một lần, lời chào, lời chúc, nhìn/nghe/đi cùng, tiếp đón, báo cáo trong một cảnh, đánh nhau, quan sát, cảm xúc hoặc sự kiện thoáng qua không phải quan hệ ổn định.
- Nếu evidence chỉ chứng minh một event hoặc interaction tạm thời, bỏ qua thay vì cố đặt nhãn quan hệ.

Luật nhãn:
- Không ép vào taxonomy cố định. Tự tạo nhãn ngắn, cụ thể, tự nhiên theo evidence của truyện.
- Không dùng nhãn chung chung như "có quan hệ", "liên quan", "tương tác", "xuất hiện cùng", "không rõ".
- relationship_kind phải đúng "stable_relation" cho mọi item được trả. Nếu không chắc stable_relation, không trả item đó.
- relationship_type là ASCII snake_case semantic key do bạn tạo từ ý nghĩa quan hệ.
- source_to_target_label và target_to_source_label là nhãn tiếng Việt có dấu để hiển thị trên UI, đúng chiều quan hệ.

Mỗi item bắt buộc có quote ngắn từ CHAPTER_TEXT làm bằng chứng. Nếu không có bằng chứng, trả [].

Chỉ trả JSON array trực tiếp:
[
  {{
    "source_name": "name canonical từ CHARACTER_NODES_JSON",
    "target_name": "name canonical từ CHARACTER_NODES_JSON",
    "relationship_kind": "stable_relation",
    "relationship_type": "ascii_snake_case_relation",
    "source_to_target_label": "Nhãn quan hệ từ source đến target",
    "target_to_source_label": "Nhãn quan hệ từ target đến source",
    "confidence": 0.0,
    "evidence": [
      {{
        "chapter_num": {chapter_num},
        "quote": "trích dẫn ngắn trong chương",
        "reason": "vì sao quote này chứng minh quan hệ trực tiếp giữa hai nhân vật"
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            character_nodes_json = character_nodes_json,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_relationship_verification_prompt(
    input: &DraftExtractionInput,
    character_a_json: &str,
    character_b_json: &str,
    relationship_json: &str,
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

CHARACTER_A_JSON:
{character_a_json}

CHARACTER_B_JSON:
{character_b_json}

RELATIONSHIP_JSON:
{relationship_json}

Current full chapter:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Vai trò của bạn: bộ kiểm định quan hệ nhân vật trước khi ghi DB. Bạn chỉ quyết định RELATIONSHIP_JSON có phải là quan hệ đáng lưu trong story graph giữa CHARACTER_A_JSON và CHARACTER_B_JSON hay không.

Luật bắt buộc:
- Chỉ dùng CHAPTER_TEXT và evidence trong RELATIONSHIP_JSON.
- Prior context chỉ giúp hiểu thể loại/bối cảnh, không được dùng làm evidence.
- Không dùng kiến thức ngoài truyện, không dùng chương tương lai.
- Alias chỉ dùng để nhận diện đúng hai character; alias không phải node riêng.
- Nếu evidence chỉ chứng minh hai người cùng xuất hiện, cùng nói chuyện, cùng nhận lệnh, cùng tham gia một việc, nhìn/nghe/đi theo/ôm giúp/đưa đồ/trao tiền/báo cáo/chào hỏi trong một cảnh, trả accepted false.
- Nếu label chỉ mô tả một event, hành động, cảm xúc, tư thế, lời thoại, quyết định một lần hoặc tương tác tạm thời, trả accepted false.
- Nếu quote không chứng minh quan hệ có tính bền vững hoặc quan hệ hệ thống giữa hai nhân vật, trả accepted false.
- Nếu quan hệ chỉ là alias/cách gọi của cùng một người, trả accepted false.

accepted true chỉ khi relationship_scope thuộc một trong các loại sau:
- kinship: huyết thống, gia đình, hôn nhân, thân thuộc ổn định.
- organization_hierarchy: quan hệ cấp bậc, môn phái, tổ chức, chức vụ có hướng rõ giữa hai nhân vật.
- stable_relationship: quan hệ xã hội ổn định hoặc quan hệ truyện có ý nghĩa bền qua ngoài cảnh hiện tại.

accepted false cho các loại sau:
- temporary_interaction: tương tác trong cảnh.
- shared_event: cùng tham gia/cùng nhận một việc.
- co_presence: chỉ cùng xuất hiện/cùng đứng/cùng ở cạnh.
- scene_action: hành động một lần giữa hai người.
- alias_or_same_person: thực ra là alias/cùng một người.
- uncertain: không đủ chắc.

Nếu thể loại là tiên hiệp/kiếm hiệp/huyền huyễn, hãy chú ý quan hệ sư môn, môn phái, trưởng bối, hộ pháp, đường chủ, đệ tử, đồng môn chỉ được accepted true khi evidence chứng minh vai trò hệ thống giữa đúng hai character, không chỉ là một lần nói chuyện hoặc báo cáo.

relationship_scope phải là một trong:
kinship, organization_hierarchy, stable_relationship, temporary_interaction, shared_event, co_presence, scene_action, alias_or_same_person, uncertain.

owner_direction_ok phải true nếu source/target của label đúng chiều; false nếu label bị đảo chiều hoặc endpoint sai.
confidence là độ chắc của quyết định, từ 0.0 đến 1.0.
Nếu có bất kỳ nghi ngờ nào, accepted false.

Chỉ trả JSON array trực tiếp với đúng một object:
[
  {{
    "accepted": false,
    "relationship_scope": "uncertain",
    "owner_direction_ok": false,
    "confidence": 0.0,
    "reason": "lý do ngắn dựa trên evidence hiện tại",
    "evidence": [
      {{
        "chapter_num": {chapter_num},
        "quote": "trích dẫn ngắn trong chương",
        "reason": "vì sao quyết định này đúng"
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            character_a_json = character_a_json,
            character_b_json = character_b_json,
            relationship_json = relationship_json,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_identity_merge_confirmation_prompt(
    input: &DraftExtractionInput,
    observed_identity_json: &str,
    candidate_identity_json: &str,
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

OBSERVED_IDENTITY_JSON:
{observed_identity_json}

CANDIDATE_EXISTING_IDENTITY_JSON:
{candidate_identity_json}

Current chapter chunk:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Vai trò của bạn: bộ xác nhận canonical identity cho nhân vật truyện. Bạn chỉ quyết định OBSERVED_IDENTITY mới trích xuất trong đoạn hiện tại có phải cùng một người với CANDIDATE_EXISTING_IDENTITY đã biết hay không.

Luật bắt buộc:
- Chỉ dùng CHAPTER_TEXT hiện tại, OBSERVED_IDENTITY_JSON và CANDIDATE_EXISTING_IDENTITY_JSON.
- Không dùng kiến thức ngoài truyện, không dùng chương tương lai.
- score trong candidate chỉ là gợi ý kỹ thuật, không phải bằng chứng.
- Chỉ trả merge_existing khi tên/alias/ngữ cảnh trong CHAPTER_TEXT hoặc alias đã biết cho thấy rõ hai identity là cùng một người.
- Nếu chỉ giống họ, giống một token, giống danh xưng chung, giống chức vụ, hoặc giống cách gọi xã hội thì trả create_new.
- Nếu không đủ bằng chứng để nhập, trả create_new.
- Chỉ trả ignore khi OBSERVED_IDENTITY rõ ràng không phải nhân vật hoặc là chuỗi rác/generic không nên ghi.
- Nếu OBSERVED_IDENTITY là cụm sở hữu, cụm quan hệ, hoặc cụm nối từ một tên nhân vật đã biết cộng thêm vai trò/ngữ cảnh, trả ignore thay vì create_new.
- Không tự tạo alias mới, không sửa tên, không phân tích quan hệ.

action chỉ được là một trong ba giá trị:
- "merge_existing": nhập OBSERVED_IDENTITY vào CANDIDATE_EXISTING_IDENTITY.
- "create_new": giữ OBSERVED_IDENTITY thành nhân vật riêng.
- "ignore": bỏ OBSERVED_IDENTITY vì không phải nhân vật hợp lệ.

Chỉ trả JSON array trực tiếp với đúng một object:
[
  {{
    "action": "create_new",
    "confidence": 0.0,
    "reason": "lý do ngắn dựa trên bằng chứng hiện có"
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            observed_identity_json = observed_identity_json,
            candidate_identity_json = candidate_identity_json,
            chapter_text = input.text
        ),
    }
}

pub fn build_character_identity_creation_review_prompt(
    input: &DraftExtractionInput,
    observed_identity_json: &str,
    known_identities_json: &str,
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

OBSERVED_IDENTITY_JSON:
{observed_identity_json}

KNOWN_IDENTITIES_JSON:
{known_identities_json}

Current chapter chunk:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Vai trò của bạn: bộ kiểm tra trước khi ghi một nhân vật mới vào story graph. Bạn chỉ quyết định OBSERVED_IDENTITY trong đoạn hiện tại nên nhập vào nhân vật đã biết, tạo nhân vật mới, hay bỏ qua.

Luật bắt buộc:
- Chỉ dùng CHAPTER_TEXT hiện tại, OBSERVED_IDENTITY_JSON và KNOWN_IDENTITIES_JSON.
- Prior context chỉ giúp hiểu bối cảnh, không được dùng làm evidence.
- Không dùng kiến thức ngoài truyện, không dùng chương tương lai.
- Nếu OBSERVED_IDENTITY là cụm sở hữu, cụm quan hệ của một nhân vật đã biết, vai trò ngữ pháp, nhóm chung, đại từ, hoặc mô tả tạm thời thì trả reject.
- Nếu OBSERVED_IDENTITY là alias/tên gọi khác của một character đã có, trả merge_existing và chỉ rõ target_key/target_name từ KNOWN_IDENTITIES_JSON.
- Nếu CHAPTER_TEXT có bằng chứng rõ rằng OBSERVED_IDENTITY là nhân vật cá thể độc lập chưa có trong KNOWN_IDENTITIES_JSON, trả create_new.
- Nếu không đủ bằng chứng, ưu tiên reject hoặc create_new an toàn; không nhập sai vào character đã biết.
- Với câu có nhiều người, không gán surface cho người xuất hiện đầu tiên. Hãy xác định chủ thể theo ngữ pháp/ngữ nghĩa của câu.
- Với câu tự giới thiệu hoặc lời thoại có kiểu "có thể gọi ta là ...", owner là người đang nói hoặc người được lời thoại chỉ rõ. Nếu không xác định được speaker/owner, không merge vào character khác.

action chỉ được là một trong ba giá trị:
- "merge_existing": OBSERVED_IDENTITY là cùng một người với một identity trong KNOWN_IDENTITIES_JSON.
- "create_new": OBSERVED_IDENTITY là nhân vật cá thể mới, độc lập.
- "reject": OBSERVED_IDENTITY không nên ghi thành character.

target_key và target_name chỉ điền khi action là "merge_existing"; nếu không thì để null.
confidence phải từ 0.0 đến 1.0.
evidence.quote phải là trích dẫn ngắn từ CHAPTER_TEXT chứng minh quyết định.

Chỉ trả JSON array trực tiếp với đúng một object:
[
  {{
    "action": "create_new",
    "target_key": null,
    "target_name": null,
    "confidence": 0.0,
    "reason": "lý do ngắn dựa trên evidence hiện tại",
    "evidence": [
      {{
        "chapter_num": {chapter_num},
        "quote": "trích dẫn ngắn trong đoạn",
        "reason": "vì sao quote chứng minh quyết định"
      }}
    ]
  }}
]"#,
            chapter_num = input.chapter_num,
            title = title,
            source_language = source_language,
            prior_context = prior_context,
            observed_identity_json = observed_identity_json,
            known_identities_json = known_identities_json,
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
