use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use novelgraph_core::JobEvent;

use crate::{ApiError, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new().route(
        "/api/projects/{project_id}/jobs/{job_id}/events",
        get(list_job_events),
    )
}

async fn list_job_events(
    State(state): State<AppState>,
    Path((project_id, job_id)): Path<(String, String)>,
) -> Result<Json<Vec<JobEvent>>, ApiError> {
    Ok(Json(
        state.store.list_job_events(&project_id, &job_id).await?,
    ))
}
