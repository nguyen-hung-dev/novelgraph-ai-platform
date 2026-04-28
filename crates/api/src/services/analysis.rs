use std::collections::HashMap;

use novelgraph_core::{
    AnalysisChapterRun, AnalysisChapterState, AnalysisJob, AnalysisRunSnapshot,
    AnalysisRunStepInput, Chapter,
};
use novelgraph_storage::SqliteStore;

use crate::{publish_project_event, ApiError, AppState};

pub(crate) async fn get_job(
    state: &AppState,
    project_id: &str,
    job_id: &str,
) -> Result<AnalysisJob, ApiError> {
    state
        .store
        .get_analysis_job(project_id, job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))
}

pub(crate) async fn get_run(
    state: &AppState,
    project_id: &str,
    job_id: &str,
) -> Result<AnalysisRunSnapshot, ApiError> {
    build_run_snapshot(&state.store, project_id, job_id, None).await
}

pub(crate) async fn reset_run(
    state: &AppState,
    project_id: &str,
    job_id: &str,
) -> Result<AnalysisRunSnapshot, ApiError> {
    state.store.reset_analysis_run(project_id, job_id).await?;
    publish_project_event(
        state,
        project_id,
        "analysis_reset",
        Some(job_id),
        None,
        "analysis run reset",
    );

    build_run_snapshot(&state.store, project_id, job_id, None).await
}

pub(crate) async fn pause_run(
    state: &AppState,
    project_id: &str,
    job_id: &str,
) -> Result<AnalysisRunSnapshot, ApiError> {
    let reason = "Tạm dừng bởi người dùng.";
    state
        .store
        .pause_analysis_job(project_id, job_id, reason, None, false)
        .await?;
    publish_project_event(
        state,
        project_id,
        "analysis_paused",
        Some(job_id),
        None,
        "analysis run paused",
    );

    build_run_snapshot(&state.store, project_id, job_id, Some(reason.to_string())).await
}

pub(crate) async fn cancel_job(
    state: &AppState,
    project_id: &str,
    job_id: &str,
) -> Result<AnalysisJob, ApiError> {
    let job = state.store.cancel_analysis_job(project_id, job_id).await?;
    publish_project_event(
        state,
        project_id,
        "analysis_cancelled",
        Some(job_id),
        None,
        "analysis job cancelled",
    );

    Ok(job)
}

pub(crate) async fn build_run_snapshot(
    store: &SqliteStore,
    project_id: &str,
    job_id: &str,
    paused_reason: Option<String>,
) -> Result<AnalysisRunSnapshot, ApiError> {
    let job = store
        .get_analysis_job(project_id, job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))?;
    let novel_id = job
        .novel_id
        .as_deref()
        .ok_or_else(|| ApiError::bad_request("analysis job is not attached to a novel"))?;
    let chapters = store.list_chapters(project_id, novel_id).await?;
    let runs = store.list_analysis_chapter_runs(project_id, job_id).await?;
    let character_records = store
        .list_story_extraction_records(project_id, job_id, "character")
        .await?;
    let relationship_records = store
        .list_story_extraction_records(project_id, job_id, "relationship")
        .await?;
    let character_aliases = store
        .list_story_character_aliases(project_id, job_id)
        .await?;
    let run_by_chapter = runs
        .iter()
        .map(|run| (run.chapter_id.as_str(), run))
        .collect::<HashMap<_, _>>();

    let chapter_states = chapters
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
                prompt_schema_version: run.and_then(|run| run.prompt_schema_version.clone()),
                error_code: run.and_then(|run| run.error_code.clone()),
                error_message: run.and_then(|run| run.error_message.clone()),
                started_at: run.and_then(|run| run.started_at.clone()),
                finished_at: run.and_then(|run| run.finished_at.clone()),
                updated_at: run.map(|run| run.updated_at.clone()),
            }
        })
        .collect::<Vec<_>>();

    let completed_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "completed")
        .count();
    let running_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "running")
        .count();
    let failed_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "failed")
        .count();
    let pending_chapters = chapter_states
        .iter()
        .filter(|chapter| chapter.status == "pending")
        .count();
    let next_chapter_num = chapter_states
        .iter()
        .find(|chapter| chapter.status != "completed")
        .map(|chapter| chapter.chapter_num);
    let paused_reason = paused_reason.or_else(|| {
        if job.status == "paused" {
            job.error_message.clone()
        } else {
            None
        }
    });

    Ok(AnalysisRunSnapshot {
        job,
        total_chapters: chapters.len(),
        completed_chapters,
        running_chapters,
        failed_chapters,
        pending_chapters,
        next_chapter_num,
        paused_reason,
        chapters: chapter_states,
        character_aliases,
        character_records,
        relationship_records,
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChapterRange {
    pub(crate) from: i64,
    pub(crate) to: i64,
}

pub(crate) fn chapter_range_from_input(
    input: &AnalysisRunStepInput,
) -> Result<Option<ChapterRange>, ApiError> {
    match (input.from_chapter_num, input.to_chapter_num) {
        (None, None) => Ok(None),
        (Some(from), None) => validate_chapter_range(from, from).map(Some),
        (Some(from), Some(to)) => validate_chapter_range(from, to).map(Some),
        (None, Some(to)) => validate_chapter_range(to, to).map(Some),
    }
}

fn validate_chapter_range(from: i64, to: i64) -> Result<ChapterRange, ApiError> {
    if from < 1 || to < 1 {
        return Err(ApiError::bad_request(
            "chapter range must start from 1 or greater",
        ));
    }

    if to < from {
        return Err(ApiError::bad_request(
            "to chapter must be greater than or equal to from chapter",
        ));
    }

    Ok(ChapterRange { from, to })
}

pub(crate) async fn finish_range_or_job(
    store: &SqliteStore,
    project_id: &str,
    job_id: &str,
    chapters: &[Chapter],
    runs: &[AnalysisChapterRun],
    range: Option<ChapterRange>,
) -> Result<(), ApiError> {
    if next_chapter(chapters, runs, None).is_none() {
        store.complete_analysis_job(project_id, job_id).await?;
        return Ok(());
    }

    if let Some(range) = range {
        if next_chapter(chapters, runs, Some(range)).is_none() {
            let reason = format!(
                "Đã chạy xong phạm vi chương {} -> {}.",
                range.from, range.to
            );
            store
                .pause_analysis_job(
                    project_id,
                    job_id,
                    &reason,
                    Some("analysis_range_completed"),
                    true,
                )
                .await?;
        }
    }

    Ok(())
}

pub(crate) fn next_chapter<'a>(
    chapters: &'a [Chapter],
    runs: &[AnalysisChapterRun],
    range: Option<ChapterRange>,
) -> Option<&'a Chapter> {
    let completed_by_chapter = runs
        .iter()
        .filter(|run| run.status == "completed")
        .map(|run| run.chapter_id.as_str())
        .collect::<std::collections::HashSet<_>>();

    chapters
        .iter()
        .filter(|chapter| {
            range
                .map(|range| chapter.chapter_num >= range.from && chapter.chapter_num <= range.to)
                .unwrap_or(true)
        })
        .find(|chapter| !completed_by_chapter.contains(chapter.id.as_str()))
}
