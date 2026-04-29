use std::collections::{HashMap, HashSet};

use novelgraph_core::{
    build_story_chapter_cloud_extraction_prompt, AnalysisExecutionProfile, Chapter,
    CloudChapterExtractionInput, StoryExtractionDocument, StoryExtractionFieldPayload,
    StoryExtractionFieldValuePayload, StoryExtractionFieldValueView, StoryExtractionRecordPayload,
    StoryExtractionRecordView, CHARACTER_EXTRACTION_SCHEMA_VERSION,
    CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::*;

const CLOUD_GEMINI_MAX_OUTPUT_TOKENS: u32 = 24_576;
const CLOUD_MIN_CONFIDENCE: f64 = 0.45;
const EMPTY_JSON_ARRAY_TEXT: &str = "[]";

const PROVIDER_GEMINI: &str = "gemini";
const GROUP_KEY_CHARACTER: &str = "character";
const GROUP_LABEL_CHARACTER: &str = "Nhân Vật";
const GROUP_KEY_RELATIONSHIP: &str = "relationship";
const GROUP_LABEL_RELATIONSHIP: &str = "Quan Hệ";
const FIELD_KEY_APPEARANCE: &str = "appearance";
const FIELD_LABEL_APPEARANCE: &str = "Ngoại hình";
const FIELD_KEY_OTHER_ALIAS: &str = "other_alias";
const FIELD_LABEL_OTHER_ALIAS: &str = "Tên gọi khác";
const FIELD_KEY_RELATIONSHIP: &str = "relationship";
const FIELD_LABEL_RELATIONSHIP: &str = "Quan hệ";
const RELATIONSHIP_DIRECTION_SELF_TO_RELATED: &str = "self_to_related";
const RELATIONSHIP_DIRECTION_RELATED_TO_SELF: &str = "related_to_self";
const EVIDENCE_REASON_CLOUD_FIELD: &str = "cloud_field";
const EVIDENCE_REASON_CLOUD_RELATIONSHIP: &str = "cloud_relationship";
const SUSPECT_MOVED_TO_REVIEW_STATUS: &str = "moved_to_review";
const PERSISTABLE_CHARACTER_FIELD_CLASSES: &[&str] = &[
    "physical_appearance",
    "clothing",
    "age_or_build",
    FIELD_KEY_APPEARANCE,
];
const PERSISTABLE_RELATIONSHIP_SCOPES: &[&str] =
    &["kinship", "organization_hierarchy", "stable_relationship"];
const KNOWN_RELATIONSHIP_CONTEXT_LIMIT: usize = 24;

#[derive(Debug, Clone, Serialize)]
struct KnownRelationshipHint {
    left_key: String,
    left_name: String,
    right_key: String,
    right_name: String,
    relationship_type: String,
    left_to_right_label: Option<String>,
    right_to_left_label: Option<String>,
    evidence_chapter_nums: Vec<i64>,
}

pub(crate) async fn run_cloud_gemini_one_shot(
    state: &AppState,
    project_id: &str,
    job_id: &str,
    execution_profile: AnalysisExecutionProfile,
    step_ready: services::analysis_step::AnalysisStepReady,
) -> Result<AnalysisRunSnapshot, ApiError> {
    let novel = step_ready.novel;
    let chapters = step_ready.chapters;
    let chapter = step_ready.chapter;
    let chapter_run = step_ready.chapter_run;
    let chapter_range = step_ready.chapter_range;

    if services::analysis_step::analysis_job_should_stop(&state.store, project_id, job_id).await? {
        return services::analysis::build_run_snapshot(&state.store, project_id, job_id, None)
            .await;
    }

    let runtime_provider =
        match services::byok::load_runtime_provider_config(state, PROVIDER_GEMINI).await {
            Ok(config) => config,
            Err(error) => {
                let reason = format!("cloud provider config error: {}", error.message);
                return services::analysis_step::fail_analysis_chapter_and_pause(
                    state,
                    project_id,
                    job_id,
                    &chapter.id,
                    "cloud_provider_config_error",
                    reason,
                )
                .await;
            }
        };
    let db_aliases = state
        .store
        .list_story_character_aliases(project_id, job_id)
        .await?;
    let prior_db_aliases = db_aliases
        .iter()
        .filter(|alias| alias.first_chapter_num < chapter.chapter_num)
        .cloned()
        .collect::<Vec<_>>();
    let known_alias_hints = services::analysis_alias::known_alias_map_identities_for_chunk(
        &chapter.content,
        &prior_db_aliases,
    );
    let known_alias_context = serde_json::to_string(&known_alias_hints)
        .unwrap_or_else(|_| EMPTY_JSON_ARRAY_TEXT.to_string());
    let prior_relationship_records = state
        .store
        .list_story_extraction_records(project_id, job_id, GROUP_KEY_RELATIONSHIP)
        .await?;
    let known_relationship_hints = build_known_relationship_context(
        &prior_relationship_records,
        &known_alias_hints,
        chapter.chapter_num,
    );
    let known_relationship_context = serde_json::to_string(&known_relationship_hints)
        .unwrap_or_else(|_| EMPTY_JSON_ARRAY_TEXT.to_string());
    let input = CloudChapterExtractionInput {
        chapter_num: chapter.chapter_num,
        title: Some(chapter.title.clone()),
        source_language: novel.source_language.clone(),
        chapter_text: chapter.content.clone(),
        novel_context: Some(build_current_novel_analysis_context(&novel)),
        known_alias_surfaces_json: Some(known_alias_context),
        known_relationships_json: Some(known_relationship_context),
    };
    let prompt = build_story_chapter_cloud_extraction_prompt(&input);
    let trace_id = format!(
        "analysis:{}:{}:chapter:{}:attempt:{}",
        project_id, job_id, chapter.chapter_num, chapter_run.attempt
    );
    let structured =
        match services::llm_structured::call_gemini_structured::<CloudExtractionDocument>(
            state,
            &prompt,
            &runtime_provider.model,
            &runtime_provider.api_key,
            &trace_id,
            CLOUD_GEMINI_MAX_OUTPUT_TOKENS,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                let reason = format!("cloud structured extraction call failed: {}", error.message);
                return services::analysis_step::fail_analysis_chapter_and_pause(
                    state,
                    project_id,
                    job_id,
                    &chapter.id,
                    "cloud_structured_call_failed",
                    reason,
                )
                .await;
            }
        };

    if structured.value.schema_version.trim() != CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION {
        let reason = format!(
            "cloud schema mismatch: expected {}, got {}",
            CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION, structured.value.schema_version
        );
        return services::analysis_step::fail_analysis_chapter_and_pause(
            state,
            project_id,
            job_id,
            &chapter.id,
            "cloud_schema_mismatch",
            reason,
        )
        .await;
    }
    if structured.value.chapter_num != chapter.chapter_num {
        let reason = format!(
            "cloud chapter mismatch: expected {}, got {}",
            chapter.chapter_num, structured.value.chapter_num
        );
        return services::analysis_step::fail_analysis_chapter_and_pause(
            state,
            project_id,
            job_id,
            &chapter.id,
            "cloud_chapter_num_mismatch",
            reason,
        )
        .await;
    }

    let mut suspect_items = Vec::<serde_json::Value>::new();
    let character_records =
        build_character_records(&chapter, &structured.value.characters, &mut suspect_items);
    let relationship_records = build_relationship_records(
        &chapter,
        &character_records,
        &structured.value.relationships,
        &mut suspect_items,
    );
    let mut records = character_records;
    records.extend(relationship_records);
    let mut extraction = StoryExtractionDocument {
        schema_version: CHARACTER_EXTRACTION_SCHEMA_VERSION.to_string(),
        chapter_num: chapter.chapter_num,
        records,
    };
    services::analysis_document::normalize_character_field_keys(&mut extraction);
    if let Err(error) = services::analysis_document::validate_character_extraction_document(
        &extraction,
        chapter.chapter_num,
        &chapter.content,
    ) {
        let reason = format!("cloud extraction validation failed: {}", error.message);
        return services::analysis_step::fail_analysis_chapter_and_pause(
            state,
            project_id,
            job_id,
            &chapter.id,
            "cloud_extraction_validation_failed",
            reason,
        )
        .await;
    }

    let call_status = if !suspect_items.is_empty() {
        SUSPECT_MOVED_TO_REVIEW_STATUS.to_string()
    } else {
        structured.telemetry.call_status.clone()
    };
    let model_call_profile = structured
        .value
        .call_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(prompt.call_profile);
    let output_json = json!({
        "schema_version": CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION,
        "prompt_template": {
            "id": prompt.template_id,
            "version": prompt.template_version,
        },
        "execution_profile": execution_profile.as_str(),
        "call_profile": model_call_profile,
        "call_summary": {
            "status": call_status,
            "api_call_count": structured.telemetry.api_call_count,
            "provider": structured.telemetry.provider,
            "model": structured.telemetry.model,
            "input_tokens": structured.telemetry.input_tokens,
            "output_tokens": structured.telemetry.output_tokens,
            "estimated_cost": structured.telemetry.estimated_cost,
            "trace_id": structured.telemetry.trace_id,
        },
        "review_item_count": structured.value.review_items.len(),
        "suspect_count": suspect_items.len(),
        "suspect_items": suspect_items,
        "raw_response_preview": structured.telemetry.raw_response_preview,
        "known_alias_hints_count": known_alias_hints.len(),
        "known_relationship_hints_count": known_relationship_hints.len(),
        "provider_config": {
            "provider": runtime_provider.provider,
            "api_format": runtime_provider.api_format,
            "base_url": runtime_provider.base_url,
        },
        "persisted_group_keys": [GROUP_KEY_CHARACTER, GROUP_KEY_RELATIONSHIP],
        "character_record_count": extraction.records.iter().filter(|record| record.group_key == GROUP_KEY_CHARACTER).count(),
        "relationship_record_count": extraction.records.iter().filter(|record| record.group_key == GROUP_KEY_RELATIONSHIP).count(),
    })
    .to_string();

    state
        .store
        .complete_analysis_chapter_run_with_story_extraction(
            project_id,
            job_id,
            &chapter.id,
            CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION,
            &output_json,
            &extraction,
        )
        .await?;
    publish_project_event(
        state,
        project_id,
        "analysis_chapter_completed",
        Some(job_id),
        Some(&chapter.id),
        "analysis chapter completed (cloud Gemini one-shot)",
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

    services::analysis::build_run_snapshot(&state.store, project_id, job_id, None).await
}

fn build_character_records(
    chapter: &Chapter,
    characters: &[CloudCharacter],
    suspect_items: &mut Vec<serde_json::Value>,
) -> Vec<StoryExtractionRecordPayload> {
    let mut output = Vec::new();
    let mut records_by_key = HashMap::<String, usize>::new();

    for candidate in characters {
        let confidence = candidate.confidence.unwrap_or(0.0);
        if confidence < CLOUD_MIN_CONFIDENCE {
            continue;
        }

        let entity_nature = normalize_cloud_schema_key(&candidate.entity_nature);
        if matches!(
            entity_nature.as_str(),
            "temporary_reference" | "role_title" | "uncertain"
        ) {
            suspect_items.push(json!({
                "reason": "character_entity_nature_not_persistable",
                "display_name": candidate.display_name,
                "entity_nature": entity_nature,
            }));
            continue;
        }

        let name = clean_character_surface(&candidate.display_name);
        if name.is_empty() {
            continue;
        }

        let mut identity = CharacterIdentity {
            name: name.clone(),
            aliases: Vec::new(),
        };
        for alias_text in &candidate.aliases {
            let alias = clean_character_surface(alias_text);
            if alias.is_empty() {
                continue;
            }

            push_character_alias_if_valid(
                &mut identity.aliases,
                CharacterAlias {
                    text: alias,
                    alias_type: FIELD_KEY_OTHER_ALIAS.to_string(),
                    alias_label: FIELD_LABEL_OTHER_ALIAS.to_string(),
                    is_primary: false,
                    evidence: Vec::new(),
                },
                &identity.name,
            );
        }

        let mentions = scan_identity_mentions(chapter, &identity);
        if mentions.is_empty() && !candidate.evidence_quotes.is_empty() {
            suspect_items.push(json!({
                "reason": "character_mentions_not_found_from_surfaces",
                "display_name": candidate.display_name,
                "evidence_quotes": candidate.evidence_quotes,
            }));
        }
        let mut fields = build_character_fields(chapter, candidate, suspect_items);
        let alias_values = identity
            .aliases
            .iter()
            .filter_map(|alias| {
                let value = alias.text.trim();
                if value.is_empty() {
                    return None;
                }

                Some(StoryExtractionFieldValuePayload {
                    value: value.to_string(),
                    confidence: Some(1.0),
                    related_character: None,
                    relationship_type: None,
                    relationship_label: None,
                    relationship_direction: None,
                    evidence: Vec::new(),
                })
            })
            .collect::<Vec<_>>();
        if !alias_values.is_empty() {
            fields.push(StoryExtractionFieldPayload {
                field_key: FIELD_KEY_OTHER_ALIAS.to_string(),
                field_label: FIELD_LABEL_OTHER_ALIAS.to_string(),
                values: alias_values,
            });
        }
        let key = normalize_ascii_snake_key(&identity.name);
        let record = StoryExtractionRecordPayload {
            group_key: GROUP_KEY_CHARACTER.to_string(),
            group_label: GROUP_LABEL_CHARACTER.to_string(),
            entity_key: Some(key.clone()),
            display_name: identity.name.clone(),
            mentions,
            fields,
        };
        if let Some(existing_index) = records_by_key.get(&key).copied() {
            merge_record_mentions(&mut output[existing_index], record.mentions);
            merge_record_fields(&mut output[existing_index], record.fields);
        } else {
            records_by_key.insert(key, output.len());
            output.push(record);
        }
    }

    merge_probable_alias_character_records(&mut output, suspect_items);

    output
}

fn merge_probable_alias_character_records(
    records: &mut Vec<StoryExtractionRecordPayload>,
    suspect_items: &mut Vec<serde_json::Value>,
) {
    let mut source_index = 0usize;
    while source_index < records.len() {
        if !is_low_signal_alias_candidate_record(&records[source_index]) {
            source_index += 1;
            continue;
        }

        let Some((target_index, match_kind)) =
            probable_alias_record_merge_target(records, source_index)
        else {
            source_index += 1;
            continue;
        };

        let source_display_name = records[source_index].display_name.clone();
        let target_display_name = records[target_index].display_name.clone();
        let source_record = records.remove(source_index);
        let adjusted_target_index = if target_index > source_index {
            target_index - 1
        } else {
            target_index
        };
        merge_record_mentions(&mut records[adjusted_target_index], source_record.mentions);
        merge_record_fields(&mut records[adjusted_target_index], source_record.fields);
        suspect_items.push(json!({
            "reason": "probable_alias_character_record_merged",
            "source_display_name": source_display_name,
            "target_display_name": target_display_name,
            "match_kind": match_kind,
        }));
    }
}

fn is_low_signal_alias_candidate_record(record: &StoryExtractionRecordPayload) -> bool {
    record.group_key == GROUP_KEY_CHARACTER
        && record.mentions.len() <= 2
        && non_alias_field_value_count(record) == 0
}

fn probable_alias_record_merge_target(
    records: &[StoryExtractionRecordPayload],
    source_index: usize,
) -> Option<(usize, &'static str)> {
    let source = &records[source_index];
    let source_key = normalized_folded_text_key(&source.display_name);
    if source_key.is_empty() {
        return None;
    }

    let mut best: Option<(usize, &'static str, usize)> = None;
    for (target_index, target) in records.iter().enumerate() {
        if target_index == source_index
            || target.group_key != GROUP_KEY_CHARACTER
            || !target_record_has_stronger_identity_signal(target, source)
        {
            continue;
        }

        for target_key in character_record_surface_keys(target) {
            if target_key != source_key {
                continue;
            }

            match best {
                Some((_, _, best_score)) if best_score >= 100 => {}
                _ => best = Some((target_index, "exact_alias_surface", 100usize)),
            }
        }
        for target_alias_key in character_record_alias_surface_keys(target) {
            if !folded_surface_keys_are_near_alias(&source_key, &target_alias_key) {
                continue;
            }

            match best {
                Some((_, _, best_score)) if best_score >= 90 => {}
                _ => best = Some((target_index, "near_alias_surface", 90usize)),
            }
        }
    }

    best.map(|(target_index, match_kind, _)| (target_index, match_kind))
}

fn target_record_has_stronger_identity_signal(
    target: &StoryExtractionRecordPayload,
    source: &StoryExtractionRecordPayload,
) -> bool {
    target.mentions.len() > source.mentions.len()
        || non_alias_field_value_count(target) > 0
        || !services::analysis_document::aliases_from_payload_record(target).is_empty()
}

fn non_alias_field_value_count(record: &StoryExtractionRecordPayload) -> usize {
    record
        .fields
        .iter()
        .filter(|field| normalize_ascii_snake_key(&field.field_key) != FIELD_KEY_OTHER_ALIAS)
        .map(|field| field.values.len())
        .sum()
}

fn character_record_surface_keys(record: &StoryExtractionRecordPayload) -> Vec<String> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();
    push_character_record_surface_key(&mut keys, &mut seen, &record.display_name);
    for alias in services::analysis_document::aliases_from_payload_record(record) {
        push_character_record_surface_key(&mut keys, &mut seen, &alias.text);
    }

    keys
}

fn character_record_alias_surface_keys(record: &StoryExtractionRecordPayload) -> Vec<String> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();
    for alias in services::analysis_document::aliases_from_payload_record(record) {
        push_character_record_surface_key(&mut keys, &mut seen, &alias.text);
    }

    keys
}

