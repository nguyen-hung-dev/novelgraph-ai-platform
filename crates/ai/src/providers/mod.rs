use async_trait::async_trait;

use crate::{
    AiResult, ProviderChatRequest, ProviderChatResponse, ProviderKeyValidation,
    ProviderModelCapabilities, StructuredGenerationRequest, StructuredGenerationResponse,
    StructuredGenerationUsage,
};

pub mod gemini;
pub mod llama_cpp;
pub mod openai_compatible;

#[async_trait]
pub trait AiProvider {
    fn provider_name(&self) -> &'static str;

    fn model_capabilities(&self, model: &str) -> ProviderModelCapabilities;

    fn estimate_cost(&self, model: &str, usage: &StructuredGenerationUsage) -> Option<f64>;

    async fn validate_key(&self, model: &str, api_key: &str) -> AiResult<ProviderKeyValidation>;

    async fn generate_chat(
        &self,
        api_key: Option<&str>,
        request: ProviderChatRequest,
    ) -> AiResult<ProviderChatResponse>;

    async fn generate_structured(
        &self,
        api_key: Option<&str>,
        request: StructuredGenerationRequest,
    ) -> AiResult<StructuredGenerationResponse>;
}
