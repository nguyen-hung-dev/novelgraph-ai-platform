use std::collections::HashMap;

use serde_json::json;

use crate::*;

pub(crate) async fn extract_character_relationships_for_chapter(
    state: &AppState,
    project_id: &str,
    job_id: &str,
    chapter: &Chapter,
    novel: &Novel,
    document: &mut StoryExtractionDocument,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let identities = character_identities_for_relationships(document);
    let mut outputs = Vec::new();

    if identities.len() < 2 {
        return Ok(outputs);
    }

    let identity_nodes_json =
        serde_json::to_string(&identities).unwrap_or_else(|_| "[]".to_string());
    let novel_analysis_context = build_current_novel_analysis_context(novel);
    let relationship_input = DraftExtractionInput {
        chapter_num: chapter.chapter_num,
        title: Some(chapter.title.clone()),
        source_language: novel.source_language.clone(),
        text: chapter.content.clone(),
        prior_context: Some(format!(
            "{novel_analysis_context}\n\nĐây là relationship candidate pass sau khi character identity, aliases, mentions và fields của chương hiện tại đã hoàn tất. Chỉ trả quan hệ có evidence trong chương hiện tại; aliases chỉ dùng để resolve về canonical character, không phải node riêng."
        )),
    };
    let prompt =
        build_character_relationship_candidate_prompt(&relationship_input, &identity_nodes_json);
    let (candidates, response) = match services::llm_json::call_local_json_array::<
        CharacterRelationshipCandidate,
    >(
        state, &prompt, CHARACTER_RELATIONSHIP_CANDIDATE_MAX_TOKENS
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            outputs.push(json!({
                "mode": "relationship_candidate_pass_failed_non_blocking",
                "error": error.message,
            }));
            return Ok(outputs);
        }
    };

    let candidate_count = candidates.len();
    let (relationships_by_pair, resolution_warnings) =
        resolve_character_relationship_candidates(candidates, &identities, chapter);
    let mut persisted_record_count = 0usize;
    let mut persisted_relationship_count = 0usize;
    let mut relationship_verification_reports = Vec::new();

    for ((left_index, right_index), relationships) in relationships_by_pair {
        let character_a = &identities[left_index];
        let character_b = &identities[right_index];
        let relationships = normalize_character_relationships(relationships);
        if relationships.is_empty() {
            continue;
        }
        let (relationships, reports) = verify_character_relationships(
            state,
            &relationship_input,
            character_a,
            character_b,
            relationships,
        )
        .await;
        relationship_verification_reports.extend(reports);
        if relationships.is_empty() {
            continue;
        }

        persisted_relationship_count += relationships.len();
        if let Some(record) = relationship_pair_to_record(character_a, character_b, relationships) {
            document.records.push(record);
            persisted_record_count += 1;
        }
    }

    if persisted_record_count > 0 {
        state
            .store
            .replace_story_extraction_records_for_chapter(
                project_id,
                job_id,
                &chapter.id,
                CHARACTER_EXTRACTION_SCHEMA_VERSION,
                document,
                "character_relationship_candidate_pass",
            )
            .await?;
        publish_project_event(
            state,
            project_id,
            "story_extraction_updated",
            Some(job_id),
            Some(&chapter.id),
            "character relationship candidates persisted",
        );
    }

    outputs.push(json!({
        "mode": "relationship_candidate_pass",
        "candidate_count": candidate_count,
        "persisted_record_count": persisted_record_count,
        "persisted_relationship_count": persisted_relationship_count,
        "resolution_warnings": resolution_warnings,
        "verification": relationship_verification_reports,
        "response": response,
    }));

    Ok(outputs)
}

