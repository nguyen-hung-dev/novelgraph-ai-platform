use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::json;

use crate::{
    providers::AiProvider, AiError, AiResult, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, LlmRole, ProviderChatRequest, ProviderChatResponse, ProviderKeyValidation,
    ProviderModelCapabilities, StructuredGenerationRequest, StructuredGenerationResponse,
    StructuredGenerationUsage,
};

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProviderConfig {
    pub provider_name: &'static str,
    pub base_url: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    provider_name: &'static str,
    base_url: String,
    http: Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: OpenAiCompatibleProviderConfig) -> AiResult<Self> {
        let base_url = normalize_base_url(&config.base_url)?;
        let timeout_secs = config.timeout_secs.max(1);
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            provider_name: config.provider_name,
            base_url,
            http,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn send_chat_completion(
        &self,
        api_key: &str,
        request: ChatCompletionRequest,
    ) -> AiResult<ChatCompletionResponse> {
        let response = self
            .http
            .post(self.url("/chat/completions"))
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await?;

        ensure_success(response.status()).await?;
        response
            .json::<ChatCompletionResponse>()
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl AiProvider for OpenAiCompatibleProvider {
    fn provider_name(&self) -> &'static str {
        self.provider_name
    }

    fn model_capabilities(&self, _model: &str) -> ProviderModelCapabilities {
        ProviderModelCapabilities {
            supports_structured_output: false,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_context_caching: false,
            supports_thinking_config: true,
        }
    }

    fn estimate_cost(&self, _model: &str, _usage: &StructuredGenerationUsage) -> Option<f64> {
        None
    }

    async fn validate_key(&self, _model: &str, api_key: &str) -> AiResult<ProviderKeyValidation> {
        let response = self
            .http
            .get(self.url("/models"))
            .bearer_auth(api_key)
            .send()
            .await?;
        let status = response.status();
        let valid = status.is_success();

        Ok(ProviderKeyValidation {
            valid,
            status_code: Some(status.as_u16()),
            message: if valid {
                "Provider accepted API key".to_string()
            } else {
                "Provider rejected API key".to_string()
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
        let response = self
            .send_chat_completion(
                api_key,
                ChatCompletionRequest {
                    model: Some(request.model.clone()),
                    messages: vec![
                        ChatMessage {
                            role: LlmRole::System,
                            content: request.system_prompt,
                        },
                        ChatMessage {
                            role: LlmRole::User,
                            content: request.user_prompt,
                        },
                    ],
                    temperature: Some(request.temperature),
                    max_tokens: Some(request.max_output_tokens),
                    chat_template_kwargs: request
                        .thinking_budget_tokens
                        .map(|tokens| json!({ "thinking_budget_tokens": tokens }))
                        .or_else(|| Some(json!({ "enable_thinking": false }))),
                    stream: false,
                },
            )
            .await?;
        let content = response
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AiError::StructuredOutput("provider returned empty message".to_string())
            })?;

        Ok(ProviderChatResponse {
            provider: self.provider_name.to_string(),
            model: response
                .model
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(&request.model)
                .to_string(),
            content,
            usage: map_usage(response.usage),
            finish_reason: response
                .choices
                .first()
                .and_then(|choice| choice.finish_reason.clone()),
            trace_id: request.trace_id,
        })
    }

    async fn generate_structured(
        &self,
        api_key: Option<&str>,
        request: StructuredGenerationRequest,
    ) -> AiResult<StructuredGenerationResponse> {
        let chat_response = self
            .generate_chat(
                api_key,
                ProviderChatRequest {
                    model: request.model.clone(),
                    system_prompt: request.system_prompt,
                    user_prompt: request.user_prompt,
                    temperature: request.temperature,
                    max_output_tokens: request.max_output_tokens,
                    thinking_budget_tokens: request.thinking_budget_tokens,
                    trace_id: request.trace_id.clone(),
                },
            )
            .await?;

        Ok(StructuredGenerationResponse {
            provider: chat_response.provider,
            model: chat_response.model,
            json_text: chat_response.content,
            usage: chat_response.usage,
            finish_reason: chat_response.finish_reason,
            trace_id: request.trace_id,
        })
    }
}

fn map_usage(usage: Option<crate::TokenUsage>) -> StructuredGenerationUsage {
    StructuredGenerationUsage {
        input_tokens: usage.as_ref().and_then(|usage| usage.prompt_tokens),
        output_tokens: usage.as_ref().and_then(|usage| usage.completion_tokens),
        total_tokens: usage.as_ref().and_then(|usage| usage.total_tokens),
    }
}

async fn ensure_success(status: StatusCode) -> AiResult<()> {
    if status.is_success() {
        return Ok(());
    }

    Err(AiError::ProviderHttpStatus {
        status: status.as_u16(),
        message: status
            .canonical_reason()
            .unwrap_or("provider request failed")
            .to_string(),
    })
}

fn normalize_base_url(value: &str) -> AiResult<String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err(AiError::ProviderConfig(
            "provider base URL is required".to_string(),
        ));
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err(AiError::ProviderConfig(
            "provider base URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(value.to_string())
}