fn push_character_record_surface_key(
    keys: &mut Vec<String>,
    seen: &mut HashSet<String>,
    surface: &str,
) {
    let key = normalized_folded_text_key(surface);
    if !key.is_empty() && seen.insert(key.clone()) {
        keys.push(key);
    }
}

fn folded_surface_keys_are_near_alias(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if !surface_key_is_near_alias_candidate(left) || !surface_key_is_near_alias_candidate(right) {
        return false;
    }

    one_edit_distance_or_less(left, right)
}

fn surface_key_is_near_alias_candidate(key: &str) -> bool {
    key.chars().count() >= 6 && key.split('_').filter(|token| !token.is_empty()).count() >= 2
}

fn one_edit_distance_or_less(left: &str, right: &str) -> bool {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let left_len = left_chars.len();
    let right_len = right_chars.len();
    if left_len.abs_diff(right_len) > 1 {
        return false;
    }

    if left_len == right_len {
        return left_chars
            .iter()
            .zip(right_chars.iter())
            .filter(|(left, right)| left != right)
            .count()
            <= 1;
    }

    let (shorter, longer) = if left_len < right_len {
        (&left_chars, &right_chars)
    } else {
        (&right_chars, &left_chars)
    };
    let mut short_index = 0usize;
    let mut long_index = 0usize;
    let mut edits = 0usize;
    while short_index < shorter.len() && long_index < longer.len() {
        if shorter[short_index] == longer[long_index] {
            short_index += 1;
            long_index += 1;
            continue;
        }

        edits += 1;
        if edits > 1 {
            return false;
        }
        long_index += 1;
    }

    true
}

