use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    providers::AiProvider, AiError, AiResult, ProviderChatRequest, ProviderChatResponse,
    ProviderKeyValidation, ProviderModelCapabilities, StructuredGenerationRequest,
    StructuredGenerationResponse, StructuredGenerationUsage,
};

const DEFAULT_FLASH_INPUT_TOKENS: u32 = 1_000_000;
const DEFAULT_FLASH_OUTPUT_TOKENS: u32 = 65_536;
const DEFAULT_PRO_INPUT_TOKENS: u32 = 1_000_000;
const DEFAULT_PRO_OUTPUT_TOKENS: u32 = 65_536;

#[derive(Debug, Clone)]
pub struct GeminiProviderConfig {
    pub base_url: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    base_url: String,
    http: Client,
}

impl GeminiProvider {
    pub fn new(config: GeminiProviderConfig) -> AiResult<Self> {
        let base_url = normalize_base_url(&config.base_url)?;
        let timeout_secs = config.timeout_secs.max(1);
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self { base_url, http })
    }

    fn generate_content_url(&self, model: &str, api_key: &str) -> AiResult<Url> {
        let model = model.trim();
        if model.is_empty() {
            return Err(AiError::InvalidRequest("model is required".to_string()));
        }
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Err(AiError::ProviderRequest("API key is required".to_string()));
        }

        let mut url = Url::parse(&format!(
            "{}/models/{}:generateContent",
            self.base_url, model
        ))
        .map_err(|err| AiError::ProviderConfig(format!("invalid Gemini URL: {err}")))?;
        url.query_pairs_mut().append_pair("key", api_key);
        Ok(url)
    }

    async fn call_generate_content(
        &self,
        model: &str,
        api_key: &str,
        payload: &GeminiGenerateContentRequest,
    ) -> AiResult<GeminiGenerateContentResponse> {
        let url = self.generate_content_url(model, api_key)?;
        let response = self.http.post(url).json(payload).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(AiError::ProviderHttpStatus {
                status: status.as_u16(),
                message: redact_provider_error_message(&body_text),
            });
        }

        response
            .json::<GeminiGenerateContentResponse>()
            .await
            .map_err(|err| AiError::StructuredOutput(format!("invalid Gemini response: {err}")))
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    fn model_capabilities(&self, model: &str) -> ProviderModelCapabilities {
        let model = model.trim().to_ascii_lowercase();
        let (max_input_tokens, max_output_tokens) = if model.contains("2.5-pro") {
            (
                Some(DEFAULT_PRO_INPUT_TOKENS),
                Some(DEFAULT_PRO_OUTPUT_TOKENS),
            )
        } else {
            (
                Some(DEFAULT_FLASH_INPUT_TOKENS),
                Some(DEFAULT_FLASH_OUTPUT_TOKENS),
            )
        };

        ProviderModelCapabilities {
            supports_structured_output: true,
            max_input_tokens,
            max_output_tokens,
            supports_context_caching: true,
            supports_thinking_config: true,
        }
    }

    fn estimate_cost(&self, model: &str, usage: &StructuredGenerationUsage) -> Option<f64> {
        let model = model.trim().to_ascii_lowercase();
        let input_tokens = usage.input_tokens? as f64;
        let output_tokens = usage.output_tokens? as f64;

        // Approximation using public Gemini 2.5 list price buckets in USD per 1M tokens.
        let (input_per_million, output_per_million) = if model.contains("2.5-pro") {
            (1.25_f64, 10.0_f64)
        } else {
            (0.30_f64, 2.50_f64)
        };

        Some(
            (input_tokens / 1_000_000.0 * input_per_million)
                + (output_tokens / 1_000_000.0 * output_per_million),
        )
    }

    async fn validate_key(&self, model: &str, api_key: &str) -> AiResult<ProviderKeyValidation> {
        let url = self.generate_content_url(model, api_key)?;
        let body = GeminiGenerateContentRequest {
            system_instruction: None,
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: Some("ping".to_string()),
                }],
            }],
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(0.0),
                max_output_tokens: Some(8),
                response_mime_type: None,
                response_schema: None,
            }),
            thinking_config: None,
        };
        let response = self.http.post(url).json(&body).send().await?;
        let status = response.status();
        let valid = status.is_success();

        Ok(ProviderKeyValidation {
            valid,
            status_code: Some(status.as_u16()),
            message: if valid {
                "Provider accepted API key".to_string()
            } else if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
                "Provider rejected API key".to_string()
            } else {
                "Provider returned non-success status".to_string()
            },
        })
    }

    async fn generate_chat(
        &self,
        api_key: Option<&str>,
        request: ProviderChatRequest,
    ) -> AiResult<ProviderChatResponse> {
        let api_key = api_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AiError::ProviderRequest("API key is required".to_string()))?;
        let payload = GeminiGenerateContentRequest {
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: Some(request.system_prompt.clone()),
                }],
            }),
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: Some(request.user_prompt.clone()),
                }],
            }],
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(request.temperature),
                max_output_tokens: Some(request.max_output_tokens),
                response_mime_type: None,
                response_schema: None,
            }),
            thinking_config: request
                .thinking_budget_tokens
                .filter(|budget| *budget > 0)
                .map(|budget| GeminiThinkingConfig {
                    thinking_budget: Some(budget),
                }),
        };

        let response = self
            .call_generate_content(&request.model, api_key, &payload)
            .await?;
        let content = first_candidate_text(&response).ok_or_else(|| {
            AiError::StructuredOutput("Gemini returned empty content".to_string())
        })?;

        Ok(ProviderChatResponse {
            provider: "gemini".to_string(),
            model: response.model_version.unwrap_or(request.model),
            content,
            usage: map_usage(&response.usage_metadata),
            finish_reason: response
                .candidates
                .first()
                .and_then(|candidate| candidate.finish_reason.clone()),
            trace_id: request.trace_id,
        })
    }

    async fn generate_structured(
        &self,
        api_key: Option<&str>,
        request: StructuredGenerationRequest,
    ) -> AiResult<StructuredGenerationResponse> {
        let api_key = api_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AiError::ProviderRequest("API key is required".to_string()))?;
        let payload = GeminiGenerateContentRequest {
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: Some(request.system_prompt.clone()),
                }],
            }),
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: Some(request.user_prompt.clone()),
                }],
            }],
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(request.temperature),
                max_output_tokens: Some(request.max_output_tokens),
                response_mime_type: Some("application/json".to_string()),
                response_schema: Some(request.response_schema.clone()),
            }),
            thinking_config: request
                .thinking_budget_tokens
                .filter(|budget| *budget > 0)
                .map(|budget| GeminiThinkingConfig {
                    thinking_budget: Some(budget),
                }),
        };

        let response = self
            .call_generate_content(&request.model, api_key, &payload)
            .await?;
        let json_text = first_candidate_text(&response).ok_or_else(|| {
            AiError::StructuredOutput("Gemini returned empty structured content".to_string())
        })?;

        Ok(StructuredGenerationResponse {
            provider: "gemini".to_string(),
            model: response.model_version.unwrap_or(request.model),
            json_text,
            usage: map_usage(&response.usage_metadata),
            finish_reason: response
                .candidates
                .first()
                .and_then(|candidate| candidate.finish_reason.clone()),
            trace_id: request.trace_id,
        })
    }
}

