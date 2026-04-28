use serde_json::json;

use crate::*;

pub(crate) async fn run_next_analysis_chapter(
    state: &AppState,
    project_id: &str,
    job_id: &str,
    input: AnalysisRunStepInput,
) -> Result<AnalysisRunSnapshot, ApiError> {
    let step_ready =
        match services::analysis_step::prepare_analysis_step(state, project_id, job_id, &input)
            .await?
        {
            services::analysis_step::AnalysisStepPreflight::Snapshot(snapshot) => {
                return Ok(snapshot);
            }
            services::analysis_step::AnalysisStepPreflight::Ready(ready) => ready,
        };
    let novel = step_ready.novel;
    let chapters = step_ready.chapters;
    let chapter = step_ready.chapter;
    let chapter_run = step_ready.chapter_run;
    let chapter_range = step_ready.chapter_range;

    let chunks = split_chapter_for_character_extraction(&chapter.content);
    let mut working_document = StoryExtractionDocument {
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION.to_string(),
        chapter_num: chapter.chapter_num,
        records: Vec::new(),
    };
    let mut chunk_outputs = Vec::with_capacity(chunks.len());
    let novel_analysis_context = build_current_novel_analysis_context(&novel);

    for (index, chunk) in chunks.iter().enumerate() {
        if services::analysis_step::analysis_job_should_stop(&state.store, project_id, job_id)
            .await?
        {
            return services::analysis::build_run_snapshot(&state.store, project_id, job_id, None)
                .await;
        }

        let base_prior_context = format!(
            "{novel_analysis_context}\n\nĐây là đoạn nhỏ {}/{} của chương hiện tại. Mỗi pass chỉ xử lý dữ liệu có trong đoạn này. Offset mention phải tính từ CHAPTER_TEXT của đoạn này; backend sẽ tự quy đổi về toàn chương.",
            index + 1,
            chunks.len()
        );
        let db_records_before_identity = state
            .store
            .list_story_extraction_records(project_id, job_id, "character")
            .await?;
        let db_aliases_before_identity = state
            .store
            .list_story_character_aliases(project_id, job_id)
            .await?;
        let known_alias_identity_hints =
            services::analysis_alias::known_alias_map_identities_for_chunk(
                &chunk.text,
                &db_aliases_before_identity,
            );
        let known_alias_context =
            serde_json::to_string(&known_alias_identity_hints).unwrap_or_else(|_| "[]".to_string());
        let candidate_input = DraftExtractionInput {
            chapter_num: chapter.chapter_num,
            title: Some(chapter.title.clone()),
            source_language: novel.source_language.clone(),
            text: chunk.text.clone(),
            prior_context: Some(format!(
                "{base_prior_context}\n\nKnown character/alias surfaces already present in this chunk from previous chapters:\n{known_alias_context}\n\nNếu một known surface xuất hiện trong CHAPTER_TEXT, ưu tiên giữ node canonical đã biết thay vì tạo node mới cho cùng người."
            )),
        };
        let candidate_prompt = build_character_candidate_prompt(&candidate_input);
        let (candidate_identities, candidate_response) =
            match services::llm_json::call_local_json_array::<CharacterCandidate>(
                state,
                &candidate_prompt,
                CHARACTER_CANDIDATE_MAX_TOKENS,
            )
            .await
            {
                Ok((candidates, response)) => {
                    (normalize_character_candidates(candidates), json!(response))
                }
                Err(error) => (
                    Vec::new(),
                    json!({
                        "mode": "candidate_pass_failed_non_blocking",
                        "error": error.message,
                    }),
                ),
            };
        let candidate_context =
            serde_json::to_string(&candidate_identities).unwrap_or_else(|_| "[]".to_string());
        let chunk_input = DraftExtractionInput {
            chapter_num: chapter.chapter_num,
            title: Some(chapter.title.clone()),
            source_language: novel.source_language.clone(),
            text: chunk.text.clone(),
            prior_context: Some(format!(
                "{base_prior_context}\n\nKnown character/alias surfaces already present in this chunk from previous chapters:\n{known_alias_context}\n\nCandidate coverage checklist từ pass quét nhanh trước identity:\n{candidate_context}\n\nIdentity pass phải dùng checklist này để tránh sót tên/alias có evidence trong đoạn, nhưng vẫn chỉ ghi nhân vật/alias khi CHAPTER_TEXT hiện tại hỗ trợ."
            )),
        };

        let identity_prompt = build_character_identity_prompt(&chunk_input);
        let (chunk_identities, identity_response) =
            match services::llm_json::call_local_json_array::<CharacterIdentity>(
                state,
                &identity_prompt,
                CHARACTER_IDENTITY_MAX_TOKENS,
            )
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    let reason = format!(
                        "character identity chunk {}/{} failed: {}",
                        index + 1,
                        chunks.len(),
                        error.message
                    );
                    return services::analysis_step::fail_analysis_chapter_and_pause(
                        state,
                        project_id,
                        job_id,
                        &chapter_run.chapter_id,
                        "character_identity_pass_failed",
                        reason,
                    )
                    .await;
                }
            };
        let mut chunk_identities = normalize_character_identities(chunk_identities);
        merge_character_identity_hints(&mut chunk_identities, candidate_identities.clone());
        merge_character_identity_hints(&mut chunk_identities, known_alias_identity_hints.clone());
        filter_substring_only_identities(&mut chunk_identities, &chunk.text);
        let identity_nodes_json =
            serde_json::to_string(&chunk_identities).unwrap_or_else(|_| "[]".to_string());
        let quoted_alias_candidates = services::analysis_alias::quoted_alias_candidate_context(
            &chunk_identities,
            &chunk.text,
        );
        let quoted_alias_context =
            serde_json::to_string(&quoted_alias_candidates).unwrap_or_else(|_| "[]".to_string());
        let alias_ownership_input = DraftExtractionInput {
            chapter_num: chapter.chapter_num,
            title: Some(chapter.title.clone()),
            source_language: novel.source_language.clone(),
            text: chunk.text.clone(),
            prior_context: Some(format!(
                "{base_prior_context}\n\nIdentity/Candidate pass đã tạo danh sách node nhân vật bên dưới. Alias Ownership pass là bước duy nhất được nhập surface alias/coreference vào owner trước khi resolver xuyên chương chạy.\n\nQuoted surface checklist do backend chỉ quét hình thức ngoặc kép, không tự kết luận owner:\n{quoted_alias_context}\n\nDùng checklist này để xét kỹ các surface trong ngoặc kép, nhưng chỉ trả alias ownership khi CHAPTER_TEXT chứng minh bằng ngữ pháp/ngữ nghĩa."
            )),
        };
        let alias_ownership_prompt =
            build_character_alias_ownership_prompt(&alias_ownership_input, &identity_nodes_json);
        let (alias_ownerships, alias_ownership_response) =
            match services::llm_json::call_local_json_array::<CharacterAliasOwnership>(
                state,
                &alias_ownership_prompt,
                CHARACTER_ALIAS_OWNERSHIP_MAX_TOKENS,
            )
            .await
            {
                Ok((ownerships, response)) => (ownerships, json!(response)),
                Err(error) => (
                    Vec::new(),
                    json!({
                        "mode": "alias_ownership_pass_failed_non_blocking",
                        "error": error.message,
                    }),
                ),
            };
        let alias_ownership_applications =
            services::analysis_alias::apply_character_alias_ownerships(
                &mut chunk_identities,
                alias_ownerships,
                chapter.chapter_num,
            );
        filter_substring_only_identities(&mut chunk_identities, &chunk.text);
        let (chunk_identities, merge_decision_outputs) =
            services::analysis_identity::resolve_character_identities_across_chapters(
                state,
                &chunk_input,
                chunk_identities,
                &db_records_before_identity,
                &db_aliases_before_identity,
                &working_document,
            )
            .await;
        services::analysis_document::merge_character_identity_records(
            &mut working_document,
            &chunk_identities,
        );
        services::analysis_document::normalize_character_field_keys(&mut working_document);
        state
            .store
            .replace_story_extraction_records_for_chapter(
                project_id,
                job_id,
                &chapter.id,
                CHARACTER_EXTRACTION_SCHEMA_VERSION,
                &working_document,
                "character_identity_chunk",
            )
            .await?;
        publish_project_event(
            state,
            project_id,
            "story_extraction_updated",
            Some(job_id),
            Some(&chapter.id),
            "character identity chunk persisted",
        );

        let db_records = state
            .store
            .list_story_extraction_records(project_id, job_id, "character")
            .await?;
        let current_identities = services::analysis_document::hydrate_identities_with_alias_map(
            services::analysis_document::working_identities_for_chunk(
                &working_document,
                &chunk_identities,
            ),
            &db_aliases_before_identity,
        );
        let mut character_passes = Vec::new();

        for identity in current_identities {
            if services::analysis_step::analysis_job_should_stop(&state.store, project_id, job_id)
                .await?
            {
                return services::analysis::build_run_snapshot(
                    &state.store,
                    project_id,
                    job_id,
                    None,
                )
                .await;
            }

            let character_json =
                serde_json::to_string(&identity).unwrap_or_else(|_| "{}".to_string());

            let (mentions, mentions_response) =
                match services::analysis_mentions::scan_character_mentions_with_backend(
                    state,
                    &chunk_input,
                    &identity,
                    &character_json,
                    &chapter.content,
                )
                .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        let reason = format!(
                            "character backend mention scan chunk {}/{} for {} failed: {}",
                            index + 1,
                            chunks.len(),
                            identity.name,
                            error.message
                        );
                        return services::analysis_step::fail_analysis_chapter_and_pause(
                            state,
                            project_id,
                            job_id,
                            &chapter_run.chapter_id,
                            "character_mentions_backend_scan_failed",
                            reason,
                        )
                        .await;
                    }
                };
            services::analysis_document::merge_character_identity_mentions(
                &mut working_document,
                &identity,
                mentions,
            );
            state
                .store
                .replace_story_extraction_records_for_chapter(
                    project_id,
                    job_id,
                    &chapter.id,
                    CHARACTER_EXTRACTION_SCHEMA_VERSION,
                    &working_document,
                    "character_mentions_chunk",
                )
                .await?;
            publish_project_event(
                state,
                project_id,
                "story_extraction_updated",
                Some(job_id),
                Some(&chapter.id),
                "character mentions chunk persisted",
            );

            let field_contexts =
                services::analysis_fields::build_character_field_contexts(&chunk.text, &identity);
            let mut field_input_for_verification: Option<DraftExtractionInput> = None;
            let (fields, fields_response) = if field_contexts.is_empty() {
                (
                    Vec::new(),
                    json!({
                        "mode": "skipped_no_target_context",
                        "target": identity.name,
                    }),
                )
            } else {
                let field_input = DraftExtractionInput {
                    chapter_num: chunk_input.chapter_num,
                    title: chunk_input.title.clone(),
                    source_language: chunk_input.source_language.clone(),
                    text: field_contexts.join("\n---\n"),
                    prior_context: Some(format!(
                        "{novel_analysis_context}\n\nĐây là TARGET_CONTEXTS đã được backend chọn từ đoạn nhỏ {}/{} của chương hiện tại. Các occurrence của target được đánh dấu bằng [[...]]. Fields pass chỉ được dùng các context này, không dùng toàn chunk.",
                        index + 1,
                        chunks.len()
                    )),
                };
                field_input_for_verification = Some(field_input.clone());
                let fields_prompt = build_character_fields_prompt(&field_input, &character_json);
                let (fields, response) = match services::llm_json::call_local_json_array::<
                    StoryExtractionFieldPayload,
                >(
                    state, &fields_prompt, CHARACTER_FIELDS_MAX_TOKENS
                )
                .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        let reason = format!(
                            "character fields chunk {}/{} for {} failed: {}",
                            index + 1,
                            chunks.len(),
                            identity.name,
                            error.message
                        );
                        return services::analysis_step::fail_analysis_chapter_and_pause(
                            state,
                            project_id,
                            job_id,
                            &chapter_run.chapter_id,
                            "character_fields_pass_failed",
                            reason,
                        )
                        .await;
                    }
                };
                (
                    fields,
                    json!({
                        "mode": "target_marked_contexts",
                        "context_count": field_contexts.len(),
                        "contexts": field_contexts,
                        "response": response,
                    }),
                )
            };
            let normalized_fields = services::analysis_fields::normalize_character_field_payloads(
                fields,
                &identity,
                &db_records,
                &working_document,
            );
            let (fields, field_verification_report) =
                services::analysis_fields::verify_character_field_payloads(
                    state,
                    field_input_for_verification.as_ref(),
                    &identity,
                    &character_json,
                    normalized_fields,
                )
                .await;
            services::analysis_document::merge_character_identity_fields(
                &mut working_document,
                &identity,
                fields,
            );
            services::analysis_document::normalize_character_field_keys(&mut working_document);
            state
                .store
                .replace_story_extraction_records_for_chapter(
                    project_id,
                    job_id,
                    &chapter.id,
                    CHARACTER_EXTRACTION_SCHEMA_VERSION,
                    &working_document,
                    "character_fields_chunk",
                )
                .await?;
            publish_project_event(
                state,
                project_id,
                "story_extraction_updated",
                Some(job_id),
                Some(&chapter.id),
                "character fields chunk persisted",
            );

            character_passes.push(json!({
                "name": identity.name,
                "aliases": identity.aliases,
                "mentions_response": mentions_response,
                "fields_response": fields_response,
                "field_verification": field_verification_report,
            }));
        }

        chunk_outputs.push(json!({
            "chunk_index": index + 1,
            "chunk_count": chunks.len(),
            "start_char": chunk.start_char,
            "end_char": chunk.end_char,
            "candidate_response": candidate_response,
            "candidate_identity_hints": candidate_identities,
            "known_alias_identity_hints": known_alias_identity_hints,
            "identity_response": identity_response,
            "alias_ownership_response": alias_ownership_response,
            "alias_ownership_applications": alias_ownership_applications,
            "merge_decisions": merge_decision_outputs,
            "character_passes": character_passes,
        }));

        if services::analysis_step::analysis_job_should_stop(&state.store, project_id, job_id)
            .await?
        {
            return services::analysis::build_run_snapshot(&state.store, project_id, job_id, None)
                .await;
        }
    }

    let relationship_outputs =
        match services::analysis_relationships::extract_character_relationships_for_chapter(
            state,
            project_id,
            job_id,
            &chapter,
            &novel,
            &mut working_document,
        )
        .await
        {
            Ok(outputs) => outputs,
            Err(error) => {
                let reason = format!("character relationship pass failed: {}", error.message);
                return services::analysis_step::fail_analysis_chapter_and_pause(
                    state,
                    project_id,
                    job_id,
                    &chapter_run.chapter_id,
                    "character_relationship_pass_failed",
                    reason,
                )
                .await;
            }
        };

    let mut character_extraction = working_document;
    services::analysis_document::normalize_character_field_keys(&mut character_extraction);
    if let Err(error) = services::analysis_document::validate_character_extraction_document(
        &character_extraction,
        chapter.chapter_num,
        &chapter.content,
    ) {
        let reason = format!(
            "character extraction merged result failed validation: {}",
            error.message
        );
        state
            .store
            .fail_analysis_chapter_run(
                project_id,
                job_id,
                &chapter_run.chapter_id,
                "character_extraction_validation_failed",
                &reason,
            )
            .await?;
        state
            .store
            .pause_analysis_job(
                project_id,
                job_id,
                &reason,
                Some("character_extraction_validation_failed"),
                true,
            )
            .await?;
        publish_project_event(
            state,
            project_id,
            "analysis_paused",
            Some(job_id),
            Some(&chapter.id),
            "character extraction validation failed",
        );

        return services::analysis::build_run_snapshot(
            &state.store,
            project_id,
            job_id,
            Some(reason),
        )
        .await;
    }

    let output_json = json!({
        "schema_version": CHARACTER_EXTRACTION_SCHEMA_VERSION,
        "extraction_mode": "staged_chunked_character_backend_mention_scan",
        "chunk_target_chars": CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS,
        "chunk_count": chunks.len(),
        "chunks": chunk_outputs,
        "relationship_passes": relationship_outputs,
        "persisted": true,
        "persisted_group_keys": ["character", "relationship"],
        "character_record_count": character_extraction
            .records
            .iter()
            .filter(|record| record.group_key == "character")
            .count(),
        "relationship_record_count": character_extraction
            .records
            .iter()
            .filter(|record| record.group_key == "relationship")
            .count(),
    })
    .to_string();
    state
        .store
        .complete_analysis_chapter_run_with_story_extraction(
            project_id,
            job_id,
            &chapter.id,
            CHARACTER_EXTRACTION_SCHEMA_VERSION,
            &output_json,
            &character_extraction,
        )
        .await?;
    publish_project_event(
        state,
        project_id,
        "analysis_chapter_completed",
        Some(job_id),
        Some(&chapter.id),
        "analysis chapter completed",
    );

    let runs = state
        .store
        .list_analysis_chapter_runs(project_id, job_id)
        .await?;
    services::analysis::finish_range_or_job(
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
        "analysis range or job progress updated",
    );

    services::analysis::build_run_snapshot(&state.store, project_id, job_id, None).await
}