fn build_character_fields(
    chapter: &Chapter,
    character: &CloudCharacter,
    suspect_items: &mut Vec<serde_json::Value>,
) -> Vec<StoryExtractionFieldPayload> {
    let mut values = Vec::new();
    for field in &character.fields {
        if field.value.trim().is_empty() {
            continue;
        }

        let field_key = normalize_cloud_schema_key(&field.field_key);
        let semantic_class = normalize_cloud_schema_key(&field.semantic_class);
        if !field_is_mvp_appearance(&field_key, &semantic_class) {
            suspect_items.push(json!({
                "reason": "field_semantic_class_not_persistable",
                "display_name": character.display_name,
                "field_key": field_key,
                "semantic_class": semantic_class,
                "field_label": field.field_label,
                "value": field.value,
            }));
            continue;
        }
        let evidence =
            map_quotes_to_evidence(chapter, &field.evidence_quotes, EVIDENCE_REASON_CLOUD_FIELD);
        if evidence.is_empty() {
            suspect_items.push(json!({
                "reason": "field_evidence_not_found_in_chapter",
                "display_name": character.display_name,
                "value": field.value,
                "semantic_class": semantic_class,
            }));
            continue;
        }

        values.push(StoryExtractionFieldValuePayload {
            value: field.value.trim().to_string(),
            confidence: field.confidence,
            related_character: None,
            relationship_type: None,
            relationship_label: None,
            relationship_direction: None,
            evidence,
        });
    }

    if values.is_empty() {
        return Vec::new();
    }

    vec![StoryExtractionFieldPayload {
        field_key: FIELD_KEY_APPEARANCE.to_string(),
        field_label: FIELD_LABEL_APPEARANCE.to_string(),
        values,
    }]
}

