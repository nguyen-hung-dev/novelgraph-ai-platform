use novelgraph_core::{
    build_novel_metadata_suggestion_prompt, detect_basic_source_language, Novel, NovelImportInput,
    NovelMetadataSuggestion, NovelMetadataUpdateInput,
};

use crate::{services::llm_json::call_local_json_array, ApiError, AppState};

const NOVEL_METADATA_MAX_TOKENS: u32 = 1024;

pub(crate) async fn fill_source_language_if_auto(
    state: &AppState,
    project_id: &str,
    novel_id: &str,
    input: &mut NovelMetadataUpdateInput,
) -> Result<(), ApiError> {
    if input
        .source_language
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "auto")
    {
        return Ok(());
    }

    let chapters = state.store.list_chapters(project_id, novel_id).await?;
    let text = chapters
        .iter()
        .map(|chapter| chapter.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    input.source_language = detect_basic_source_language(&text);

    Ok(())
}

pub(crate) async fn ai_fill_novel_metadata(
    state: &AppState,
    project_id: &str,
    novel_id: &str,
) -> Result<Novel, ApiError> {
    let novel = state
        .store
        .get_novel(project_id, novel_id)
        .await?
        .ok_or(ApiError::not_found("novel"))?;
    let chapters = state.store.list_chapters(project_id, novel_id).await?;
    let text = chapters
        .iter()
        .map(|chapter| chapter.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.trim().is_empty() {
        return Err(ApiError::bad_request("novel text is required"));
    }

    let suggestion = suggest_novel_metadata(
        state,
        NovelImportInput {
            title: novel.title.clone(),
            author: novel.author.clone(),
            source_language: novel.source_language.clone(),
            genre: novel.genre.clone(),
            description: novel.description.clone(),
            text,
        },
    )
    .await?;

    Ok(state
        .store
        .update_novel_metadata(
            project_id,
            novel_id,
            merge_novel_metadata_suggestion(&novel, suggestion),
        )
        .await?)
}

pub(crate) async fn suggest_novel_metadata(
    state: &AppState,
    mut input: NovelImportInput,
) -> Result<NovelMetadataSuggestion, ApiError> {
    if input
        .source_language
        .as_deref()
        .map(str::trim)
        .is_none_or(|value| value.is_empty() || value == "auto")
    {
        input.source_language = detect_basic_source_language(&input.text);
    }

    let prompt = build_novel_metadata_suggestion_prompt(&input);
    let (suggestions, _) =
        call_local_json_array::<NovelMetadataSuggestion>(state, &prompt, NOVEL_METADATA_MAX_TOKENS)
            .await?;
    let mut suggestion = suggestions
        .into_iter()
        .next()
        .unwrap_or(NovelMetadataSuggestion {
            title: None,
            author: None,
            source_language: None,
            genre: None,
            description: None,
            confidence: Some(0.0),
        });
    if suggestion
        .source_language
        .as_deref()
        .is_none_or(str::is_empty)
    {
        suggestion.source_language = input.source_language;
    }

    Ok(normalize_novel_metadata_suggestion(suggestion))
}

fn normalize_novel_metadata_suggestion(
    mut suggestion: NovelMetadataSuggestion,
) -> NovelMetadataSuggestion {
    suggestion.title = optional_metadata_text(suggestion.title);
    suggestion.author = optional_metadata_text(suggestion.author);
    suggestion.source_language =
        optional_metadata_text(suggestion.source_language).filter(|value| value != "auto");
    suggestion.genre = optional_metadata_text(suggestion.genre);
    suggestion.description = optional_metadata_text(suggestion.description);
    suggestion
}

fn merge_novel_metadata_suggestion(
    novel: &Novel,
    suggestion: NovelMetadataSuggestion,
) -> NovelMetadataUpdateInput {
    NovelMetadataUpdateInput {
        title: suggestion.title.or_else(|| Some(novel.title.clone())),
        author: suggestion.author.or_else(|| novel.author.clone()),
        source_language: suggestion
            .source_language
            .or_else(|| novel.source_language.clone()),
        genre: suggestion.genre.or_else(|| novel.genre.clone()),
        description: suggestion.description.or_else(|| novel.description.clone()),
    }
}

fn optional_metadata_text(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