fn resolve_character_relationship_candidates(
    candidates: Vec<CharacterRelationshipCandidate>,
    identities: &[CharacterIdentity],
    chapter: &Chapter,
) -> (
    HashMap<(usize, usize), Vec<CharacterRelationshipExtraction>>,
    Vec<serde_json::Value>,
) {
    let mut relationships_by_pair =
        HashMap::<(usize, usize), Vec<CharacterRelationshipExtraction>>::new();
    let mut warnings = Vec::new();

    for candidate in candidates {
        let source_name = clean_character_surface(&candidate.source_name);
        let target_name = clean_character_surface(&candidate.target_name);
        let relationship_kind = normalize_relationship_candidate_kind(&candidate.relationship_kind);
        if relationship_kind != "stable_relation" {
            warnings.push(json!({
                "code": "relationship_candidate_not_stable",
                "source_name": source_name,
                "target_name": target_name,
                "relationship_kind": relationship_kind,
            }));
            continue;
        }

        let confidence = candidate.confidence.unwrap_or(0.0);
        if confidence < CHARACTER_RELATIONSHIP_MIN_CONFIDENCE {
            warnings.push(json!({
                "code": "relationship_candidate_low_confidence",
                "source_name": source_name,
                "target_name": target_name,
                "confidence": confidence,
            }));
            continue;
        }

        let Some(source_index) = resolve_relationship_identity_index(identities, &source_name)
        else {
            warnings.push(json!({
                "code": "relationship_source_not_resolved",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        };
        let Some(target_index) = resolve_relationship_identity_index(identities, &target_name)
        else {
            warnings.push(json!({
                "code": "relationship_target_not_resolved",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        };

        if source_index == target_index {
            warnings.push(json!({
                "code": "relationship_self_reference_skipped",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        }

        if !relationship_candidate_has_grounded_evidence(&candidate, chapter) {
            warnings.push(json!({
                "code": "relationship_candidate_not_grounded",
                "source_name": source_name,
                "target_name": target_name,
            }));
            continue;
        }

        let (left_index, right_index, relationship) = relationship_candidate_to_pair_extraction(
            candidate,
            source_index,
            target_index,
            chapter.chapter_num,
        );
        relationships_by_pair
            .entry((left_index, right_index))
            .or_default()
            .push(relationship);
    }

    (relationships_by_pair, warnings)
}

fn normalize_relationship_candidate_kind(value: &str) -> &'static str {
    match normalize_ascii_snake_key(value).as_str() {
        "stable_relation" | "relationship" | "relation" | "direct_relation" => "stable_relation",
        "temporary_interaction" | "interaction" | "scene_interaction" => "temporary_interaction",
        "event" | "action" | "scene_event" => "event",
        _ => "uncertain",
    }
}

fn resolve_relationship_identity_index(
    identities: &[CharacterIdentity],
    surface: &str,
) -> Option<usize> {
    let key = normalized_text_key(surface);
    if key.is_empty() {
        return None;
    }

    let mut matched_index = None;
    for (index, identity) in identities.iter().enumerate() {
        let matches_identity = character_identity_surfaces(identity)
            .into_iter()
            .any(|(candidate_surface, _)| normalized_text_key(&candidate_surface) == key);
        if !matches_identity {
            continue;
        }

        if matched_index.is_some() {
            return None;
        }

        matched_index = Some(index);
    }

    matched_index
}

fn relationship_candidate_has_grounded_evidence(
    candidate: &CharacterRelationshipCandidate,
    chapter: &Chapter,
) -> bool {
    candidate.evidence.iter().any(|evidence| {
        evidence
            .quote
            .as_deref()
            .is_some_and(|quote| chapter_text_contains_evidence_quote(&chapter.content, quote))
    })
}

fn chapter_text_contains_evidence_quote(chapter_text: &str, quote: &str) -> bool {
    let quote = quote.trim();
    if quote.is_empty() {
        return false;
    }

    if chapter_text.contains(quote) {
        return true;
    }

    let normalized_chapter = collapse_evidence_whitespace(chapter_text);
    let normalized_quote = collapse_evidence_whitespace(quote);
    !normalized_quote.is_empty() && normalized_chapter.contains(&normalized_quote)
}

fn collapse_evidence_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn relationship_candidate_to_pair_extraction(
    candidate: CharacterRelationshipCandidate,
    source_index: usize,
    target_index: usize,
    chapter_num: i64,
) -> (usize, usize, CharacterRelationshipExtraction) {
    let relationship_type = if candidate.relationship_type.trim().is_empty() {
        normalize_ascii_snake_key(&candidate.source_to_target_label)
    } else {
        normalize_ascii_snake_key(&candidate.relationship_type)
    };
    let source_to_target_label = candidate.source_to_target_label.trim().to_string();
    let target_to_source_label = candidate.target_to_source_label.trim().to_string();
    let target_to_source_label = if target_to_source_label.is_empty() {
        source_to_target_label.clone()
    } else {
        target_to_source_label
    };
    let evidence = normalize_relationship_candidate_evidence(candidate.evidence, chapter_num);

    if source_index < target_index {
        (
            source_index,
            target_index,
            CharacterRelationshipExtraction {
                relationship_type,
                a_to_b_label: source_to_target_label,
                b_to_a_label: target_to_source_label,
                confidence: candidate.confidence,
                evidence,
            },
        )
    } else {
        (
            target_index,
            source_index,
            CharacterRelationshipExtraction {
                relationship_type,
                a_to_b_label: target_to_source_label,
                b_to_a_label: source_to_target_label,
                confidence: candidate.confidence,
                evidence,
            },
        )
    }
}

fn normalize_relationship_candidate_evidence(
    evidence: Vec<StoryEvidenceSpan>,
    chapter_num: i64,
) -> Vec<StoryEvidenceSpan> {
    evidence
        .into_iter()
        .filter_map(|mut evidence| {
            let quote = evidence.quote.as_deref()?.trim();
            if quote.is_empty() {
                return None;
            }

            evidence.chapter_num = chapter_num;
            evidence.start_char = None;
            evidence.end_char = None;
            evidence.quote = Some(quote.to_string());
            Some(evidence)
        })
        .collect()
}

fn character_identities_for_relationships(
    document: &StoryExtractionDocument,
) -> Vec<CharacterIdentity> {
    let mut identities = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for record in &document.records {
        if record.group_key != "character" || record.mentions.is_empty() {
            continue;
        }

        let key = normalize_ascii_snake_key(
            record
                .entity_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(&record.display_name),
        );
        if key.is_empty() || !seen.insert(key) {
            continue;
        }

        identities.push(CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_payload_record(record),
        });
    }

    identities
}

fn normalize_character_relationships(
    relationships: Vec<CharacterRelationshipExtraction>,
) -> Vec<CharacterRelationshipExtraction> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for mut relationship in relationships {
        relationship.relationship_type = normalize_ascii_snake_key(&relationship.relationship_type);
        relationship.a_to_b_label = relationship.a_to_b_label.trim().to_string();
        relationship.b_to_a_label = relationship.b_to_a_label.trim().to_string();

        if relationship.relationship_type.is_empty()
            || relationship.a_to_b_label.is_empty()
            || relationship.b_to_a_label.is_empty()
            || !relationship.evidence.iter().any(|evidence| {
                evidence
                    .quote
                    .as_deref()
                    .is_some_and(|quote| !quote.trim().is_empty())
            })
        {
            continue;
        }

        for evidence in &mut relationship.evidence {
            evidence.start_char = None;
            evidence.end_char = None;
        }

        let key = format!(
            "{}:{}:{}",
            relationship.relationship_type,
            normalized_text_key(&relationship.a_to_b_label),
            normalized_text_key(&relationship.b_to_a_label)
        );
        if seen.insert(key) {
            normalized.push(relationship);
        }
    }

    normalized
}

async fn verify_character_relationships(
    state: &AppState,
    relationship_input: &DraftExtractionInput,
    character_a: &CharacterIdentity,
    character_b: &CharacterIdentity,
    relationships: Vec<CharacterRelationshipExtraction>,
) -> (Vec<CharacterRelationshipExtraction>, Vec<serde_json::Value>) {
    let character_a_json = serde_json::to_string(character_a).unwrap_or_else(|_| "{}".to_string());
    let character_b_json = serde_json::to_string(character_b).unwrap_or_else(|_| "{}".to_string());
    let mut verified_relationships = Vec::new();
    let mut reports = Vec::new();

    for relationship in relationships {
        let relationship_json = serde_json::to_string(&json!({
            "relationship_type": &relationship.relationship_type,
            "a_to_b_label": &relationship.a_to_b_label,
            "b_to_a_label": &relationship.b_to_a_label,
            "confidence": relationship.confidence,
            "evidence": &relationship.evidence,
        }))
        .unwrap_or_else(|_| "{}".to_string());
        let prompt = build_character_relationship_verification_prompt(
            relationship_input,
            &character_a_json,
            &character_b_json,
            &relationship_json,
        );
        let verification_result =
            services::llm_json::call_local_json_array::<CharacterRelationshipVerification>(
                state,
                &prompt,
                CHARACTER_RELATIONSHIP_VERIFICATION_MAX_TOKENS,
            )
            .await;

        match verification_result {
            Ok((verifications, response)) => {
                let verification = verifications.into_iter().next();
                let accepted = verification
                    .as_ref()
                    .is_some_and(|verification| relationship_verification_accepts(verification));
                reports.push(json!({
                    "character_a": &character_a.name,
                    "character_b": &character_b.name,
                    "relationship_type": &relationship.relationship_type,
                    "a_to_b_label": &relationship.a_to_b_label,
                    "b_to_a_label": &relationship.b_to_a_label,
                    "accepted": accepted,
                    "verification": verification,
                    "response": response,
                }));

                if accepted {
                    verified_relationships.push(relationship);
                }
            }
            Err(error) => {
                reports.push(json!({
                    "character_a": &character_a.name,
                    "character_b": &character_b.name,
                    "relationship_type": &relationship.relationship_type,
                    "a_to_b_label": &relationship.a_to_b_label,
                    "b_to_a_label": &relationship.b_to_a_label,
                    "accepted": false,
                    "mode": "relationship_verification_failed",
                    "error": error.message,
                }));
            }
        }
    }

    (verified_relationships, reports)
}

fn relationship_verification_accepts(verification: &CharacterRelationshipVerification) -> bool {
    verification.accepted
        && verification.owner_direction_ok
        && verification.confidence.unwrap_or(0.0)
            >= CHARACTER_RELATIONSHIP_VERIFICATION_MIN_CONFIDENCE
        && relationship_scope_is_persistable(&verification.relationship_scope)
}

fn relationship_scope_is_persistable(scope: &str) -> bool {
    matches!(
        normalize_ascii_snake_key(scope).as_str(),
        "kinship" | "organization_hierarchy" | "stable_relationship"
    )
}

fn relationship_pair_to_record(
    character_a: &CharacterIdentity,
    character_b: &CharacterIdentity,
    relationships: Vec<CharacterRelationshipExtraction>,
) -> Option<StoryExtractionRecordPayload> {
    let character_a_key = normalize_ascii_snake_key(&character_a.name);
    let character_b_key = normalize_ascii_snake_key(&character_b.name);
    if character_a_key.is_empty() || character_b_key.is_empty() {
        return None;
    }

    let mut values = Vec::new();
    for relationship in relationships {
        values.push(relationship_field_value(
            &relationship.a_to_b_label,
            &relationship,
            &character_b.name,
        ));
        values.push(relationship_field_value(
            &relationship.b_to_a_label,
            &relationship,
            &character_a.name,
        ));
    }

    if values.is_empty() {
        return None;
    }

    Some(StoryExtractionRecordPayload {
        group_key: "relationship".to_string(),
        group_label: "Quan Hệ".to_string(),
        entity_key: Some(format!(
            "relationship|{}|{}",
            character_a_key, character_b_key
        )),
        display_name: format!("{} ↔ {}", character_a.name, character_b.name),
        mentions: Vec::new(),
        fields: vec![StoryExtractionFieldPayload {
            field_key: "relationship".to_string(),
            field_label: "Quan hệ".to_string(),
            values,
        }],
    })
}

fn relationship_field_value(
    label: &str,
    relationship: &CharacterRelationshipExtraction,
    related_character: &str,
) -> StoryExtractionFieldValuePayload {
    StoryExtractionFieldValuePayload {
        value: label.to_string(),
        confidence: relationship.confidence,
        related_character: Some(related_character.to_string()),
        relationship_type: Some(relationship.relationship_type.clone()),
        relationship_label: Some(label.to_string()),
        relationship_direction: Some("self_to_related".to_string()),
        evidence: relationship.evidence.clone(),
    }
}
