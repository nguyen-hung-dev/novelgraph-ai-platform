use async_trait::async_trait;
use serde_json::json;

use crate::{
    providers::AiProvider, AiError, AiResult, ChatCompletionRequest, ChatMessage, LlamaCppClient,
    LlmRole, ProviderChatRequest, ProviderChatResponse, ProviderKeyValidation,
    ProviderModelCapabilities, StructuredGenerationRequest, StructuredGenerationResponse,
    StructuredGenerationUsage, TokenUsage,
};

#[derive(Debug, Clone)]
pub struct LlamaCppProvider {
    client: LlamaCppClient,
}

impl LlamaCppProvider {
    pub fn new(client: LlamaCppClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AiProvider for LlamaCppProvider {
    fn provider_name(&self) -> &'static str {
        "llama.cpp"
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
        Some(0.0)
    }

    async fn validate_key(&self, _model: &str, _api_key: &str) -> AiResult<ProviderKeyValidation> {
        Ok(ProviderKeyValidation {
            valid: true,
            status_code: Some(200),
            message: "local provider does not require API key".to_string(),
        })
    }

    async fn generate_chat(
        &self,
        _api_key: Option<&str>,
        request: ProviderChatRequest,
    ) -> AiResult<ProviderChatResponse> {
        let response = self
            .client
            .chat_completion(ChatCompletionRequest {
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
                chat_template_kwargs: Some(
                    request
                        .thinking_budget_tokens
                        .map(|tokens| json!({ "thinking_budget_tokens": tokens }))
                        .unwrap_or_else(|| json!({ "enable_thinking": false })),
                ),
                stream: false,
            })
            .await?;

        let content = response
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AiError::StructuredOutput("local model returned empty message".to_string())
            })?;

        Ok(ProviderChatResponse {
            provider: "llama.cpp".to_string(),
            model: response.model.unwrap_or(request.model),
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
        _api_key: Option<&str>,
        request: StructuredGenerationRequest,
    ) -> AiResult<StructuredGenerationResponse> {
        let chat = self
            .generate_chat(
                None,
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
            provider: chat.provider,
            model: chat.model,
            json_text: chat.content,
            usage: chat.usage,
            finish_reason: chat.finish_reason,
            trace_id: request.trace_id,
        })
    }
}

fn map_usage(usage: Option<TokenUsage>) -> StructuredGenerationUsage {
    StructuredGenerationUsage {
        input_tokens: usage.as_ref().and_then(|usage| usage.prompt_tokens),
        output_tokens: usage.as_ref().and_then(|usage| usage.completion_tokens),
        total_tokens: usage.as_ref().and_then(|usage| usage.total_tokens),
    }
}
