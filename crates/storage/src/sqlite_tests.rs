use novelgraph_core::{CreateTranslationJobInput, NovelImportInput};

use crate::SqliteStore;

#[tokio::test]
async fn imports_novel_and_creates_translation_job() {
    let store = SqliteStore::connect_in_memory().await.unwrap();
    let project = store.create_project("Demo Project").await.unwrap();
    let projects = store.list_projects().await.unwrap();
    let fetched_project = store.get_project(&project.id).await.unwrap().unwrap();

    assert_eq!(projects.len(), 1);
    assert_eq!(fetched_project.name, "Demo Project");

    let import = store
        .import_novel(
            &project.id,
            NovelImportInput {
                title: "Truyện Thử".to_string(),
                author: Some("Tác giả".to_string()),
                source_language: Some("zh".to_string()),
                genre: None,
                description: None,
                text: "Chương 1\nMở đầu.\n\nChương 2\nTiếp tục.".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(import.chapters.len(), 2);
    assert_eq!(import.source_segment_count, 2);
    assert_eq!(import.analysis_job.status, "pending");
    let novels = store.list_novels(&project.id).await.unwrap();
    assert_eq!(novels.len(), 1);
    assert_eq!(novels[0].title, "Truyện Thử");
    let latest_analysis_job = store
        .get_latest_analysis_job_for_novel(&project.id, &import.novel.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest_analysis_job.id, import.analysis_job.id);
    let analysis_events = store
        .list_job_events(&project.id, &import.analysis_job.id)
        .await
        .unwrap();
    assert_eq!(analysis_events.len(), 1);
    assert_eq!(analysis_events[0].event_type, "analysis_job_created");
    let running_analysis_job = store
        .mark_analysis_job_running(&project.id, &import.analysis_job.id)
        .await
        .unwrap();
    assert_eq!(running_analysis_job.status, "running");
    let chapters = store
        .list_chapters(&project.id, &import.novel.id)
        .await
        .unwrap();
    let started_chapter_run = store
        .start_analysis_chapter_run(
            &project.id,
            &import.analysis_job.id,
            &import.novel.id,
            &chapters[0],
        )
        .await
        .unwrap();
    assert_eq!(started_chapter_run.status, "running");
    assert_eq!(started_chapter_run.attempt, 1);
    let completed_chapter_run = store
        .complete_analysis_chapter_run(
            &project.id,
            &import.analysis_job.id,
            &chapters[0].id,
            "draft.chapter_extraction.v0",
            "{\"persisted\":false}",
        )
        .await
        .unwrap();
    assert_eq!(completed_chapter_run.status, "completed");
    assert!(completed_chapter_run.output_json.is_some());
    let paused_analysis_job = store
        .pause_analysis_job(
            &project.id,
            &import.analysis_job.id,
            "local LLM unavailable",
            Some("local_llm_unreachable"),
            true,
        )
        .await
        .unwrap();
    assert_eq!(paused_analysis_job.status, "paused");
    let resumed_analysis_job = store
        .mark_analysis_job_running(&project.id, &import.analysis_job.id)
        .await
        .unwrap();
    assert_eq!(resumed_analysis_job.status, "running");
    let analysis_chapter_runs = store
        .list_analysis_chapter_runs(&project.id, &import.analysis_job.id)
        .await
        .unwrap();
    assert_eq!(analysis_chapter_runs.len(), 1);
    let reset_analysis_job = store
        .reset_analysis_run(&project.id, &import.analysis_job.id)
        .await
        .unwrap();
    assert_eq!(reset_analysis_job.status, "pending");
    assert!(store
        .list_analysis_chapter_runs(&project.id, &import.analysis_job.id)
        .await
        .unwrap()
        .is_empty());
    let cancelled_analysis_job = store
        .cancel_analysis_job(&project.id, &import.analysis_job.id)
        .await
        .unwrap();
    assert_eq!(cancelled_analysis_job.status, "cancelled");
    assert!(cancelled_analysis_job.finished_at.is_some());

    let translation_job = store
        .create_translation_job(
            &project.id,
            CreateTranslationJobInput {
                novel_id: import.novel.id.clone(),
                source_language: None,
                target_language: "vi".to_string(),
                provider: Some("openai".to_string()),
                model: Some("gpt-test".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(translation_job.status, "pending");
    assert_eq!(translation_job.source_language.as_deref(), Some("zh"));
    assert_eq!(translation_job.target_language, "vi");

    let translation_events = store
        .list_job_events(&project.id, &translation_job.id)
        .await
        .unwrap();
    assert_eq!(translation_events.len(), 1);
    assert_eq!(translation_events[0].event_type, "translation_job_created");
    let cancelled_translation_job = store
        .cancel_translation_job(&project.id, &translation_job.id)
        .await
        .unwrap();
    assert_eq!(cancelled_translation_job.status, "cancelled");

    let translation_events = store
        .list_job_events(&project.id, &translation_job.id)
        .await
        .unwrap();
    assert_eq!(translation_events.len(), 2);
    assert_eq!(
        translation_events[1].event_type,
        "translation_job_cancelled"
    );

    let archived_project = store.delete_project(&project.id, false).await.unwrap();
    assert_eq!(archived_project.action, "archived");
    assert!(archived_project.data_retained);
    assert!(store.get_project(&project.id).await.unwrap().is_none());
    let archived_projects = store.list_archived_projects().await.unwrap();
    assert_eq!(archived_projects.len(), 1);
    assert_eq!(archived_projects[0].id, project.id);

    let retained_novel = store
        .get_novel(&project.id, &import.novel.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retained_novel.id, import.novel.id);
    let restored_project = store.restore_project(&project.id).await.unwrap();
    assert_eq!(restored_project.id, project.id);
    assert!(store.list_archived_projects().await.unwrap().is_empty());
    let re_archived_project = store.delete_project(&project.id, false).await.unwrap();
    assert_eq!(re_archived_project.action, "archived");
    let purged_archived_project = store.delete_project(&project.id, true).await.unwrap();
    assert_eq!(purged_archived_project.action, "purged");
    assert!(store.get_project(&project.id).await.unwrap().is_none());

    let hard_delete_project = store.create_project("Hard Delete Project").await.unwrap();
    let second_import = store
        .import_novel(
            &hard_delete_project.id,
            NovelImportInput {
                title: "Truyện Xóa".to_string(),
                author: None,
                source_language: Some("vi".to_string()),
                genre: None,
                description: None,
                text: "Chương 1\nDữ liệu xóa.".to_string(),
            },
        )
        .await
        .unwrap();
    let purged_project = store
        .delete_project(&hard_delete_project.id, true)
        .await
        .unwrap();
    assert_eq!(purged_project.action, "purged");
    assert!(!purged_project.data_retained);
    assert!(store
        .get_project(&hard_delete_project.id)
        .await
        .unwrap()
        .is_none());
    assert!(store
        .get_novel(&hard_delete_project.id, &second_import.novel.id)
        .await
        .unwrap()
        .is_none());
}
