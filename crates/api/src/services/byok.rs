use std::{fs, time::Duration};

use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use novelgraph_core::{
    AppConfig, ByokProviderConfigRecord, ByokProviderConfigView, ByokProviderKeyHealth,
    ByokProviderPreset, CheckByokProviderKeyInput, SaveByokProviderConfigInput,
    SaveByokProviderConfigResult,
};
use ring::{
    aead,
    digest::{digest, SHA256},
    rand::{SecureRandom, SystemRandom},
};

use crate::{ApiError, AppState};

const GEMINI_OPENAI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";
const GEMINI_DEFAULT_MODEL: &str = "gemini-2.5-flash";
const SECRET_CIPHERTEXT_PREFIX: &str = "ngenc:v1";

pub(crate) fn provider_presets() -> Vec<ByokProviderPreset> {
    vec![
        ByokProviderPreset {
            id: "gemini".to_string(),
            name: "Google Gemini".to_string(),
            base_url: GEMINI_OPENAI_BASE_URL.to_string(),
            default_model: GEMINI_DEFAULT_MODEL.to_string(),
            models: vec![
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-pro".to_string(),
                "gemini-2.0-flash".to_string(),
            ],
            api_format: "openai".to_string(),
        },
        ByokProviderPreset {
            id: "openai-compatible".to_string(),
            name: "OpenAI-compatible".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            default_model: "provider-model-id".to_string(),
            models: Vec::new(),
            api_format: "openai".to_string(),
        },
    ]
}

pub(crate) fn config_view(record: Option<&ByokProviderConfigRecord>) -> ByokProviderConfigView {
    let default_preset = provider_preset("gemini").expect("gemini preset exists");
    let provider = record
        .map(|record| record.provider.clone())
        .unwrap_or(default_preset.id);
    let preset = provider_preset(&provider);

    ByokProviderConfigView {
        provider,
        display_name: record
            .map(|record| record.display_name.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.name.clone()))
            .unwrap_or_else(|| "Google Gemini".to_string()),
        base_url: record
            .and_then(|record| record.base_url.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.base_url.clone()))
            .unwrap_or_else(|| GEMINI_OPENAI_BASE_URL.to_string()),
        model: record
            .and_then(|record| record.model.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.default_model.clone()))
            .unwrap_or_else(|| GEMINI_DEFAULT_MODEL.to_string()),
        api_format: record
            .map(|record| record.api_format.clone())
            .or_else(|| preset.as_ref().map(|preset| preset.api_format.clone()))
            .unwrap_or_else(|| "openai".to_string()),
        has_api_key: record
            .and_then(|record| record.encrypted_secret_ref.as_ref())
            .is_some(),
        api_key_masked: record
            .and_then(|record| record.encrypted_secret_ref.as_ref())
            .map(|_| "********".to_string())
            .unwrap_or_default(),
        key_fingerprint: record.and_then(|record| record.key_fingerprint.clone()),
        session_only: record.map(|record| record.session_only).unwrap_or(false),
        last_checked_at: record.and_then(|record| record.last_checked_at.clone()),
        last_health_status: record.and_then(|record| record.last_health_status.clone()),
        updated_at: record.map(|record| record.updated_at.clone()),
    }
}

pub(crate) async fn save_config(
    state: &AppState,
    input: SaveByokProviderConfigInput,
) -> Result<SaveByokProviderConfigResult, ApiError> {
    let provider = normalized_provider(&input.provider)?;
    let preset = provider_preset(&provider);
    let display_name = preset
        .as_ref()
        .map(|provider| provider.name.as_str())
        .unwrap_or(provider.as_str());
    let api_format = preset
        .as_ref()
        .map(|provider| provider.api_format.as_str())
        .unwrap_or("openai");
    let base_url = normalized_url(&input.base_url)?;
    let model = require_text(&input.model, "model")?;
    let api_key = optional_api_key(input.api_key);
    let (encrypted_secret_ref, key_fingerprint, saved_api_key) = if input.session_only {
        (None, None, false)
    } else if let Some(api_key) = api_key.as_deref() {
        (
            Some(seal_secret(&state.config, api_key)?),
            Some(secret_fingerprint(api_key)),
            true,
        )
    } else {
        (None, None, false)
    };

    let record = state
        .store
        .save_local_byok_provider_config(
            &provider,
            display_name,
            &base_url,
            &model,
            api_format,
            encrypted_secret_ref.as_deref(),
            key_fingerprint.as_deref(),
            input.session_only,
        )
        .await?;

    Ok(SaveByokProviderConfigResult {
        config: config_view(Some(&record)),
        saved_api_key,
    })
}

