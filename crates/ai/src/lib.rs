pub mod llama_cpp;
pub mod providers;
pub mod types;

pub use llama_cpp::{LlamaCppClient, LlamaCppConfig};
pub use providers::gemini::{GeminiProvider, GeminiProviderConfig};
pub use providers::llama_cpp::LlamaCppProvider;
pub use providers::openai_compatible::{OpenAiCompatibleProvider, OpenAiCompatibleProviderConfig};
pub use providers::AiProvider;
pub use types::{
    AiError, AiResult, ChatChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage,
    LlmRole, LocalLlmHealth, ModelInfo, ModelListResponse, ProviderChatRequest,
    ProviderChatResponse, ProviderKeyValidation, ProviderModelCapabilities,
    StructuredGenerationRequest, StructuredGenerationResponse, StructuredGenerationUsage,
    TokenUsage,
};
