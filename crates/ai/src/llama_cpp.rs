use std::time::Duration;

use reqwest::{Client, StatusCode};

use crate::{
    AiError, AiResult, ChatCompletionRequest, ChatCompletionResponse, LocalLlmHealth,
    ModelListResponse,
};

#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    pub base_url: String,
    pub default_model: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct LlamaCppClient {
    config: LlamaCppConfig,
    http: Client,
}

impl LlamaCppClient {
    pub fn new(config: LlamaCppConfig) -> AiResult<Self> {
        let base_url = normalize_base_url(&config.base_url)?;
        let default_model = config.default_model.trim().to_string();
        if default_model.is_empty() {
            return Err(AiError::InvalidConfig(
                "LLAMA_CPP_DEFAULT_MODEL is required".to_string(),
            ));
        }

        let timeout_secs = config.timeout_secs.max(1);
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            config: LlamaCppConfig {
                base_url,
                default_model,
                timeout_secs,
            },
            http,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    pub fn default_model(&self) -> &str {
        &self.config.default_model
    }

    pub async fn health(&self) -> AiResult<LocalLlmHealth> {
        let url = self.url("/health");
        let response = match self.http.get(url).send().await {
            Ok(response) => response,
            Err(_) => {
                return Ok(LocalLlmHealth {
                    provider: "llama.cpp",
                    base_url: self.config.base_url.clone(),
                    reachable: false,
                    status_code: None,
                    status_text: Some("request failed".to_string()),
                });
            }
        };
        let status = response.status();

        Ok(LocalLlmHealth {
            provider: "llama.cpp",
            base_url: self.config.base_url.clone(),
            reachable: status.is_success(),
            status_code: Some(status.as_u16()),
            status_text: status.canonical_reason().map(ToOwned::to_owned),
        })
    }

    pub async fn list_models(&self) -> AiResult<ModelListResponse> {
        let response = self.http.get(self.url("/v1/models")).send().await?;
        ensure_success(response.status()).await?;
        Ok(response.json::<ModelListResponse>().await?)
    }

    pub async fn chat_completion(
        &self,
        mut request: ChatCompletionRequest,
    ) -> AiResult<ChatCompletionResponse> {
        if request.messages.is_empty() {
            return Err(AiError::InvalidRequest(
                "messages must contain at least one item".to_string(),
            ));
        }

        let model = request
            .model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| self.config.default_model.clone());
        request.model = Some(model);
        request.stream = false;

        let response = self
            .http
            .post(self.url("/v1/chat/completions"))
            .json(&request)
            .send()
            .await?;
        ensure_success(response.status()).await?;

        Ok(response.json::<ChatCompletionResponse>().await?)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }
}

async fn ensure_success(status: StatusCode) -> AiResult<()> {
    if status.is_success() {
        return Ok(());
    }

    Err(AiError::HttpStatus {
        status: status.as_u16(),
        message: status
            .canonical_reason()
            .unwrap_or("local LLM request failed")
            .to_string(),
    })
}

fn normalize_base_url(value: &str) -> AiResult<String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err(AiError::InvalidConfig(
            "LLAMA_CPP_BASE_URL is required".to_string(),
        ));
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err(AiError::InvalidConfig(
            "LLAMA_CPP_BASE_URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use crate::{ChatCompletionRequest, ChatMessage, LlmRole, ModelListResponse};

    use super::{LlamaCppClient, LlamaCppConfig};

    #[test]
    fn normalizes_base_url_and_default_model() {
        let client = LlamaCppClient::new(LlamaCppConfig {
            base_url: "http://127.0.0.1:8080/".to_string(),
            default_model: "qwen3".to_string(),
            timeout_secs: 0,
        })
        .unwrap();

        assert_eq!(client.base_url(), "http://127.0.0.1:8080");
        assert_eq!(client.default_model(), "qwen3");
    }

    #[test]
    fn rejects_invalid_base_url() {
        let error = LlamaCppClient::new(LlamaCppConfig {
            base_url: "127.0.0.1:8080".to_string(),
            default_model: "qwen3".to_string(),
            timeout_secs: 30,
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("must start with"));
    }

    #[test]
    fn serializes_chat_request_shape() {
        let request = ChatCompletionRequest {
            model: Some("qwen3".to_string()),
            messages: vec![ChatMessage {
                role: LlmRole::User,
                content: "Xin chào".to_string(),
            }],
            temperature: Some(0.2),
            max_tokens: Some(128),
            stream: false,
        };
        let value = serde_json::to_value(request).unwrap();

        assert_eq!(value["model"], "qwen3");
        assert_eq!(value["messages"][0]["role"], "user");
        assert_eq!(value["stream"], false);
    }

    #[test]
    fn deserializes_openai_compatible_models() {
        let models = serde_json::from_str::<ModelListResponse>(
            r#"{"data":[{"id":"qwen3","object":"model","owned_by":"llama.cpp"}]}"#,
        )
        .unwrap();

        assert_eq!(models.data[0].id, "qwen3");
    }
}