fn build_relationship_records(
    chapter: &Chapter,
    character_records: &[StoryExtractionRecordPayload],
    relationships: &[CloudRelationship],
    suspect_items: &mut Vec<serde_json::Value>,
) -> Vec<StoryExtractionRecordPayload> {
    let identities = character_records
        .iter()
        .map(|record| CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_payload_record(record),
        })
        .collect::<Vec<_>>();
    let surface_map = build_surface_map(&identities);
    let mut records_by_pair = HashMap::<String, usize>::new();
    let mut output = Vec::<StoryExtractionRecordPayload>::new();

    for relationship in relationships {
        let confidence = relationship.confidence.unwrap_or(0.0);
        if confidence < CLOUD_MIN_CONFIDENCE {
            continue;
        }

        let scope = normalize_cloud_schema_key(&relationship.relationship_scope);
        if !relationship_scope_is_persistable(&scope) {
            suspect_items.push(json!({
                "reason": "relationship_scope_not_persistable",
                "source_name": relationship.source_name,
                "target_name": relationship.target_name,
                "relationship_scope": scope,
            }));
            continue;
        }

        let Some(source_index) =
            resolve_surface_to_identity(&surface_map, &relationship.source_name)
        else {
            suspect_items.push(json!({
                "reason": "relationship_source_not_resolved_unique",
                "source_name": relationship.source_name,
            }));
            continue;
        };
        let Some(target_index) =
            resolve_surface_to_identity(&surface_map, &relationship.target_name)
        else {
            suspect_items.push(json!({
                "reason": "relationship_target_not_resolved_unique",
                "target_name": relationship.target_name,
            }));
            continue;
        };
        if source_index == target_index {
            continue;
        }

        let source = &identities[source_index];
        let target = &identities[target_index];
        let source_label = relationship.source_to_target_label.trim();
        let target_label = relationship.target_to_source_label.trim();
        if source_label.is_empty() || target_label.is_empty() {
            continue;
        }
        let relationship_type = normalize_cloud_schema_key(&relationship.relationship_type);
        if relationship_type.is_empty() {
            continue;
        }
        let evidence = map_quotes_to_evidence(
            chapter,
            &relationship.evidence_quotes,
            EVIDENCE_REASON_CLOUD_RELATIONSHIP,
        );
        if evidence.is_empty() {
            suspect_items.push(json!({
                "reason": "relationship_evidence_not_found_in_chapter",
                "source_name": source.name,
                "target_name": target.name,
                "relationship_type": relationship_type,
            }));
            continue;
        }

        let pair = if normalized_text_key(&source.name) <= normalized_text_key(&target.name) {
            (source, target, source_label, target_label)
        } else {
            (target, source, target_label, source_label)
        };
        let pair_key = format!(
            "relationship|{}|{}",
            normalize_ascii_snake_key(&pair.0.name),
            normalize_ascii_snake_key(&pair.1.name)
        );
        let values = vec![
            StoryExtractionFieldValuePayload {
                value: pair.2.to_string(),
                confidence: relationship.confidence,
                related_character: Some(pair.1.name.clone()),
                relationship_type: Some(relationship_type.clone()),
                relationship_label: Some(pair.2.to_string()),
                relationship_direction: Some(RELATIONSHIP_DIRECTION_SELF_TO_RELATED.to_string()),
                evidence: evidence.clone(),
            },
            StoryExtractionFieldValuePayload {
                value: pair.3.to_string(),
                confidence: relationship.confidence,
                related_character: Some(pair.0.name.clone()),
                relationship_type: Some(relationship_type.clone()),
                relationship_label: Some(pair.3.to_string()),
                relationship_direction: Some(RELATIONSHIP_DIRECTION_RELATED_TO_SELF.to_string()),
                evidence,
            },
        ];
        if let Some(index) = records_by_pair.get(&pair_key).copied() {
            if let Some(field) = output[index]
                .fields
                .iter_mut()
                .find(|field| normalize_ascii_snake_key(&field.field_key) == FIELD_KEY_RELATIONSHIP)
            {
                merge_relationship_field_values(field, values);
            }
            continue;
        }

        records_by_pair.insert(pair_key.clone(), output.len());
        output.push(StoryExtractionRecordPayload {
            group_key: GROUP_KEY_RELATIONSHIP.to_string(),
            group_label: GROUP_LABEL_RELATIONSHIP.to_string(),
            entity_key: Some(pair_key),
            display_name: format!("{} ↔ {}", pair.0.name, pair.1.name),
            mentions: Vec::new(),
            fields: vec![StoryExtractionFieldPayload {
                field_key: FIELD_KEY_RELATIONSHIP.to_string(),
                field_label: FIELD_LABEL_RELATIONSHIP.to_string(),
                values,
            }],
        });
    }

    output
}