#[derive(Debug, Clone)]
struct CharacterExtractionChunk {
    start_char: i64,
    end_char: i64,
    text: String,
}

fn split_chapter_for_character_extraction(text: &str) -> Vec<CharacterExtractionChunk> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return vec![CharacterExtractionChunk {
            start_char: 0,
            end_char: 0,
            text: String::new(),
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = choose_character_extraction_chunk_end(&chars, start);
        let chunk_text = chars[start..end].iter().collect::<String>();
        if !chunk_text.trim().is_empty() {
            chunks.push(CharacterExtractionChunk {
                start_char: start as i64,
                end_char: end as i64,
                text: chunk_text,
            });
        }
        start = end.max(start + 1);
    }

    if chunks.is_empty() {
        chunks.push(CharacterExtractionChunk {
            start_char: 0,
            end_char: chars.len() as i64,
            text: text.to_string(),
        });
    }

    chunks
}

fn choose_character_extraction_chunk_end(chars: &[char], start: usize) -> usize {
    let remaining = chars.len().saturating_sub(start);
    if remaining <= CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS {
        return chars.len();
    }

    let hard_end = (start + CHARACTER_EXTRACTION_CHUNK_TARGET_CHARS).min(chars.len());
    let min_end = (start + CHARACTER_EXTRACTION_CHUNK_MIN_CHARS).min(hard_end);

    for index in (min_end..hard_end).rev() {
        if chars[index] == '\n' && index + 1 < chars.len() && chars[index + 1] == '\n' {
            return index + 2;
        }
    }

    for index in (min_end..hard_end).rev() {
        if chars[index] == '\n' {
            return index + 1;
        }
    }

    for index in (min_end..hard_end).rev() {
        if matches!(chars[index], '.' | '!' | '?' | '。' | '！' | '？') {
            return index + 1;
        }
    }

    hard_end
}
