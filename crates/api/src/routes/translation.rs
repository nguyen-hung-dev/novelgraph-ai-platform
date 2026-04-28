use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use novelgraph_core::{CreateTranslationJobInput, TranslationJob};

use crate::{publish_project_event, ApiError, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/projects/{project_id}/translation/jobs",
            post(create_translation_job),
        )
        .route(
            "/api/projects/{project_id}/translation/jobs/{job_id}",
            get(get_translation_job),
        )
        .route(
            "/api/projects/{project_id}/translation/jobs/{job_id}/cancel",
            post(cancel_translation_job),
        )
}

async fn create_translation_job(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<CreateTranslationJobInput>,
) -> Result<Json<TranslationJob>, ApiError> {
    let job = state
        .store
        .create_translation_job(&project_id, input)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "translation_job_created",
        Some(&job.id),
        None,
        "translation job created",
    );
    Ok(Json(job))
}

async fn get_translation_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<TranslationJob>, ApiError> {
    let job = state
        .store
        .get_translation_job(&project_id, &job_id)
        .await?
        .ok_or(ApiError::not_found("translation_job"))?;

    Ok(Json(job))
}

async fn cancel_translation_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<TranslationJob>, ApiError> {
    let job = state
        .store
        .cancel_translation_job(&project_id, &job_id)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "translation_job_cancelled",
        Some(&job_id),
        None,
        "translation job cancelled",
    );
    Ok(Json(job))
}