fn build_known_relationship_context(
    records: &[StoryExtractionRecordView],
    current_identities: &[CharacterIdentity],
    current_chapter_num: i64,
) -> Vec<KnownRelationshipHint> {
    let current_identity_keys = current_chapter_identity_keys(current_identities);
    if current_identity_keys.is_empty() {
        return Vec::new();
    }

    let mut hints = Vec::<KnownRelationshipHint>::new();
    for record in records
        .iter()
        .filter(|record| record.chapter_num < current_chapter_num)
    {
        let Some((left_key, right_key)) = relationship_record_pair_keys(record) else {
            continue;
        };
        if !current_identity_keys.contains(&left_key) || !current_identity_keys.contains(&right_key)
        {
            continue;
        }

        let (left_name, right_name) =
            relationship_record_display_pair(record, &left_key, &right_key);
        let mut relationship_type = String::new();
        let mut left_to_right_label = None::<String>;
        let mut right_to_left_label = None::<String>;
        let mut evidence_chapter_nums = Vec::<i64>::new();

        for field in &record.fields {
            if normalize_ascii_snake_key(&field.field_key) != FIELD_KEY_RELATIONSHIP {
                continue;
            }

            for value in &field.values {
                let candidate_type = value
                    .relationship_type
                    .as_deref()
                    .map(normalize_cloud_schema_key)
                    .unwrap_or_default();
                if relationship_type_prefer_candidate(&relationship_type, &candidate_type) {
                    relationship_type = candidate_type;
                }

                let label = relationship_value_label_view(value);
                if !label.is_empty() {
                    match relationship_value_pair_direction(
                        value,
                        &left_key,
                        &left_name,
                        &right_key,
                        &right_name,
                    ) {
                        Some(RelationshipPairDirection::LeftToRight) => {
                            update_preferred_relationship_label(&mut left_to_right_label, &label);
                        }
                        Some(RelationshipPairDirection::RightToLeft) => {
                            update_preferred_relationship_label(&mut right_to_left_label, &label);
                        }
                        None => {}
                    }
                }

                for evidence in value
                    .evidence
                    .iter()
                    .filter(|evidence| evidence.chapter_num < current_chapter_num)
                {
                    evidence_chapter_nums.push(evidence.chapter_num);
                }
            }
        }

        if relationship_type.is_empty() {
            continue;
        }
        if evidence_chapter_nums.is_empty() {
            evidence_chapter_nums.push(record.chapter_num);
        }
        evidence_chapter_nums.sort_unstable();
        evidence_chapter_nums.dedup();

        push_known_relationship_hint(
            &mut hints,
            KnownRelationshipHint {
                left_key,
                left_name,
                right_key,
                right_name,
                relationship_type,
                left_to_right_label,
                right_to_left_label,
                evidence_chapter_nums,
            },
        );
        if hints.len() >= KNOWN_RELATIONSHIP_CONTEXT_LIMIT {
            break;
        }
    }

    hints
}

fn current_chapter_identity_keys(identities: &[CharacterIdentity]) -> HashSet<String> {
    let mut keys = HashSet::new();
    for identity in identities {
        push_identity_context_key(&mut keys, &identity.name);
        for alias in &identity.aliases {
            push_identity_context_key(&mut keys, &alias.text);
        }
    }

    keys
}

fn push_identity_context_key(keys: &mut HashSet<String>, value: &str) {
    let key = normalize_ascii_snake_key(value);
    if !key.is_empty() {
        keys.insert(key);
    }
}

fn relationship_record_pair_keys(record: &StoryExtractionRecordView) -> Option<(String, String)> {
    let parts = record.entity_key.as_deref()?.split('|').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != GROUP_KEY_RELATIONSHIP {
        return None;
    }

    let left_key = normalize_ascii_snake_key(parts[1]);
    let right_key = normalize_ascii_snake_key(parts[2]);
    if left_key.is_empty() || right_key.is_empty() {
        return None;
    }

    Some((left_key, right_key))
}

fn relationship_record_display_pair(
    record: &StoryExtractionRecordView,
    left_key: &str,
    right_key: &str,
) -> (String, String) {
    let parts = record
        .display_name
        .split('↔')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() == 2 {
        return (parts[0].to_string(), parts[1].to_string());
    }

    (left_key.replace('_', " "), right_key.replace('_', " "))
}

