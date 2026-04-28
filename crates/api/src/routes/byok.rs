use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use novelgraph_core::{
    ByokProviderConfigView, ByokProviderKeyHealth, ByokProviderPreset, CheckByokProviderKeyInput,
    SaveByokProviderConfigInput, SaveByokProviderConfigResult,
};

use crate::{services::byok as byok_service, ApiError, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/byok/providers", get(list_byok_providers))
        .route(
            "/api/byok/config",
            get(get_byok_config).post(save_byok_config),
        )
        .route("/api/byok/health-check", post(check_byok_key))
}

async fn list_byok_providers() -> Json<Vec<ByokProviderPreset>> {
    Json(byok_service::provider_presets())
}

async fn get_byok_config(
    State(state): State<AppState>,
) -> Result<Json<ByokProviderConfigView>, ApiError> {
    let record = state.store.get_local_byok_provider_config().await?;
    Ok(Json(byok_service::config_view(record.as_ref())))
}

async fn save_byok_config(
    State(state): State<AppState>,
    Json(input): Json<SaveByokProviderConfigInput>,
) -> Result<Json<SaveByokProviderConfigResult>, ApiError> {
    byok_service::save_config(&state, input).await.map(Json)
}

async fn check_byok_key(
    State(state): State<AppState>,
    Json(input): Json<CheckByokProviderKeyInput>,
) -> Result<Json<ByokProviderKeyHealth>, ApiError> {
    byok_service::check_key(&state, input).await.map(Json)
}