fn map_usage(usage: &Option<GeminiUsageMetadata>) -> StructuredGenerationUsage {
    StructuredGenerationUsage {
        input_tokens: usage.as_ref().and_then(|usage| usage.prompt_token_count),
        output_tokens: usage
            .as_ref()
            .and_then(|usage| usage.candidates_token_count),
        total_tokens: usage.as_ref().and_then(|usage| usage.total_token_count),
    }
}

fn first_candidate_text(response: &GeminiGenerateContentResponse) -> Option<String> {
    response
        .candidates
        .first()
        .and_then(|candidate| candidate.content.as_ref())
        .and_then(|content| content.parts.iter().find_map(|part| part.text.clone()))
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn normalize_base_url(value: &str) -> AiResult<String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err(AiError::ProviderConfig(
            "Gemini base URL is required".to_string(),
        ));
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err(AiError::ProviderConfig(
            "Gemini base URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(value.to_string())
}

fn redact_provider_error_message(body: &str) -> String {
    let mut redacted = body.to_string();
    redacted = redact_google_api_keys(&redacted);
    redacted = redact_bearer_tokens(&redacted);
    let trimmed = redacted.trim();
    if trimmed.is_empty() {
        "provider request failed".to_string()
    } else {
        trimmed.to_string()
    }
}

fn redact_google_api_keys(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let chars = value.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if index + 4 <= chars.len()
            && chars[index] == 'A'
            && chars[index + 1] == 'I'
            && chars[index + 2] == 'z'
            && chars[index + 3] == 'a'
        {
            let mut end = index + 4;
            while end < chars.len() && chars[end].is_ascii_alphanumeric() {
                end += 1;
            }
            if end.saturating_sub(index) >= 16 {
                output.push_str("[redacted_api_key]");
                index = end;
                continue;
            }
        }
        output.push(chars[index]);
        index += 1;
    }
    output
}

fn redact_bearer_tokens(value: &str) -> String {
    value
        .split_whitespace()
        .scan(false, |seen_bearer, token| {
            if *seen_bearer {
                *seen_bearer = false;
                Some("[redacted_token]".to_string())
            } else {
                *seen_bearer = token.eq_ignore_ascii_case("bearer");
                Some(token.to_string())
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerateContentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<GeminiThinkingConfig>,
}

#[derive(Debug, Clone, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerateContentResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
    #[serde(default)]
    model_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiContent>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: Option<u32>,
    #[serde(default)]
    candidates_token_count: Option<u32>,
    #[serde(default)]
    total_token_count: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::{redact_bearer_tokens, redact_google_api_keys, redact_provider_error_message};

    #[test]
    fn redacts_google_api_keys() {
        let text = "invalid key AIzaSyA1234567890ABCDEFGHIJKLMNOP";
        let redacted = redact_google_api_keys(text);
        assert!(!redacted.contains("AIza"));
        assert!(redacted.contains("[redacted_api_key]"));
    }

    #[test]
    fn redacts_bearer_tokens() {
        let text = "Authorization: Bearer super-secret-token";
        let redacted = redact_bearer_tokens(text);
        assert!(redacted.contains("Bearer [redacted_token]"));
        assert!(!redacted.contains("super-secret-token"));
    }

    #[test]
    fn redaction_keeps_safe_message() {
        let text = "{\"error\":\"Bearer token invalid: AIzaSyA1234567890ABCDEFGHIJK\"}";
        let redacted = redact_provider_error_message(text);
        assert!(redacted.contains("[redacted"));
        assert!(!redacted.contains("AIza"));
    }
}