fn relationship_value_label_view(value: &StoryExtractionFieldValueView) -> String {
    value
        .relationship_label
        .as_deref()
        .unwrap_or(&value.value)
        .trim()
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationshipPairDirection {
    LeftToRight,
    RightToLeft,
}

fn relationship_value_pair_direction(
    value: &StoryExtractionFieldValueView,
    left_key: &str,
    left_name: &str,
    right_key: &str,
    right_name: &str,
) -> Option<RelationshipPairDirection> {
    match normalize_ascii_snake_key(value.relationship_direction.as_deref().unwrap_or("")).as_str()
    {
        RELATIONSHIP_DIRECTION_SELF_TO_RELATED => Some(RelationshipPairDirection::LeftToRight),
        RELATIONSHIP_DIRECTION_RELATED_TO_SELF => Some(RelationshipPairDirection::RightToLeft),
        _ => {
            let related_key =
                normalize_ascii_snake_key(value.related_character.as_deref().unwrap_or(""));
            if related_key.is_empty() {
                return None;
            }
            if related_key == normalize_ascii_snake_key(right_name) || related_key == right_key {
                Some(RelationshipPairDirection::LeftToRight)
            } else if related_key == normalize_ascii_snake_key(left_name) || related_key == left_key
            {
                Some(RelationshipPairDirection::RightToLeft)
            } else {
                None
            }
        }
    }
}

fn push_known_relationship_hint(
    hints: &mut Vec<KnownRelationshipHint>,
    hint: KnownRelationshipHint,
) {
    if let Some(existing) = hints
        .iter_mut()
        .find(|existing| known_relationship_hints_match(existing, &hint))
    {
        merge_known_relationship_hint(existing, hint);
        return;
    }

    hints.push(hint);
}

fn known_relationship_hints_match(
    left: &KnownRelationshipHint,
    right: &KnownRelationshipHint,
) -> bool {
    left.left_key == right.left_key
        && left.right_key == right.right_key
        && (left.relationship_type == right.relationship_type
            || optional_relationship_labels_overlap(
                left.left_to_right_label.as_deref(),
                right.left_to_right_label.as_deref(),
            )
            || optional_relationship_labels_overlap(
                left.right_to_left_label.as_deref(),
                right.right_to_left_label.as_deref(),
            ))
}

fn merge_known_relationship_hint(
    target: &mut KnownRelationshipHint,
    source: KnownRelationshipHint,
) {
    if relationship_type_prefer_candidate(&target.relationship_type, &source.relationship_type) {
        target.relationship_type = source.relationship_type;
    }
    if let Some(label) = source.left_to_right_label {
        update_preferred_relationship_label(&mut target.left_to_right_label, &label);
    }
    if let Some(label) = source.right_to_left_label {
        update_preferred_relationship_label(&mut target.right_to_left_label, &label);
    }
    target
        .evidence_chapter_nums
        .extend(source.evidence_chapter_nums);
    target.evidence_chapter_nums.sort_unstable();
    target.evidence_chapter_nums.dedup();
}

fn build_surface_map(identities: &[CharacterIdentity]) -> HashMap<String, Option<usize>> {
    let mut map = HashMap::<String, Option<usize>>::new();
    for (index, identity) in identities.iter().enumerate() {
        for (surface, _) in character_identity_surfaces(identity) {
            let key = normalized_text_key(&surface);
            if key.is_empty() {
                continue;
            }

            match map.get(&key).copied().flatten() {
                Some(existing) if existing != index => {
                    map.insert(key, None);
                }
                None => {
                    map.entry(key).or_insert(Some(index));
                }
                _ => {}
            }
        }
    }

    map
}

fn resolve_surface_to_identity(
    surface_map: &HashMap<String, Option<usize>>,
    surface: &str,
) -> Option<usize> {
    let key = normalized_text_key(surface);
    if key.is_empty() {
        return None;
    }

    surface_map.get(&key).copied().flatten()
}

fn scan_identity_mentions(
    chapter: &Chapter,
    identity: &CharacterIdentity,
) -> Vec<StoryCharacterMention> {
    let mut scanned = Vec::new();
    for (surface, mention_type) in character_identity_surfaces(identity) {
        let ambiguous = is_ambiguous_character_surface(&surface);
        scanned.extend(find_surface_occurrences(
            &chapter.content,
            &surface,
            &mention_type,
            ambiguous,
        ));
    }

    select_non_overlapping_occurrences(scanned)
        .into_iter()
        .map(|occurrence| StoryCharacterMention {
            text: occurrence.text,
            start_char: occurrence.start_char,
            end_char: occurrence.end_char,
            mention_type: Some(occurrence.mention_type),
        })
        .collect()
}

fn map_quotes_to_evidence(
    chapter: &Chapter,
    quotes: &[String],
    reason: &str,
) -> Vec<StoryEvidenceSpan> {
    let mut evidence = Vec::new();
    let mut seen = HashSet::<String>::new();
    for quote in quotes {
        let quote = quote.trim();
        if quote.is_empty() || !seen.insert(normalized_text_key(quote)) {
            continue;
        }
        if let Some((start_char, end_char)) = find_quote_offsets(&chapter.content, quote) {
            evidence.push(StoryEvidenceSpan {
                chapter_num: chapter.chapter_num,
                start_char: Some(start_char),
                end_char: Some(end_char),
                quote: Some(quote.to_string()),
                reason: Some(reason.to_string()),
            });
        }
    }

    evidence
}

fn find_quote_offsets(chapter_text: &str, quote: &str) -> Option<(i64, i64)> {
    let byte_start = chapter_text.find(quote)?;
    let byte_end = byte_start + quote.len();
    let start_char = chapter_text[..byte_start].chars().count() as i64;
    let end_char = start_char + chapter_text[byte_start..byte_end].chars().count() as i64;
    Some((start_char, end_char))
}

fn normalize_cloud_schema_key(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    normalize_ascii_snake_key(value)
}

fn field_is_mvp_appearance(field_key: &str, semantic_class: &str) -> bool {
    if semantic_class.is_empty() {
        return field_semantic_class_is_persistable(field_key) || field_key == FIELD_KEY_APPEARANCE;
    }

    field_semantic_class_is_persistable(semantic_class)
}

fn field_semantic_class_is_persistable(semantic_class: &str) -> bool {
    PERSISTABLE_CHARACTER_FIELD_CLASSES.contains(&semantic_class)
}

fn relationship_scope_is_persistable(scope: &str) -> bool {
    PERSISTABLE_RELATIONSHIP_SCOPES.contains(&scope)
}

fn merge_relationship_field_values(
    field: &mut StoryExtractionFieldPayload,
    values: Vec<StoryExtractionFieldValuePayload>,
) {
    for value in values {
        if let Some(existing) = field
            .values
            .iter_mut()
            .find(|existing| relationship_values_refer_to_same_edge(existing, &value))
        {
            merge_relationship_field_value(existing, value);
        } else {
            field.values.push(value);
        }
    }
}

fn relationship_values_refer_to_same_edge(
    left: &StoryExtractionFieldValuePayload,
    right: &StoryExtractionFieldValuePayload,
) -> bool {
    normalize_ascii_snake_key(left.related_character.as_deref().unwrap_or(""))
        == normalize_ascii_snake_key(right.related_character.as_deref().unwrap_or(""))
        && normalize_ascii_snake_key(left.relationship_direction.as_deref().unwrap_or(""))
            == normalize_ascii_snake_key(right.relationship_direction.as_deref().unwrap_or(""))
        && (relationship_type_keys_match(
            left.relationship_type.as_deref(),
            right.relationship_type.as_deref(),
        ) || relationship_labels_overlap(
            relationship_value_label(left),
            relationship_value_label(right),
        ))
}

fn relationship_type_keys_match(left: Option<&str>, right: Option<&str>) -> bool {
    let left_key = normalize_cloud_schema_key(left.unwrap_or(""));
    let right_key = normalize_cloud_schema_key(right.unwrap_or(""));
    !left_key.is_empty() && left_key == right_key
}

fn merge_relationship_field_value(
    target: &mut StoryExtractionFieldValuePayload,
    source: StoryExtractionFieldValuePayload,
) {
    let source_label = relationship_value_label(&source).trim().to_string();
    if !source_label.is_empty() {
        update_preferred_payload_relationship_label(target, &source_label);
    }

    if let Some(source_type) = source.relationship_type.as_deref() {
        let source_type = normalize_cloud_schema_key(source_type);
        if relationship_type_prefer_candidate(
            target.relationship_type.as_deref().unwrap_or(""),
            &source_type,
        ) {
            target.relationship_type = Some(source_type);
        }
    }

    target.confidence = max_optional_confidence(target.confidence, source.confidence);
    merge_evidence_spans(&mut target.evidence, source.evidence);
}

fn update_preferred_payload_relationship_label(
    target: &mut StoryExtractionFieldValuePayload,
    candidate: &str,
) {
    let current = relationship_value_label(target);
    if relationship_label_prefer_candidate(current, candidate) {
        target.value = candidate.to_string();
        target.relationship_label = Some(candidate.to_string());
    }
}

fn relationship_value_label(value: &StoryExtractionFieldValuePayload) -> &str {
    value
        .relationship_label
        .as_deref()
        .unwrap_or(&value.value)
        .trim()
}

fn update_preferred_relationship_label(target: &mut Option<String>, candidate: &str) {
    let candidate = candidate.trim();
    if candidate.is_empty() {
        return;
    }

    match target.as_deref() {
        Some(current) if !relationship_label_prefer_candidate(current, candidate) => {}
        _ => *target = Some(candidate.to_string()),
    }
}

fn relationship_label_prefer_candidate(current: &str, candidate: &str) -> bool {
    let current = current.trim();
    let candidate = candidate.trim();
    if candidate.is_empty() {
        return false;
    }
    if current.is_empty() {
        return true;
    }

    relationship_label_specificity(candidate) > relationship_label_specificity(current)
}

fn relationship_label_specificity(value: &str) -> usize {
    let key = normalized_folded_text_key(value);
    let token_count = key.split('_').filter(|token| !token.is_empty()).count();
    let char_count = key.chars().filter(|ch| ch.is_ascii_alphanumeric()).count();
    token_count * 1000 + char_count
}

fn relationship_type_prefer_candidate(current: &str, candidate: &str) -> bool {
    let current = normalize_cloud_schema_key(current);
    let candidate = normalize_cloud_schema_key(candidate);
    !candidate.is_empty()
        && (current.is_empty()
            || relationship_label_specificity(&candidate)
                > relationship_label_specificity(&current))
}

fn optional_relationship_labels_overlap(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => relationship_labels_overlap(left, right),
        _ => false,
    }
}

