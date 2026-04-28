use novelgraph_core::{
    AnalysisChapterRun, AnalysisRunSnapshot, AnalysisRunStepInput, Chapter, Novel,
};
use novelgraph_storage::SqliteStore;

use crate::{
    publish_project_event,
    services::analysis::{self, ChapterRange},
    ApiError, AppState,
};

pub(crate) struct AnalysisStepReady {
    pub(crate) novel: Novel,
    pub(crate) chapters: Vec<Chapter>,
    pub(crate) chapter: Chapter,
    pub(crate) chapter_run: AnalysisChapterRun,
    pub(crate) chapter_range: Option<ChapterRange>,
}

pub(crate) enum AnalysisStepPreflight {
    Snapshot(AnalysisRunSnapshot),
    Ready(AnalysisStepReady),
}

pub(crate) async fn prepare_analysis_step(
    state: &AppState,
    project_id: &str,
    job_id: &str,
    input: &AnalysisRunStepInput,
) -> Result<AnalysisStepPreflight, ApiError> {
    let chapter_range = analysis::chapter_range_from_input(input)?;
    if input.force {
        if let Some(range) = chapter_range {
            state
                .store
                .reset_analysis_run_range(project_id, job_id, range.from, range.to)
                .await?;
        } else {
            state.store.reset_analysis_run(project_id, job_id).await?;
        }
        publish_project_event(
            state,
            project_id,
            "analysis_reset",
            Some(job_id),
            None,
            "analysis run reset before force run",
        );
    }

    let current_job = state
        .store
        .get_analysis_job(project_id, job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))?;
    if current_job.status == "completed" {
        let snapshot = analysis::build_run_snapshot(&state.store, project_id, job_id, None).await?;
        return Ok(AnalysisStepPreflight::Snapshot(snapshot));
    }

    let job = state
        .store
        .mark_analysis_job_running(project_id, job_id)
        .await?;
    publish_project_event(
        state,
        project_id,
        "analysis_running",
        Some(job_id),
        None,
        "analysis job marked running",
    );
    let novel_id = job
        .novel_id
        .as_deref()
        .ok_or_else(|| ApiError::bad_request("analysis job is not attached to a novel"))?;
    let novel = state
        .store
        .get_novel(project_id, novel_id)
        .await?
        .ok_or(ApiError::not_found("novel"))?;
    let chapters = state.store.list_chapters(project_id, novel_id).await?;
    let runs = state
        .store
        .list_analysis_chapter_runs(project_id, job_id)
        .await?;

    if analysis::next_chapter(&chapters, &runs, chapter_range).is_none() {
        analysis::finish_range_or_job(
            &state.store,
            project_id,
            job_id,
            &chapters,
            &runs,
            chapter_range,
        )
        .await?;
        publish_project_event(
            state,
            project_id,
            "analysis_finished",
            Some(job_id),
            None,
            "analysis range or job finished",
        );
        let snapshot = analysis::build_run_snapshot(&state.store, project_id, job_id, None).await?;
        return Ok(AnalysisStepPreflight::Snapshot(snapshot));
    }

    let health = state.local_llm.health().await?;
    if !health.reachable {
        let reason = format!(
            "Local llama.cpp không reachable: {}",
            health
                .status_text
                .unwrap_or_else(|| "request failed".to_string())
        );
        state
            .store
            .pause_analysis_job(
                project_id,
                job_id,
                &reason,
                Some("local_llm_unreachable"),
                true,
            )
            .await?;
        publish_project_event(
            state,
            project_id,
            "analysis_paused",
            Some(job_id),
            None,
            "local LLM unreachable",
        );

        let snapshot =
            analysis::build_run_snapshot(&state.store, project_id, job_id, Some(reason)).await?;
        return Ok(AnalysisStepPreflight::Snapshot(snapshot));
    }

    let chapter = analysis::next_chapter(&chapters, &runs, chapter_range)
        .cloned()
        .ok_or_else(|| ApiError::bad_request("no chapter is available for analysis"))?;
    let chapter_run = state
        .store
        .start_analysis_chapter_run(project_id, job_id, novel_id, &chapter)
        .await?;
    publish_project_event(
        state,
        project_id,
        "analysis_chapter_started",
        Some(job_id),
        Some(&chapter.id),
        "analysis chapter started",
    );

    Ok(AnalysisStepPreflight::Ready(AnalysisStepReady {
        novel,
        chapters,
        chapter,
        chapter_run,
        chapter_range,
    }))
}

pub(crate) async fn fail_analysis_chapter_and_pause(
    state: &AppState,
    project_id: &str,
    job_id: &str,
    chapter_id: &str,
    error_code: &'static str,
    reason: String,
) -> Result<AnalysisRunSnapshot, ApiError> {
    state
        .store
        .fail_analysis_chapter_run(project_id, job_id, chapter_id, error_code, &reason)
        .await?;
    state
        .store
        .pause_analysis_job(project_id, job_id, &reason, Some(error_code), true)
        .await?;
    publish_project_event(
        state,
        project_id,
        "analysis_paused",
        Some(job_id),
        Some(chapter_id),
        error_code,
    );

    analysis::build_run_snapshot(&state.store, project_id, job_id, Some(reason)).await
}

pub(crate) async fn analysis_job_should_stop(
    store: &SqliteStore,
    project_id: &str,
    job_id: &str,
) -> Result<bool, ApiError> {
    let job = store
        .get_analysis_job(project_id, job_id)
        .await?
        .ok_or(ApiError::not_found("analysis_job"))?;

    Ok(matches!(
        job.status.as_str(),
        "paused" | "cancelled" | "completed" | "failed"
    ))
}
