use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use novelgraph_core::{
    build_import_preview, Chapter, ImportPreview, Novel, NovelImportInput, NovelImportResult,
    NovelMetadataSuggestion, NovelMetadataUpdateInput,
};

use crate::{publish_project_event, services::novels as novel_service, ApiError, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/projects/{project_id}/novels/import/preview",
            post(preview_novel_import),
        )
        .route(
            "/api/projects/{project_id}/novels/import/metadata-suggest",
            post(suggest_novel_import_metadata),
        )
        .route(
            "/api/projects/{project_id}/novels/import/confirm",
            post(confirm_novel_import),
        )
        .route(
            "/api/projects/{project_id}/novels/{novel_id}",
            get(get_novel),
        )
        .route(
            "/api/projects/{project_id}/novels/{novel_id}/metadata",
            post(update_novel_metadata),
        )
        .route(
            "/api/projects/{project_id}/novels/{novel_id}/metadata/ai-fill",
            post(ai_fill_novel_metadata),
        )
        .route(
            "/api/projects/{project_id}/novels/{novel_id}/chapters",
            get(list_chapters),
        )
}

async fn preview_novel_import(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<NovelImportInput>,
) -> Result<Json<ImportPreview>, ApiError> {
    state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;
    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("novel text is required"));
    }

    Ok(Json(build_import_preview(&input)))
}

async fn suggest_novel_import_metadata(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<NovelImportInput>,
) -> Result<Json<NovelMetadataSuggestion>, ApiError> {
    state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;
    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("novel text is required"));
    }

    Ok(Json(
        novel_service::suggest_novel_metadata(&state, input).await?,
    ))
}

async fn confirm_novel_import(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<NovelImportInput>,
) -> Result<Json<NovelImportResult>, ApiError> {
    let result = state.store.import_novel(&project_id, input).await?;
    publish_project_event(
        &state,
        &project_id,
        "novel_imported",
        None,
        None,
        "novel imported",
    );
    Ok(Json(result))
}

async fn update_novel_metadata(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
    Json(mut input): Json<NovelMetadataUpdateInput>,
) -> Result<Json<Novel>, ApiError> {
    novel_service::fill_source_language_if_auto(&state, &project_id, &novel_id, &mut input).await?;
    let novel = state
        .store
        .update_novel_metadata(&project_id, &novel_id, input)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "novel_metadata_updated",
        None,
        None,
        "novel metadata updated",
    );
    Ok(Json(novel))
}

async fn ai_fill_novel_metadata(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
) -> Result<Json<Novel>, ApiError> {
    let updated = novel_service::ai_fill_novel_metadata(&state, &project_id, &novel_id).await?;
    publish_project_event(
        &state,
        &project_id,
        "novel_metadata_updated",
        None,
        None,
        "novel metadata updated by AI",
    );
    Ok(Json(updated))
}

async fn get_novel(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
) -> Result<Json<Novel>, ApiError> {
    let novel = state
        .store
        .get_novel(&project_id, &novel_id)
        .await?
        .ok_or(ApiError::not_found("novel"))?;

    Ok(Json(novel))
}

async fn list_chapters(
    State(state): State<AppState>,
    Path((project_id, novel_id)): Path<(String, String)>,
) -> Result<Json<Vec<Chapter>>, ApiError> {
    Ok(Json(
        state.store.list_chapters(&project_id, &novel_id).await?,
    ))
}
