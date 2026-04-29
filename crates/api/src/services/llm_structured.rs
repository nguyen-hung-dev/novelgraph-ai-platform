use novelgraph_ai::{
    AiProvider, ProviderChatRequest, StructuredGenerationRequest, StructuredGenerationUsage,
};
use novelgraph_core::{build_structured_json_repair_prompt, CloudChapterExtractionPrompt};
use serde::de::DeserializeOwned;

use crate::{ApiError, AppState};

const STRUCTURED_REPAIR_INPUT_MAX_CHARS: usize = 24_000;

#[derive(Debug, Clone)]
pub(crate) struct StructuredCallTelemetry {
    pub call_status: String,
    pub api_call_count: i64,
    pub provider: String,
    pub model: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost: Option<f64>,
    pub trace_id: String,
    pub raw_response_preview: String,
}

#[derive(Debug, Clone)]
pub(crate) struct StructuredCallResult<T> {
    pub value: T,
    pub telemetry: StructuredCallTelemetry,
}

pub(crate) async fn call_gemini_structured<T>(
    state: &AppState,
    prompt: &CloudChapterExtractionPrompt,
    model: &str,
    api_key: &str,
    trace_id: &str,
    max_output_tokens: u32,
) -> Result<StructuredCallResult<T>, ApiError>
where
    T: DeserializeOwned,
{
    let schema = serde_json::from_str::<serde_json::Value>(prompt.response_schema_json)
        .map_err(|err| ApiError::internal(format!("invalid cloud response schema JSON: {err}")))?;
    let mut aggregate_usage = StructuredGenerationUsage {
        input_tokens: Some(0),
        output_tokens: Some(0),
        total_tokens: Some(0),
    };
    let mut api_call_count = 0_i64;

    let response = state
        .gemini
        .generate_structured(
            Some(api_key),
            StructuredGenerationRequest {
                model: model.to_string(),
                system_prompt: prompt.system_prompt.clone(),
                user_prompt: prompt.user_prompt.clone(),
                response_schema: schema.clone(),
                temperature: 0.0,
                max_output_tokens,
                thinking_budget_tokens: Some(0),
                trace_id: Some(trace_id.to_string()),
            },
        )
        .await?;
    api_call_count += 1;
    merge_usage(&mut aggregate_usage, &response.usage);

    let initial_raw = response.json_text;
    if let Ok(value) = parse_structured_json::<T>(&initial_raw) {
        return Ok(StructuredCallResult {
            value,
            telemetry: StructuredCallTelemetry {
                call_status: "one_shot_completed".to_string(),
                api_call_count,
                provider: response.provider,
                model: response.model,
                input_tokens: aggregate_usage.input_tokens.map(i64::from),
                output_tokens: aggregate_usage.output_tokens.map(i64::from),
                estimated_cost: state.gemini.estimate_cost(model, &aggregate_usage),
                trace_id: trace_id.to_string(),
                raw_response_preview: truncate_chars(&initial_raw, 2048),
            },
        });
    }

    let repair_response = state
        .gemini
        .generate_chat(Some(api_key), {
            let invalid_json = truncate_chars(&initial_raw, STRUCTURED_REPAIR_INPUT_MAX_CHARS);
            let repair_prompt = build_structured_json_repair_prompt(
                prompt.schema_version,
                prompt.response_schema_json,
                &invalid_json,
            );
            ProviderChatRequest {
                model: model.to_string(),
                system_prompt: repair_prompt.system_prompt,
                user_prompt: repair_prompt.user_prompt,
                temperature: 0.0,
                max_output_tokens,
                thinking_budget_tokens: Some(0),
                trace_id: Some(trace_id.to_string()),
            }
        })
        .await?;
    api_call_count += 1;
    merge_usage(&mut aggregate_usage, &repair_response.usage);
    let repaired_raw = repair_response.content;
    let value = parse_structured_json::<T>(&repaired_raw).map_err(|err| {
        ApiError::bad_request(format!(
            "cloud structured parse failed after repair: {err}; schema={}",
            prompt.schema_version
        ))
    })?;

    Ok(StructuredCallResult {
        value,
        telemetry: StructuredCallTelemetry {
            call_status: "repaired".to_string(),
            api_call_count,
            provider: repair_response.provider,
            model: repair_response.model,
            input_tokens: aggregate_usage.input_tokens.map(i64::from),
            output_tokens: aggregate_usage.output_tokens.map(i64::from),
            estimated_cost: state.gemini.estimate_cost(model, &aggregate_usage),
            trace_id: trace_id.to_string(),
            raw_response_preview: truncate_chars(&repaired_raw, 2048),
        },
    })
}

fn parse_structured_json<T>(json_text: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let candidate = extract_json_object(json_text)
        .ok_or_else(|| "provider did not return a JSON object".to_string())?;
    serde_json::from_str(candidate).map_err(|err| err.to_string())
}

fn extract_json_object(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }

    Some(&content[start..=end])
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("\n...[truncated]");
    }
    output
}

fn merge_usage(target: &mut StructuredGenerationUsage, source: &StructuredGenerationUsage) {
    target.input_tokens = sum_optional_tokens(target.input_tokens, source.input_tokens);
    target.output_tokens = sum_optional_tokens(target.output_tokens, source.output_tokens);
    target.total_tokens = sum_optional_tokens(target.total_tokens, source.total_tokens);
}

fn sum_optional_tokens(left: Option<u32>, right: Option<u32>) -> Option<u32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.saturating_add(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}
