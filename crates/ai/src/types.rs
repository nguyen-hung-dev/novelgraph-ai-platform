use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type AiResult<T> = Result<T, AiError>;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("invalid local LLM configuration: {0}")]
    InvalidConfig(String),

    #[error("invalid local LLM request: {0}")]
    InvalidRequest(String),

    #[error("local LLM request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("local LLM returned HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("provider configuration is invalid: {0}")]
    ProviderConfig(String),

    #[error("provider request failed: {0}")]
    ProviderRequest(String),

    #[error("provider returned HTTP {status}: {message}")]
    ProviderHttpStatus { status: u16, message: String },

    #[error("provider returned invalid structured output: {0}")]
    StructuredOutput(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalLlmHealth {
    pub provider: &'static str,
    pub base_url: String,
    pub reachable: bool,
    pub status_code: Option<u16>,
    pub status_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: LlmRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_template_kwargs: Option<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatChoice {
    pub index: Option<u32>,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelListResponse {
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub object: Option<String>,
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderModelCapabilities {
    pub supports_structured_output: bool,
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub supports_context_caching: bool,
    pub supports_thinking_config: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderKeyValidation {
    pub valid: bool,
    pub status_code: Option<u16>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredGenerationUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredGenerationRequest {
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub response_schema: serde_json::Value,
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub thinking_budget_tokens: Option<u32>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredGenerationResponse {
    pub provider: String,
    pub model: String,
    pub json_text: String,
    pub usage: StructuredGenerationUsage,
    pub finish_reason: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderChatRequest {
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub thinking_budget_tokens: Option<u32>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderChatResponse {
    pub provider: String,
    pub model: String,
    pub content: String,
    pub usage: StructuredGenerationUsage,
    pub finish_reason: Option<String>,
    pub trace_id: Option<String>,
}