pub(crate) async fn check_key(
    state: &AppState,
    input: CheckByokProviderKeyInput,
) -> Result<ByokProviderKeyHealth, ApiError> {
    let provider = normalized_provider(&input.provider)?;
    let base_url = normalized_url(&input.base_url)?;
    let model = require_text(&input.model, "model")?;
    let api_key = match optional_api_key(input.api_key) {
        Some(api_key) => api_key,
        None => {
            let record = state
                .store
                .get_local_byok_provider_config_for_provider(&provider)
                .await?;
            let encrypted_secret_ref = record
                .as_ref()
                .and_then(|record| record.encrypted_secret_ref.as_deref())
                .ok_or_else(|| ApiError::bad_request("API key is required"))?;
            open_secret(&state.config, encrypted_secret_ref)?
        }
    };

    let health = probe_provider_key(&provider, &base_url, &model, &api_key).await;
    let status = if health.valid { "valid" } else { "invalid" };
    let _ = state
        .store
        .update_local_byok_provider_health(&provider, status)
        .await;

    Ok(health)
}

fn provider_preset(provider: &str) -> Option<ByokProviderPreset> {
    provider_presets()
        .into_iter()
        .find(|preset| preset.id == provider)
}

fn normalized_provider(provider: &str) -> Result<String, ApiError> {
    let provider = require_text(provider, "provider")?;
    if provider
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        Ok(provider)
    } else {
        Err(ApiError::bad_request(
            "provider contains unsupported characters",
        ))
    }
}

fn normalized_url(value: &str) -> Result<String, ApiError> {
    let value = require_text(value, "base_url")?;
    let value = value.trim_end_matches('/').to_string();
    if value.starts_with("https://") || value.starts_with("http://") {
        Ok(value)
    } else {
        Err(ApiError::bad_request(
            "base_url must start with http:// or https://",
        ))
    }
}

fn require_text(value: &str, field: &str) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApiError::bad_request(format!("{field} is required")));
    }

    Ok(value.to_string())
}

fn optional_api_key(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !value.chars().all(|ch| ch == '*'))
        .map(ToOwned::to_owned)
}

fn secret_fingerprint(secret: &str) -> String {
    let digest = digest(&SHA256, secret.as_bytes());
    STANDARD_NO_PAD.encode(&digest.as_ref()[..9])
}

fn seal_secret(config: &AppConfig, secret: &str) -> Result<String, ApiError> {
    let key_bytes = load_or_create_secret_key(config)?;
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| ApiError::internal("failed to prepare BYOK encryption key"))?;
    let sealing_key = aead::LessSafeKey::new(unbound_key);
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0_u8; 12];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| ApiError::internal("failed to generate BYOK encryption nonce"))?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let mut ciphertext = secret.as_bytes().to_vec();

    sealing_key
        .seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut ciphertext)
        .map_err(|_| ApiError::internal("failed to encrypt BYOK key"))?;

    Ok(format!(
        "{SECRET_CIPHERTEXT_PREFIX}:{}:{}",
        STANDARD_NO_PAD.encode(nonce_bytes),
        STANDARD_NO_PAD.encode(ciphertext)
    ))
}

