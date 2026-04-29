use std::{env, path::PathBuf};

use crate::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Web,
    Desktop,
    Demo,
}

impl AppMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Desktop => "desktop",
            Self::Demo => "demo",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "web" => Ok(Self::Web),
            "desktop" => Ok(Self::Desktop),
            "demo" => Ok(Self::Demo),
            other => Err(AppError::InvalidConfig(format!(
                "unsupported APP_MODE: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mode: AppMode,
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub sqlite_database_path: Option<String>,
    pub llama_cpp_base_url: String,
    pub llama_cpp_default_model: String,
    pub llama_cpp_server_bin: String,
    pub llama_cpp_timeout_secs: u64,
    pub gemini_base_url: String,
    pub gemini_timeout_secs: u64,
    pub secrets_encryption_key: Option<String>,
    pub secrets_key_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> AppResult<Self> {
        let mode = AppMode::parse(&env::var("APP_MODE").unwrap_or_else(|_| "web".to_string()))?;
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|err| AppError::InvalidConfig(format!("invalid PORT: {err}")))?;

        let database_url = env::var("DATABASE_URL").ok();
        let sqlite_database_path = env::var("SQLITE_DATABASE_PATH").ok().or_else(|| {
            database_url
                .is_none()
                .then(|| "data/novelgraph.sqlite3".to_string())
        });
        let secrets_key_path = env::var("SECRETS_KEY_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let data_dir = sqlite_database_path
                    .as_deref()
                    .and_then(|path| PathBuf::from(path).parent().map(PathBuf::from))
                    .unwrap_or_else(|| PathBuf::from("data"));

                data_dir.join("secrets").join("byok.key")
            });
        let llama_cpp_timeout_secs = env::var("LLAMA_CPP_TIMEOUT_SECS")
            .unwrap_or_else(|_| "120".to_string())
            .parse::<u64>()
            .map_err(|err| {
                AppError::InvalidConfig(format!("invalid LLAMA_CPP_TIMEOUT_SECS: {err}"))
            })?;
        let gemini_timeout_secs = env::var("GEMINI_TIMEOUT_SECS")
            .unwrap_or_else(|_| "120".to_string())
            .parse::<u64>()
            .map_err(|err| {
                AppError::InvalidConfig(format!("invalid GEMINI_TIMEOUT_SECS: {err}"))
            })?;

        Ok(Self {
            mode,
            host,
            port,
            database_url,
            sqlite_database_path,
            llama_cpp_base_url: env::var("LLAMA_CPP_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string()),
            llama_cpp_default_model: env::var("LLAMA_CPP_DEFAULT_MODEL")
                .unwrap_or_else(|_| "qwen3".to_string()),
            llama_cpp_server_bin: env::var("LLAMA_CPP_SERVER_BIN")
                .unwrap_or_else(|_| default_llama_cpp_server_bin()),
            llama_cpp_timeout_secs,
            gemini_base_url: env::var("GEMINI_BASE_URL")
                .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta".to_string()),
            gemini_timeout_secs,
            secrets_encryption_key: env::var("SECRETS_ENCRYPTION_KEY")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            secrets_key_path,
        })
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn default_llama_cpp_server_bin() -> String {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let bundled_windows = repo_root
        .join("tools")
        .join("llama.cpp")
        .join("llama-server.exe");

    if bundled_windows.exists() {
        return bundled_windows.to_string_lossy().into_owned();
    }

    "llama-server".to_string()
}

#[cfg(test)]
mod tests {
    use super::AppMode;

    #[test]
    fn parses_app_modes() {
        assert_eq!(AppMode::parse("web").unwrap(), AppMode::Web);
        assert_eq!(AppMode::parse("desktop").unwrap(), AppMode::Desktop);
        assert_eq!(AppMode::parse("demo").unwrap(), AppMode::Demo);
        assert!(AppMode::parse("invalid").is_err());
    }
}
