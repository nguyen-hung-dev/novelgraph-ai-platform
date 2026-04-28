use novelgraph_ai::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, LlamaCppClient, LlmRole,
};
use novelgraph_core::DraftExtractionPrompt;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{ApiError, AppState};

const LOCAL_JSON_REPAIR_INPUT_MAX_CHARS: usize = 16_000;

pub(crate) async fn call_local_json_array<T>(
    state: &AppState,
    prompt: &DraftExtractionPrompt,
    max_tokens: u32,
) -> Result<(Vec<T>, ChatCompletionResponse), ApiError>
where
    T: DeserializeOwned,
{
    call_client_json_array(&state.local_llm, prompt, max_tokens).await
}

async fn call_client_json_array<T>(
    local_llm: &LlamaCppClient,
    prompt: &DraftExtractionPrompt,
    max_tokens: u32,
) -> Result<(Vec<T>, ChatCompletionResponse), ApiError>
where
    T: DeserializeOwned,
{
    let response = local_llm
        .chat_completion(ChatCompletionRequest {
            model: None,
            messages: vec![
                ChatMessage {
                    role: LlmRole::System,
                    content: prompt.system_prompt.clone(),
                },
                ChatMessage {
                    role: LlmRole::User,
                    content: prompt.user_prompt.clone(),
                },
            ],
            temperature: Some(0.0),
            max_tokens: Some(max_tokens),
            chat_template_kwargs: Some(json!({ "enable_thinking": false })),
            stream: false,
        })
        .await?;
    match parse_json_array_response::<T>(&response) {
        Ok(items) => Ok((items, response)),
        Err(parse_error) => {
            let repair_response =
                repair_local_json_array_response(local_llm, prompt, &response, max_tokens).await?;
            match parse_json_array_response::<T>(&repair_response) {
                Ok(items) => Ok((items, repair_response)),
                Err(retry_error) => Err(ApiError::bad_request(format!(
                    "{}; repair retry failed: {}",
                    parse_error.message, retry_error.message
                ))),
            }
        }
    }
}

fn parse_json_array_response<T>(response: &ChatCompletionResponse) -> Result<Vec<T>, ApiError>
where
    T: DeserializeOwned,
{
    let content = response
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| ApiError::bad_request("local LLM returned empty JSON array response"))?;
    let json_text = extract_json_array(content)
        .ok_or_else(|| ApiError::bad_request("local LLM did not return a JSON array"))?;

    parse_json_array_text(json_text)
}

fn parse_json_array_text<T>(json_text: &str) -> Result<Vec<T>, ApiError>
where
    T: DeserializeOwned,
{
    match serde_json::from_str::<Vec<T>>(json_text) {
        Ok(items) => Ok(items),
        Err(initial_error) => {
            let sanitized = escape_control_chars_inside_json_strings(json_text);
            if sanitized == json_text {
                return Err(ApiError::bad_request(format!(
                    "local LLM JSON array parse failed: {initial_error}"
                )));
            }

            serde_json::from_str::<Vec<T>>(&sanitized).map_err(|retry_error| {
                ApiError::bad_request(format!(
                    "local LLM JSON array parse failed: {initial_error}; sanitized parse failed: {retry_error}"
                ))
            })
        }
    }
}

fn escape_control_chars_inside_json_strings(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_string = false;
    let mut escaped = false;

    for ch in value.chars() {
        if !in_string {
            output.push(ch);
            if ch == '"' {
                in_string = true;
            }
            continue;
        }

        if escaped {
            output.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                output.push(ch);
                escaped = true;
            }
            '"' => {
                output.push(ch);
                in_string = false;
            }
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => {
                output.push_str(&format!("\\u{:04x}", ch as u32));
            }
            _ => output.push(ch),
        }
    }

    output
}

async fn repair_local_json_array_response(
    local_llm: &LlamaCppClient,
    prompt: &DraftExtractionPrompt,
    invalid_response: &ChatCompletionResponse,
    max_tokens: u32,
) -> Result<ChatCompletionResponse, ApiError> {
    let invalid_content = response_message_content(invalid_response);
    let repair_input = truncate_chars(&invalid_content, LOCAL_JSON_REPAIR_INPUT_MAX_CHARS);

    local_llm
        .chat_completion(ChatCompletionRequest {
            model: None,
            messages: vec![
                ChatMessage {
                    role: LlmRole::System,
                    content: "You repair invalid JSON array output. Return valid JSON only. Do not add new facts. Do not explain.".to_string(),
                },
                ChatMessage {
                    role: LlmRole::User,
                    content: format!(
                        "Schema version: {}\n\nThe previous response was intended to be a JSON array but failed parsing. Repair only syntax problems such as raw control characters inside strings, missing escaping, trailing text, or malformed commas. Preserve the same array items and fields as much as possible. If an item cannot be repaired safely, remove that item. Return a JSON array directly.\n\nInvalid response:\n<<<INVALID_JSON\n{}\nINVALID_JSON",
                        prompt.schema_version,
                        repair_input
                    ),
                },
            ],
            temperature: Some(0.0),
            max_tokens: Some(max_tokens),
            chat_template_kwargs: Some(json!({ "enable_thinking": false })),
            stream: false,
        })
        .await
        .map_err(ApiError::from)
}

fn response_message_content(response: &ChatCompletionResponse) -> String {
    response
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .unwrap_or_default()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("\n...[truncated]");
    }
    output
}

fn extract_json_array(content: &str) -> Option<&str> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;

    if end <= start {
        return None;
    }

    Some(&content[start..=end])
}