fn open_secret(config: &AppConfig, value: &str) -> Result<String, ApiError> {
    let mut parts = value.split(':');
    let prefix = parts.next();
    let version = parts.next();
    let nonce = parts.next();
    let ciphertext = parts.next();
    if prefix != Some("ngenc") || version != Some("v1") || parts.next().is_some() {
        return Err(ApiError::internal("stored BYOK key format is unsupported"));
    }

    let nonce = nonce.ok_or_else(|| ApiError::internal("stored BYOK key nonce is missing"))?;
    let ciphertext =
        ciphertext.ok_or_else(|| ApiError::internal("stored BYOK ciphertext is missing"))?;
    let nonce_bytes = STANDARD_NO_PAD
        .decode(nonce)
        .map_err(|_| ApiError::internal("stored BYOK key nonce is invalid"))?;
    let mut nonce_array = [0_u8; 12];
    if nonce_bytes.len() != nonce_array.len() {
        return Err(ApiError::internal(
            "stored BYOK key nonce length is invalid",
        ));
    }
    nonce_array.copy_from_slice(&nonce_bytes);

    let mut ciphertext = STANDARD_NO_PAD
        .decode(ciphertext)
        .map_err(|_| ApiError::internal("stored BYOK ciphertext is invalid"))?;
    let key_bytes = load_or_create_secret_key(config)?;
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| ApiError::internal("failed to prepare BYOK encryption key"))?;
    let opening_key = aead::LessSafeKey::new(unbound_key);
    let plaintext = opening_key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_array),
            aead::Aad::empty(),
            &mut ciphertext,
        )
        .map_err(|_| ApiError::internal("failed to decrypt stored BYOK key"))?;

    String::from_utf8(plaintext.to_vec())
        .map_err(|_| ApiError::internal("stored BYOK key is not valid UTF-8"))
}

fn load_or_create_secret_key(config: &AppConfig) -> Result<[u8; 32], ApiError> {
    if let Some(secret) = &config.secrets_encryption_key {
        let digest = digest(&SHA256, secret.as_bytes());
        let mut key = [0_u8; 32];
        key.copy_from_slice(digest.as_ref());
        return Ok(key);
    }

    if config.secrets_key_path.exists() {
        let encoded = fs::read_to_string(&config.secrets_key_path)
            .map_err(|_| ApiError::internal("failed to read local BYOK encryption key"))?;
        let decoded = STANDARD_NO_PAD
            .decode(encoded.trim())
            .map_err(|_| ApiError::internal("local BYOK encryption key is invalid"))?;
        let mut key = [0_u8; 32];
        if decoded.len() != key.len() {
            return Err(ApiError::internal(
                "local BYOK encryption key length is invalid",
            ));
        }
        key.copy_from_slice(&decoded);
        return Ok(key);
    }

    if let Some(parent) = config.secrets_key_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| ApiError::internal("failed to create local secrets directory"))?;
    }

    let rng = SystemRandom::new();
    let mut key = [0_u8; 32];
    rng.fill(&mut key)
        .map_err(|_| ApiError::internal("failed to generate local BYOK encryption key"))?;
    fs::write(&config.secrets_key_path, STANDARD_NO_PAD.encode(key))
        .map_err(|_| ApiError::internal("failed to write local BYOK encryption key"))?;

    Ok(key)
}

async fn probe_provider_key(
    provider: &str,
    base_url: &str,
    model: &str,
    api_key: &str,
) -> ByokProviderKeyHealth {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return byok_health(
                provider,
                base_url,
                model,
                false,
                None,
                "HTTP client setup failed",
            );
        }
    };
    let url = if provider == "gemini" {
        format!("{base_url}/models/{model}")
    } else {
        format!("{base_url}/models")
    };

    match client.get(url).bearer_auth(api_key).send().await {
        Ok(response) if response.status().is_success() => byok_health(
            provider,
            base_url,
            model,
            true,
            Some(response.status().as_u16()),
            "Provider accepted the API key",
        ),
        Ok(response) if matches!(response.status().as_u16(), 401 | 403) => byok_health(
            provider,
            base_url,
            model,
            false,
            Some(response.status().as_u16()),
            "Provider rejected the API key",
        ),
        Ok(response) => byok_health(
            provider,
            base_url,
            model,
            false,
            Some(response.status().as_u16()),
            "Provider returned a non-success status",
        ),
        Err(_) => byok_health(
            provider,
            base_url,
            model,
            false,
            None,
            "Provider health request failed",
        ),
    }
}

fn byok_health(
    provider: &str,
    base_url: &str,
    model: &str,
    valid: bool,
    status_code: Option<u16>,
    message: &str,
) -> ByokProviderKeyHealth {
    ByokProviderKeyHealth {
        provider: provider.to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        valid,
        status_code,
        message: message.to_string(),
        checked_at: unix_timestamp_label(),
    }
}

fn unix_timestamp_label() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix:{seconds}")
}