fn relationship_labels_overlap(left: &str, right: &str) -> bool {
    let left_tokens = relationship_label_tokens(left);
    let right_tokens = relationship_label_tokens(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return false;
    }
    left_tokens == right_tokens
        || token_slice_is_prefix(&left_tokens, &right_tokens)
        || token_slice_is_prefix(&right_tokens, &left_tokens)
}

fn relationship_label_tokens(value: &str) -> Vec<String> {
    normalized_folded_text_key(value)
        .split('_')
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn token_slice_is_prefix(left: &[String], right: &[String]) -> bool {
    left.len() < right.len() && right.starts_with(left)
}

fn max_optional_confidence(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn merge_evidence_spans(target: &mut Vec<StoryEvidenceSpan>, source: Vec<StoryEvidenceSpan>) {
    let mut seen = target
        .iter()
        .map(evidence_span_key)
        .collect::<HashSet<String>>();
    for evidence in source {
        if seen.insert(evidence_span_key(&evidence)) {
            target.push(evidence);
        }
    }

    target.sort_by(|left, right| {
        left.chapter_num
            .cmp(&right.chapter_num)
            .then_with(|| left.start_char.cmp(&right.start_char))
            .then_with(|| left.end_char.cmp(&right.end_char))
            .then_with(|| left.quote.cmp(&right.quote))
    });
}

fn evidence_span_key(evidence: &StoryEvidenceSpan) -> String {
    format!(
        "{}:{}:{}:{}",
        evidence.chapter_num,
        evidence.start_char.unwrap_or(-1),
        evidence.end_char.unwrap_or(-1),
        normalized_text_key(evidence.quote.as_deref().unwrap_or(""))
    )
}

fn merge_record_fields(
    target: &mut StoryExtractionRecordPayload,
    fields: Vec<StoryExtractionFieldPayload>,
) {
    for field in fields {
        if let Some(existing_field) = target.fields.iter_mut().find(|existing| {
            normalize_ascii_snake_key(&existing.field_key)
                == normalize_ascii_snake_key(&field.field_key)
        }) {
            existing_field.values.extend(field.values);
        } else {
            target.fields.push(field);
        }
    }
}

fn merge_record_mentions(
    target: &mut StoryExtractionRecordPayload,
    mut mentions: Vec<StoryCharacterMention>,
) {
    target.mentions.append(&mut mentions);
    target.mentions.sort_by(|left, right| {
        left.start_char
            .cmp(&right.start_char)
            .then_with(|| right.end_char.cmp(&left.end_char))
            .then_with(|| left.text.cmp(&right.text))
    });
    target.mentions.dedup_by(|left, right| {
        left.start_char == right.start_char
            && left.end_char == right.end_char
            && normalized_text_key(&left.text) == normalized_text_key(&right.text)
    });
}

#[derive(Debug, Clone, Deserialize)]
struct CloudExtractionDocument {
    schema_version: String,
    chapter_num: i64,
    #[serde(default)]
    call_profile: Option<String>,
    #[serde(default)]
    characters: Vec<CloudCharacter>,
    #[serde(default)]
    relationships: Vec<CloudRelationship>,
    #[serde(default)]
    review_items: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct CloudCharacter {
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    entity_nature: String,
    #[serde(default)]
    fields: Vec<CloudCharacterField>,
    #[serde(default)]
    evidence_quotes: Vec<String>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct CloudCharacterField {
    #[serde(default)]
    field_key: String,
    #[serde(default)]
    field_label: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    semantic_class: String,
    #[serde(default)]
    evidence_quotes: Vec<String>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct CloudRelationship {
    #[serde(default)]
    source_name: String,
    #[serde(default)]
    target_name: String,
    #[serde(default)]
    relationship_scope: String,
    #[serde(default)]
    relationship_type: String,
    #[serde(default)]
    source_to_target_label: String,
    #[serde(default)]
    target_to_source_label: String,
    #[serde(default)]
    evidence_quotes: Vec<String>,
    confidence: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probable_alias_merge_uses_near_match_only_against_alias_surfaces() {
        let source = character_record_for_test("Hàn Chú", 1, &[]);
        let target = character_record_for_test("Hàn phụ", 4, &[]);
        let records = vec![source, target];

        assert!(probable_alias_record_merge_target(&records, 0).is_none());
    }

    #[test]
    fn probable_alias_merge_accepts_near_match_against_known_alias() {
        let source = character_record_for_test("Anh ngốc", 1, &[]);
        let target = character_record_for_test("Hàn Lập", 10, &["Anh ngố"]);
        let records = vec![source, target];

        assert_eq!(
            probable_alias_record_merge_target(&records, 0),
            Some((1, "near_alias_surface"))
        );
    }

    #[test]
    fn relationship_merge_collapses_label_variants_and_keeps_evidence() {
        let mut field = StoryExtractionFieldPayload {
            field_key: FIELD_KEY_RELATIONSHIP.to_string(),
            field_label: FIELD_LABEL_RELATIONSHIP.to_string(),
            values: vec![relationship_value_for_test(
                "uncle",
                "kinship_uncle",
                "Hàn Lập",
                RELATIONSHIP_DIRECTION_SELF_TO_RELATED,
                1,
                "quote one",
            )],
        };

        merge_relationship_field_values(
            &mut field,
            vec![relationship_value_for_test(
                "uncle by blood",
                "kinship_uncle",
                "Hàn Lập",
                RELATIONSHIP_DIRECTION_SELF_TO_RELATED,
                1,
                "quote two",
            )],
        );

        assert_eq!(field.values.len(), 1);
        assert_eq!(
            field.values[0].relationship_label.as_deref(),
            Some("uncle by blood")
        );
        assert_eq!(field.values[0].evidence.len(), 2);
    }

    #[test]
    fn known_relationship_context_uses_only_prior_visible_endpoint_pairs() {
        let identities = vec![
            CharacterIdentity {
                name: "Hàn Lập".to_string(),
                aliases: Vec::new(),
            },
            CharacterIdentity {
                name: "Tam Thúc".to_string(),
                aliases: Vec::new(),
            },
        ];
        let records = vec![
            relationship_record_view_for_test(1, "Hàn Lập", "Tam Thúc", "uncle", "nephew"),
            relationship_record_view_for_test(3, "Hàn Lập", "Tam Thúc", "uncle", "nephew"),
            relationship_record_view_for_test(1, "Hàn Lập", "Người Lạ", "ally", "ally"),
        ];

        let hints = build_known_relationship_context(&records, &identities, 2);

        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].left_name, "Hàn Lập");
        assert_eq!(hints[0].right_name, "Tam Thúc");
        assert_eq!(hints[0].evidence_chapter_nums, vec![1]);
    }

    fn character_record_for_test(
        display_name: &str,
        mention_count: usize,
        aliases: &[&str],
    ) -> StoryExtractionRecordPayload {
        let mentions = (0..mention_count)
            .map(|index| StoryCharacterMention {
                text: display_name.to_string(),
                start_char: index as i64,
                end_char: index as i64 + display_name.chars().count() as i64,
                mention_type: Some("name".to_string()),
            })
            .collect::<Vec<_>>();
        let alias_values = aliases
            .iter()
            .map(|alias| StoryExtractionFieldValuePayload {
                value: (*alias).to_string(),
                confidence: Some(1.0),
                related_character: None,
                relationship_type: None,
                relationship_label: None,
                relationship_direction: None,
                evidence: Vec::new(),
            })
            .collect::<Vec<_>>();
        let fields = if alias_values.is_empty() {
            Vec::new()
        } else {
            vec![StoryExtractionFieldPayload {
                field_key: FIELD_KEY_OTHER_ALIAS.to_string(),
                field_label: FIELD_LABEL_OTHER_ALIAS.to_string(),
                values: alias_values,
            }]
        };

        StoryExtractionRecordPayload {
            group_key: GROUP_KEY_CHARACTER.to_string(),
            group_label: GROUP_LABEL_CHARACTER.to_string(),
            entity_key: Some(normalize_ascii_snake_key(display_name)),
            display_name: display_name.to_string(),
            mentions,
            fields,
        }
    }

    fn relationship_value_for_test(
        label: &str,
        relationship_type: &str,
        related_character: &str,
        direction: &str,
        chapter_num: i64,
        quote: &str,
    ) -> StoryExtractionFieldValuePayload {
        StoryExtractionFieldValuePayload {
            value: label.to_string(),
            confidence: Some(0.95),
            related_character: Some(related_character.to_string()),
            relationship_type: Some(relationship_type.to_string()),
            relationship_label: Some(label.to_string()),
            relationship_direction: Some(direction.to_string()),
            evidence: vec![StoryEvidenceSpan {
                chapter_num,
                start_char: Some(0),
                end_char: Some(quote.chars().count() as i64),
                quote: Some(quote.to_string()),
                reason: Some(EVIDENCE_REASON_CLOUD_RELATIONSHIP.to_string()),
            }],
        }
    }

    fn relationship_record_view_for_test(
        chapter_num: i64,
        left_name: &str,
        right_name: &str,
        left_label: &str,
        right_label: &str,
    ) -> StoryExtractionRecordView {
        let left_key = normalize_ascii_snake_key(left_name);
        let right_key = normalize_ascii_snake_key(right_name);
        StoryExtractionRecordView {
            id: format!("record-{chapter_num}-{left_key}-{right_key}"),
            project_id: "project".to_string(),
            novel_id: "novel".to_string(),
            chapter_id: format!("chapter-{chapter_num}"),
            job_id: "job".to_string(),
            run_id: "run".to_string(),
            chapter_num,
            group_key: GROUP_KEY_RELATIONSHIP.to_string(),
            group_label: GROUP_LABEL_RELATIONSHIP.to_string(),
            entity_key: Some(format!("relationship|{left_key}|{right_key}")),
            display_name: format!("{left_name} ↔ {right_name}"),
            prompt_schema_version: CLOUD_CHAPTER_EXTRACTION_SCHEMA_VERSION.to_string(),
            mentions: Vec::new(),
            fields: vec![novelgraph_core::StoryExtractionFieldView {
                id: format!("field-{chapter_num}-{left_key}-{right_key}"),
                record_id: format!("record-{chapter_num}-{left_key}-{right_key}"),
                field_key: FIELD_KEY_RELATIONSHIP.to_string(),
                field_label: FIELD_LABEL_RELATIONSHIP.to_string(),
                values: vec![
                    relationship_value_view_for_test(
                        left_label,
                        "kinship_uncle",
                        right_name,
                        RELATIONSHIP_DIRECTION_SELF_TO_RELATED,
                        chapter_num,
                    ),
                    relationship_value_view_for_test(
                        right_label,
                        "kinship_uncle",
                        left_name,
                        RELATIONSHIP_DIRECTION_RELATED_TO_SELF,
                        chapter_num,
                    ),
                ],
                created_at: String::new(),
                updated_at: String::new(),
            }],
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn relationship_value_view_for_test(
        label: &str,
        relationship_type: &str,
        related_character: &str,
        direction: &str,
        chapter_num: i64,
    ) -> StoryExtractionFieldValueView {
        StoryExtractionFieldValueView {
            id: format!("value-{chapter_num}-{label}-{direction}"),
            field_id: "field".to_string(),
            value: label.to_string(),
            confidence: Some(0.95),
            related_character: Some(related_character.to_string()),
            relationship_type: Some(relationship_type.to_string()),
            relationship_label: Some(label.to_string()),
            relationship_direction: Some(direction.to_string()),
            evidence: vec![StoryEvidenceSpan {
                chapter_num,
                start_char: Some(0),
                end_char: Some(label.chars().count() as i64),
                quote: Some(format!("quote {chapter_num}")),
                reason: Some(EVIDENCE_REASON_CLOUD_RELATIONSHIP.to_string()),
            }],
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}
