use crate::services::llm_json::call_local_json_array;
use crate::*;
use serde_json::json;

pub(crate) fn build_character_field_contexts(
    chunk_text: &str,
    identity: &CharacterIdentity,
) -> Vec<String> {
    let mut contexts = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sentence in split_character_field_sentences(chunk_text) {
        if contexts.len() >= CHARACTER_FIELD_CONTEXT_MAX_ITEMS {
            break;
        }
        let Some(context) = mark_character_field_context(&sentence, identity) else {
            continue;
        };
        let key = normalized_text_key(&context);
        if !key.is_empty() && seen.insert(key) {
            contexts.push(context);
        }
    }

    contexts
}

fn split_character_field_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if is_character_field_sentence_boundary(ch) {
            push_character_field_sentence(&mut sentences, &mut current);
        }
    }
    push_character_field_sentence(&mut sentences, &mut current);

    sentences
}

fn push_character_field_sentence(sentences: &mut Vec<String>, current: &mut String) {
    let sentence = current.trim();
    if !sentence.is_empty() {
        sentences.push(sentence.to_string());
    }
    current.clear();
}

fn is_character_field_sentence_boundary(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?' | '。' | '！' | '？' | '\n' | '\r')
}

fn mark_character_field_context(sentence: &str, identity: &CharacterIdentity) -> Option<String> {
    let mut occurrences = Vec::new();
    for (surface, _) in character_identity_surfaces(identity) {
        occurrences.extend(find_surface_occurrences(
            sentence,
            &surface,
            "field_context",
            false,
        ));
    }

    let occurrences = select_non_overlapping_occurrences(occurrences);
    if occurrences.is_empty() {
        return None;
    }

    Some(insert_character_field_markers(sentence, &occurrences))
}

fn insert_character_field_markers(
    text: &str,
    occurrences: &[ScannedCharacterOccurrence],
) -> String {
    let starts = occurrences
        .iter()
        .map(|occurrence| occurrence.start_char as usize)
        .collect::<std::collections::HashSet<_>>();
    let ends = occurrences
        .iter()
        .map(|occurrence| occurrence.end_char as usize)
        .collect::<std::collections::HashSet<_>>();
    let mut marked = String::new();

    for (index, ch) in text.chars().enumerate() {
        if starts.contains(&index) {
            marked.push_str("[[");
        }
        marked.push(ch);
        if ends.contains(&(index + 1)) {
            marked.push_str("]]");
        }
    }

    marked
}

pub(crate) fn normalize_character_field_payloads(
    fields: Vec<StoryExtractionFieldPayload>,
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> Vec<StoryExtractionFieldPayload> {
    let mut normalized_fields = Vec::new();
    let identity_value_keys = character_identity_value_keys(identity);
    let other_character_value_keys =
        other_character_value_keys(identity, db_records, working_document);

    for mut field in fields {
        let Some((field_key, field_label)) = normalize_character_minimal_field(&field.field_key)
        else {
            continue;
        };
        field.field_key = field_key.to_string();
        field.field_label = field_label.to_string();

        let mut values = Vec::new();
        for mut value in field.values {
            value.value = value.value.trim().to_string();
            if value.value.is_empty() {
                continue;
            }

            if identity_value_keys.contains(&normalized_text_key(&value.value)) {
                continue;
            }
            if other_character_value_keys.contains(&normalized_text_key(&value.value)) {
                continue;
            }
            if !field_value_has_quoted_evidence(&value) {
                continue;
            }
            if !field_value_has_target_marker(&value) {
                continue;
            }
            if !field_value_has_minimum_confidence(&value) {
                continue;
            }
            if !field_value_is_grounded_in_quoted_evidence(&value) {
                continue;
            }
            normalize_character_field_evidence_chapter(&mut value, working_document.chapter_num);
            strip_character_field_markers_from_evidence(&mut value);
            value.related_character = None;
            value.relationship_type = None;
            value.relationship_label = None;
            value.relationship_direction = None;

            values.push(value);
        }

        if values.is_empty() {
            continue;
        }

        field.values = values;
        normalized_fields.push(field);
    }

    normalized_fields
}

fn normalize_character_field_evidence_chapter(
    value: &mut StoryExtractionFieldValuePayload,
    chapter_num: i64,
) {
    for evidence in &mut value.evidence {
        evidence.chapter_num = chapter_num;
    }
}

pub(crate) async fn verify_character_field_payloads(
    state: &AppState,
    field_input: Option<&DraftExtractionInput>,
    identity: &CharacterIdentity,
    character_json: &str,
    fields: Vec<StoryExtractionFieldPayload>,
) -> (Vec<StoryExtractionFieldPayload>, serde_json::Value) {
    let Some(field_input) = field_input else {
        return (
            fields,
            json!({
                "mode": "skipped_no_target_context",
            }),
        );
    };

    let mut verified_fields = Vec::new();
    let mut reports = Vec::new();

    for mut field in fields {
        let mut verified_values = Vec::new();
        for value in field.values {
            let field_value_json = serde_json::to_string(&json!({
                "field_key": &field.field_key,
                "field_label": &field.field_label,
                "value": &value.value,
                "confidence": value.confidence,
                "evidence": &value.evidence,
            }))
            .unwrap_or_else(|_| "{}".to_string());
            let prompt = build_character_field_value_verification_prompt(
                field_input,
                character_json,
                &field_value_json,
            );
            let verification_result = call_local_json_array::<CharacterFieldValueVerification>(
                state,
                &prompt,
                CHARACTER_FIELD_VALUE_VERIFICATION_MAX_TOKENS,
            )
            .await;

            match verification_result {
                Ok((verifications, response)) => {
                    let verification = verifications.into_iter().next();
                    let accepted = verification.as_ref().is_some_and(|verification| {
                        field_value_verification_accepts(identity, verification)
                    });
                    reports.push(json!({
                        "field_key": &field.field_key,
                        "field_label": &field.field_label,
                        "value": &value.value,
                        "accepted": accepted,
                        "verification": verification,
                        "response": response,
                    }));

                    if accepted {
                        verified_values.push(value);
                    }
                }
                Err(error) => {
                    reports.push(json!({
                        "field_key": &field.field_key,
                        "field_label": &field.field_label,
                        "value": &value.value,
                        "accepted": false,
                        "mode": "field_value_verification_failed",
                        "error": error.message,
                    }));
                }
            }
        }

        if verified_values.is_empty() {
            continue;
        }
        field.values = verified_values;
        verified_fields.push(field);
    }

    (
        verified_fields,
        json!({
            "mode": "field_value_verification",
            "checks": reports,
        }),
    )
}

fn field_value_verification_accepts(
    identity: &CharacterIdentity,
    verification: &CharacterFieldValueVerification,
) -> bool {
    verification.accepted
        && verification.confidence.unwrap_or(0.0)
            >= CHARACTER_FIELD_VALUE_VERIFICATION_MIN_CONFIDENCE
        && field_value_semantic_class_is_allowed(&verification.semantic_class)
        && field_value_verification_owner_matches_identity(identity, verification)
}

fn field_value_semantic_class_is_allowed(semantic_class: &str) -> bool {
    matches!(
        normalize_ascii_snake_key(semantic_class).as_str(),
        "physical_appearance" | "clothing" | "age_or_build" | "appearance"
    )
}

fn field_value_verification_owner_matches_identity(
    identity: &CharacterIdentity,
    verification: &CharacterFieldValueVerification,
) -> bool {
    let Some(owner_name) = verification
        .owner_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return true;
    };

    character_identity_surfaces(identity)
        .into_iter()
        .any(|(surface, _)| normalized_text_key(&surface) == normalized_text_key(owner_name))
}

