use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use novelgraph_core::{AnalysisJob, AnalysisRunSnapshot, AnalysisRunStepInput};

use crate::{services::analysis as analysis_service, ApiError, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}",
            get(get_analysis_job),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/run",
            get(get_analysis_run),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/run/step",
            post(run_next_analysis_chapter),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/run/reset",
            post(reset_analysis_run),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/pause",
            post(pause_analysis_run),
        )
        .route(
            "/api/projects/{project_id}/analysis/jobs/{job_id}/cancel",
            post(cancel_analysis_job),
        )
}

async fn get_analysis_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisJob>, ApiError> {
    Ok(Json(
        analysis_service::get_job(&state, &project_id, &job_id).await?,
    ))
}

async fn get_analysis_run(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    Ok(Json(
        analysis_service::get_run(&state, &project_id, &job_id).await?,
    ))
}

async fn reset_analysis_run(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    Ok(Json(
        analysis_service::reset_run(&state, &project_id, &job_id).await?,
    ))
}

async fn pause_analysis_run(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    Ok(Json(
        analysis_service::pause_run(&state, &project_id, &job_id).await?,
    ))
}

async fn run_next_analysis_chapter(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
    Json(input): Json<AnalysisRunStepInput>,
) -> Result<Json<AnalysisRunSnapshot>, ApiError> {
    crate::run_next_analysis_chapter(State(state), Path((project_id, job_id)), Json(input)).await
}

async fn cancel_analysis_job(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<AnalysisJob>, ApiError> {
    Ok(Json(
        analysis_service::cancel_job(&state, &project_id, &job_id).await?,
    ))
}
