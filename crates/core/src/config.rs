use std::env;

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

        Ok(Self {
            mode,
            host,
            port,
            database_url,
            sqlite_database_path,
        })
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
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