fn strip_character_field_markers_from_evidence(value: &mut StoryExtractionFieldValuePayload) {
    for evidence in &mut value.evidence {
        if let Some(quote) = &mut evidence.quote {
            if quote.contains("[[") || quote.contains("]]") {
                *quote = quote.replace("[[", "").replace("]]", "");
            }
        }
    }
}

fn normalize_character_minimal_field(field_key: &str) -> Option<(&'static str, &'static str)> {
    match normalize_ascii_snake_key(field_key).as_str() {
        "appearance" => Some(("appearance", "Ngoại hình")),
        _ => None,
    }
}

fn character_identity_value_keys(
    identity: &CharacterIdentity,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    keys.insert(normalized_text_key(&identity.name));

    for alias in &identity.aliases {
        keys.insert(normalized_text_key(&alias.text));
    }

    keys.retain(|key| !key.is_empty());
    keys
}

fn other_character_value_keys(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();

    for record in db_records {
        if services::analysis_document::view_record_matches_identity(record, identity) {
            continue;
        }
        keys.insert(normalized_text_key(&record.display_name));
        for alias in services::analysis_document::aliases_from_record(record) {
            keys.insert(normalized_text_key(&alias.text));
        }
    }

    for record in &working_document.records {
        if services::analysis_document::payload_record_matches_identity(record, identity)
            || normalized_text_key(&record.display_name) == normalized_text_key(&identity.name)
        {
            continue;
        }
        keys.insert(normalized_text_key(&record.display_name));
        for alias in services::analysis_document::aliases_from_payload_record(record) {
            keys.insert(normalized_text_key(&alias.text));
        }
    }

    keys.retain(|key| !key.is_empty());
    keys
}

fn field_value_has_quoted_evidence(value: &StoryExtractionFieldValuePayload) -> bool {
    value.evidence.iter().any(|evidence| {
        evidence
            .quote
            .as_deref()
            .is_some_and(|quote| !quote.trim().is_empty())
    })
}

fn field_value_has_target_marker(value: &StoryExtractionFieldValuePayload) -> bool {
    value.evidence.iter().any(|evidence| {
        evidence.quote.as_deref().is_some_and(|quote| {
            let quote = quote.trim();
            quote.contains("[[") && quote.contains("]]")
        })
    })
}

fn field_value_has_minimum_confidence(value: &StoryExtractionFieldValuePayload) -> bool {
    value
        .confidence
        .is_some_and(|confidence| confidence >= CHARACTER_FIELD_VALUE_MIN_CONFIDENCE)
}

fn field_value_is_grounded_in_quoted_evidence(value: &StoryExtractionFieldValuePayload) -> bool {
    let value_tokens = normalized_field_value_tokens(&value.value);
    if value_tokens.is_empty() {
        return false;
    }

    value.evidence.iter().any(|evidence| {
        let Some(quote) = evidence.quote.as_deref() else {
            return false;
        };
        let quote_tokens = normalized_field_value_tokens(quote);
        if quote_tokens.is_empty() {
            return false;
        }

        let overlap_count = value_tokens
            .iter()
            .filter(|token| quote_tokens.contains(*token))
            .count();

        if value_tokens.len() <= 2 {
            overlap_count == value_tokens.len()
        } else {
            overlap_count * 5 >= value_tokens.len() * 3
        }
    })
}

fn normalized_field_value_tokens(value: &str) -> std::collections::HashSet<String> {
    normalized_folded_text_key(value)
        .split('_')
        .filter(|token| token.chars().count() >= 2)
        .map(str::to_string)
        .collect()
}
