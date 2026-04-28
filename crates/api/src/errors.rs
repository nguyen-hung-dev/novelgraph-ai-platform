use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use novelgraph_ai::AiError;
use novelgraph_storage::StorageError;
use serde::Serialize;

use crate::local_runtime::LocalRuntimeError;

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    code: &'static str,
    pub(crate) message: String,
}

impl ApiError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
        }
    }

    pub(crate) fn not_found(resource: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: format!("{resource} was not found"),
        }
    }

    pub(crate) fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "invalid_job_transition",
            message: message.into(),
        }
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
        }
    }
}

impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::InvalidInput(message) => Self::bad_request(message),
            StorageError::InvalidJobTransition(message) => Self::conflict(message),
            StorageError::NotFound(resource) => Self::not_found(resource),
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "storage_error",
                message: "storage operation failed".to_string(),
            },
        }
    }
}

impl From<AiError> for ApiError {
    fn from(error: AiError) -> Self {
        match error {
            AiError::InvalidRequest(message) => Self::bad_request(message),
            AiError::InvalidConfig(message) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "local_llm_config_error",
                message,
            },
            AiError::Request(_) => Self {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "local_llm_unreachable",
                message: "local LLM server is unreachable".to_string(),
            },
            AiError::HttpStatus { status, message } => Self {
                status: StatusCode::BAD_GATEWAY,
                code: "local_llm_http_error",
                message: format!("local LLM returned HTTP {status}: {message}"),
            },
        }
    }
}

impl From<LocalRuntimeError> for ApiError {
    fn from(error: LocalRuntimeError) -> Self {
        match error {
            LocalRuntimeError::SelectionCancelled
            | LocalRuntimeError::UnknownPreset(_)
            | LocalRuntimeError::MissingModel(_)
            | LocalRuntimeError::ManagedModelOutsideRepo => Self::bad_request(error.to_string()),
            LocalRuntimeError::DownloadAlreadyRunning => Self {
                status: StatusCode::CONFLICT,
                code: "local_llm_download_busy",
                message: error.to_string(),
            },
            LocalRuntimeError::InvalidBaseUrl(_) | LocalRuntimeError::StartFailed(_) => Self {
                status: StatusCode::FAILED_DEPENDENCY,
                code: "local_llm_runtime_unavailable",
                message: error.to_string(),
            },
            LocalRuntimeError::Io(_)
            | LocalRuntimeError::Request(_)
            | LocalRuntimeError::Serde(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "local_llm_runtime_error",
                message: error.to_string(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
            },
        };

        (self.status, Json(body)).into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}
