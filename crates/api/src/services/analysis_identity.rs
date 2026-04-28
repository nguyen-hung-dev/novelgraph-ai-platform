use crate::services::analysis_identity_review as review;
use crate::*;
use serde_json::json;

pub(crate) async fn resolve_character_identities_across_chapters(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identities: Vec<CharacterIdentity>,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
) -> (Vec<CharacterIdentity>, Vec<serde_json::Value>) {
    let mut resolved = Vec::new();
    let mut merge_decision_outputs = Vec::new();

    for identity in identities {
        let (canonical, merge_decision_output) = resolve_character_identity_across_chapters(
            state,
            chunk_input,
            identity,
            db_records,
            db_aliases,
            working_document,
        )
        .await;
        if let Some(output) = merge_decision_output {
            merge_decision_outputs.push(output);
        }
        if let Some(canonical) = canonical {
            review::merge_character_identity_into_list(&mut resolved, canonical);
        }
    }

    (resolved, merge_decision_outputs)
}

async fn resolve_character_identity_across_chapters(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
) -> (Option<CharacterIdentity>, Option<serde_json::Value>) {
    let known_name_keys = review::known_character_name_keys(db_records, working_document);

    if let Some(record) = find_exact_db_character_record(&identity, db_records) {
        return (
            Some(review::identity_from_db_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(record) = find_exact_working_character_record(&identity, working_document) {
        return (
            Some(review::identity_from_payload_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(candidate) =
        find_exact_alias_map_character_identity(&identity, db_aliases, &known_name_keys)
    {
        return (Some(candidate), None);
    }

    if let Some(candidate) =
        find_high_confidence_alias_map_character_identity(&identity, db_aliases, &known_name_keys)
    {
        return (Some(candidate), None);
    }

    if let Some(record) = find_high_confidence_db_character_record(&identity, db_records) {
        return (
            Some(review::identity_from_db_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(record) = find_high_confidence_working_character_record(&identity, working_document)
    {
        return (
            Some(review::identity_from_payload_record(
                record,
                Some(&identity),
                &known_name_keys,
            )),
            None,
        );
    }

    if let Some(candidate) = review::find_character_merge_review_candidate(
        &identity,
        db_records,
        db_aliases,
        working_document,
    ) {
        let (decision, response) =
            review::confirm_character_identity_merge(state, chunk_input, &identity, &candidate)
                .await;
        let action = review::normalize_character_identity_merge_action(&decision.action);
        let confidence = decision.confidence.unwrap_or(0.0);
        let observed_name = identity.name.clone();
        let candidate_name = candidate.display_name.clone();

        if action == "merge_existing"
            && candidate.score >= CHARACTER_CANONICAL_AI_MERGE_MIN_SCORE
            && confidence >= CHARACTER_CANONICAL_MERGE_MIN_CONFIDENCE
        {
            let mut canonical = CharacterIdentity {
                name: candidate.display_name.clone(),
                aliases: candidate.aliases.clone(),
            };
            review::merge_observed_identity_aliases(
                &mut canonical,
                Some(&identity),
                &known_name_keys,
            );
            return (
                Some(canonical),
                Some(json!({
                    "mode": "ai_merge_confirmation",
                    "applied": "merge_existing",
                    "observed_identity": observed_name,
                    "candidate_identity": candidate_name,
                    "candidate_score": candidate.score,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        if action == "ignore" && confidence >= CHARACTER_CANONICAL_IGNORE_MIN_CONFIDENCE {
            return (
                None,
                Some(json!({
                    "mode": "ai_merge_confirmation",
                    "applied": "ignore",
                    "observed_identity": observed_name,
                    "candidate_identity": candidate_name,
                    "candidate_score": candidate.score,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        if action == "create_new"
            && review::observed_identity_is_known_name_phrase(
                &identity,
                db_records,
                working_document,
            )
        {
            return (
                None,
                Some(json!({
                    "mode": "ai_merge_confirmation",
                    "applied": "ignore_known_name_phrase",
                    "observed_identity": observed_name,
                    "candidate_identity": candidate_name,
                    "candidate_score": candidate.score,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        let sanitized =
            review::sanitize_new_character_identity(identity, db_records, working_document);
        return (
            Some(sanitized),
            Some(json!({
                "mode": "ai_merge_confirmation",
                "applied": "create_new",
                "observed_identity": observed_name,
                "candidate_identity": candidate_name,
                "candidate_score": candidate.score,
                "decision": decision,
                "response": response,
            })),
        );
    }

    if let Some(review_candidates) = review::character_identity_creation_review_candidates(
        &identity,
        db_records,
        db_aliases,
        working_document,
        &chunk_input.text,
    ) {
        let (decision, response) = review::confirm_character_identity_creation(
            state,
            chunk_input,
            &identity,
            &review_candidates,
        )
        .await;
        let action = review::normalize_character_identity_creation_action(&decision.action);
        let confidence = decision.confidence.unwrap_or(0.0);
        let observed_name = identity.name.clone();
        let candidate =
            review::find_creation_review_target_candidate(&decision, &review_candidates);

        if action == "merge_existing"
            && confidence >= CHARACTER_IDENTITY_CREATION_REVIEW_MIN_CONFIDENCE
        {
            if let Some(candidate) = candidate {
                let mut canonical = review::identity_from_alias_map_candidate(
                    candidate.clone(),
                    Some(&identity),
                    &known_name_keys,
                );
                review::merge_observed_identity_aliases(
                    &mut canonical,
                    Some(&identity),
                    &known_name_keys,
                );
                return (
                    Some(canonical),
                    Some(json!({
                        "mode": "identity_creation_review",
                        "applied": "merge_existing",
                        "observed_identity": observed_name,
                        "candidate_identity": candidate.display_name,
                        "decision": decision,
                        "response": response,
                    })),
                );
            }
        }

        if action == "reject" && confidence >= CHARACTER_IDENTITY_REJECT_MIN_CONFIDENCE {
            return (
                None,
                Some(json!({
                    "mode": "identity_creation_review",
                    "applied": "reject",
                    "observed_identity": observed_name,
                    "decision": decision,
                    "response": response,
                })),
            );
        }

        if review::observed_identity_is_known_name_phrase(&identity, db_records, working_document) {
            return (
                None,
                Some(json!({
                    "mode": "identity_creation_review",
                    "applied": "reject_known_name_phrase",
                    "observed_identity": observed_name,
                    "decision": decision,
                    "response": response,
                })),
            );
        }
    } else if review::observed_identity_is_known_name_phrase(
        &identity,
        db_records,
        working_document,
    ) {
        return (
            None,
            Some(json!({
                "mode": "identity_creation_review",
                "applied": "reject_known_name_phrase",
                "observed_identity": identity.name,
            })),
        );
    }

    (
        Some(review::sanitize_new_character_identity(
            identity,
            db_records,
            working_document,
        )),
        None,
    )
}

fn find_exact_db_character_record<'a>(
    identity: &CharacterIdentity,
    db_records: &'a [StoryExtractionRecordView],
) -> Option<&'a StoryExtractionRecordView> {
    let name_key = normalized_text_key(&identity.name);
    db_records
        .iter()
        .find(|record| normalized_text_key(&record.display_name) == name_key)
}

fn find_exact_working_character_record<'a>(
    identity: &CharacterIdentity,
    working_document: &'a StoryExtractionDocument,
) -> Option<&'a StoryExtractionRecordPayload> {
    let name_key = normalized_text_key(&identity.name);
    working_document
        .records
        .iter()
        .find(|record| normalized_text_key(&record.display_name) == name_key)
}

fn find_exact_alias_map_character_identity(
    identity: &CharacterIdentity,
    db_aliases: &[StoryCharacterAliasView],
    known_name_keys: &std::collections::HashSet<String>,
) -> Option<CharacterIdentity> {
    let mut matched_candidate: Option<CharacterIdentityMergeCandidate> = None;
    for candidate in review::character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        if review::character_identity_candidate_score(identity, &candidate_identity) < 1.0 {
            continue;
        }

        if matched_candidate
            .as_ref()
            .is_some_and(|matched| matched.target_key != candidate.target_key)
        {
            return None;
        }

        matched_candidate = Some(candidate);
    }

    matched_candidate.map(|candidate| {
        review::identity_from_alias_map_candidate(candidate, Some(identity), known_name_keys)
    })
}

fn find_high_confidence_db_character_record<'a>(
    identity: &CharacterIdentity,
    db_records: &'a [StoryExtractionRecordView],
) -> Option<&'a StoryExtractionRecordView> {
    let mut best: Option<(&StoryExtractionRecordView, f64)> = None;
    let mut best_tie_count = 0;

    for record in db_records {
        let candidate = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_record(record),
        };
        let score = review::character_identity_candidate_score(identity, &candidate);
        if score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            Some((_, best_score)) if (score - best_score).abs() < f64::EPSILON => {
                best_tie_count += 1;
            }
            None => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            _ => {}
        }
    }

    if best_tie_count == 1 {
        best.map(|(record, _)| record)
    } else {
        None
    }
}

fn find_high_confidence_working_character_record<'a>(
    identity: &CharacterIdentity,
    working_document: &'a StoryExtractionDocument,
) -> Option<&'a StoryExtractionRecordPayload> {
    let mut best: Option<(&StoryExtractionRecordPayload, f64)> = None;
    let mut best_tie_count = 0;

    for record in &working_document.records {
        if record.group_key != "character" {
            continue;
        }

        let candidate = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_payload_record(record),
        };
        let score = review::character_identity_candidate_score(identity, &candidate);
        if score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            Some((_, best_score)) if (score - best_score).abs() < f64::EPSILON => {
                best_tie_count += 1;
            }
            None => {
                best = Some((record, score));
                best_tie_count = 1;
            }
            _ => {}
        }
    }

    if best_tie_count == 1 {
        best.map(|(record, _)| record)
    } else {
        None
    }
}

fn find_high_confidence_alias_map_character_identity(
    identity: &CharacterIdentity,
    db_aliases: &[StoryCharacterAliasView],
    known_name_keys: &std::collections::HashSet<String>,
) -> Option<CharacterIdentity> {
    let mut best: Option<(CharacterIdentityMergeCandidate, f64)> = None;
    let mut best_tie_count = 0;

    for candidate in review::character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        let score = review::character_identity_candidate_score(identity, &candidate_identity);
        if score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => {
                best = Some((candidate, score));
                best_tie_count = 1;
            }
            Some((_, best_score)) if (score - best_score).abs() < f64::EPSILON => {
                best_tie_count += 1;
            }
            None => {
                best = Some((candidate, score));
                best_tie_count = 1;
            }
            _ => {}
        }
    }

    if best_tie_count == 1 {
        best.map(|(candidate, _)| {
            review::identity_from_alias_map_candidate(candidate, Some(identity), known_name_keys)
        })
    } else {
        None
    }
}
