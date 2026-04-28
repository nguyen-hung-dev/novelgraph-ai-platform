use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use novelgraph_core::{
    AnalysisChapterState, CreateProjectInput, DeleteProjectInput, DeleteProjectResult, Project,
    ProjectWorkspaceSnapshot,
};

use crate::{publish_project_event, ApiError, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/archived", get(list_archived_projects))
        .route(
            "/api/projects/{project_id}",
            get(get_project).post(delete_project),
        )
        .route("/api/projects/{project_id}/restore", post(restore_project))
        .route(
            "/api/projects/{project_id}/workspace",
            get(get_project_workspace),
        )
}

async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<Project>>, ApiError> {
    Ok(Json(state.store.list_projects().await?))
}

async fn create_project(
    State(state): State<AppState>,
    Json(input): Json<CreateProjectInput>,
) -> Result<Json<Project>, ApiError> {
    let project = state.store.create_project(&input.name).await?;
    publish_project_event(
        &state,
        &project.id,
        "project_created",
        None,
        None,
        "project created",
    );
    Ok(Json(project))
}

async fn list_archived_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<Project>>, ApiError> {
    Ok(Json(state.store.list_archived_projects().await?))
}

async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<Project>, ApiError> {
    let project = state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;

    Ok(Json(project))
}

async fn delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<DeleteProjectInput>,
) -> Result<Json<DeleteProjectResult>, ApiError> {
    let result = state
        .store
        .delete_project(&project_id, input.purge_data)
        .await?;
    publish_project_event(
        &state,
        &project_id,
        "project_deleted",
        None,
        None,
        "project deleted",
    );
    Ok(Json(result))
}

async fn restore_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<Project>, ApiError> {
    let project = state.store.restore_project(&project_id).await?;
    publish_project_event(
        &state,
        &project_id,
        "project_restored",
        None,
        None,
        "project restored",
    );
    Ok(Json(project))
}

async fn get_project_workspace(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectWorkspaceSnapshot>, ApiError> {
    let project = state
        .store
        .get_project(&project_id)
        .await?
        .ok_or(ApiError::not_found("project"))?;
    let novels = state.store.list_novels(&project_id).await?;
    let active_novel = novels.first().cloned();
    let chapters = match &active_novel {
        Some(novel) => state.store.list_chapters(&project_id, &novel.id).await?,
        None => Vec::new(),
    };
    let latest_analysis_job = match &active_novel {
        Some(novel) => {
            state
                .store
                .get_latest_analysis_job_for_novel(&project_id, &novel.id)
                .await?
        }
        None => None,
    };
    let latest_analysis_chapters = match &latest_analysis_job {
        Some(job) => {
            let runs = state
                .store
                .list_analysis_chapter_runs(&project_id, &job.id)
                .await?;
            let run_by_chapter = runs
                .iter()
                .map(|run| (run.chapter_id.as_str(), run))
                .collect::<HashMap<_, _>>();

            chapters
                .iter()
                .map(|chapter| {
                    let run = run_by_chapter.get(chapter.id.as_str()).copied();

                    AnalysisChapterState {
                        chapter_id: chapter.id.clone(),
                        chapter_num: chapter.chapter_num,
                        title: chapter.title.clone(),
                        status: run
                            .map(|run| run.status.clone())
                            .unwrap_or_else(|| "pending".to_string()),
                        run_id: run.map(|run| run.id.clone()),
                        attempt: run.map(|run| run.attempt),
                        prompt_schema_version: run
                            .and_then(|run| run.prompt_schema_version.clone()),
                        error_code: run.and_then(|run| run.error_code.clone()),
                        error_message: run.and_then(|run| run.error_message.clone()),
                        started_at: run.and_then(|run| run.started_at.clone()),
                        finished_at: run.and_then(|run| run.finished_at.clone()),
                        updated_at: run.map(|run| run.updated_at.clone()),
                    }
                })
                .collect()
        }
        None => Vec::new(),
    };
    let latest_job_events = match &latest_analysis_job {
        Some(job) => state.store.list_job_events(&project_id, &job.id).await?,
        None => Vec::new(),
    };
    let character_records = match &latest_analysis_job {
        Some(job) => {
            state
                .store
                .list_story_extraction_records(&project_id, &job.id, "character")
                .await?
        }
        None => Vec::new(),
    };
    let relationship_records = match &latest_analysis_job {
        Some(job) => {
            state
                .store
                .list_story_extraction_records(&project_id, &job.id, "relationship")
                .await?
        }
        None => Vec::new(),
    };
    let character_aliases = match &latest_analysis_job {
        Some(job) => {
            state
                .store
                .list_story_character_aliases(&project_id, &job.id)
                .await?
        }
        None => Vec::new(),
    };

    Ok(Json(ProjectWorkspaceSnapshot {
        project,
        novels,
        active_novel,
        chapters,
        latest_analysis_job,
        latest_analysis_chapters,
        latest_job_events,
        character_aliases,
        character_records,
        relationship_records,
    }))
}
