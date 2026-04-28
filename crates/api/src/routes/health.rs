use axum::{extract::State, routing::get, Json, Router};
use novelgraph_core::{API_VERSION, APP_VERSION, STORAGE_SCHEMA_VERSION};
use serde::Serialize;

use crate::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

#[derive(Debug, Serialize)]
pub(crate) struct HealthResponse {
    pub status: &'static str,
    pub app_mode: &'static str,
    pub version: &'static str,
    pub api_version: &'static str,
    pub storage_schema_version: &'static str,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app_mode: state.config.mode.as_str(),
        version: APP_VERSION,
        api_version: API_VERSION,
        storage_schema_version: STORAGE_SCHEMA_VERSION,
    })
}
