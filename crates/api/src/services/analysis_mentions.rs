use crate::services::llm_json::call_local_json_array;
use crate::*;
use serde_json::json;

pub(crate) async fn scan_character_mentions_with_backend(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: &CharacterIdentity,
    character_json: &str,
    chapter_text: &str,
) -> Result<(Vec<StoryCharacterMention>, serde_json::Value), ApiError> {
    let surfaces = character_identity_surfaces(identity);
    let mut scanned = Vec::new();
    for (surface, mention_type) in &surfaces {
        let ambiguous = is_ambiguous_character_surface(surface);
        scanned.extend(find_surface_occurrences(
            chapter_text,
            surface,
            mention_type,
            ambiguous,
        ));
    }

    let scanned_count = scanned.len();
    let selected = select_non_overlapping_occurrences(scanned);
    let mut mentions = Vec::new();
    let mut occurrence_reports = Vec::new();
    let mut ambiguous_groups = Vec::<(String, Vec<ScannedCharacterOccurrence>)>::new();
    let mut ambiguous_group_indexes = std::collections::HashMap::<String, usize>::new();

    for occurrence in selected {
        if occurrence.ambiguous {
            let group_key = character_occurrence_group_key(&occurrence);
            if let Some(index) = ambiguous_group_indexes.get(&group_key).copied() {
                ambiguous_groups[index].1.push(occurrence);
            } else {
                ambiguous_group_indexes.insert(group_key.clone(), ambiguous_groups.len());
                ambiguous_groups.push((group_key, vec![occurrence]));
            }
        } else {
            occurrence_reports.push(json!({
                "mode": "direct_boundary_scan",
                "occurrence": occurrence.clone(),
                "confirmed": true,
            }));
            mentions.push(scanned_occurrence_to_mention(occurrence));
        }
    }

    for (group_key, occurrences) in ambiguous_groups {
        let occurrence_count = occurrences.len();
        let surface_text = occurrences
            .first()
            .map(|occurrence| occurrence.text.clone())
            .unwrap_or_default();
        let samples = sample_character_occurrences_for_confirmation(&occurrences);
        let mut sample_reports = Vec::new();
        let mut confirmed_samples = Vec::new();
        let mut rejected_sample_count = 0usize;

        for occurrence in &samples {
            let (confirmed, confirmation, response) = confirm_character_occurrence_with_llm(
                state,
                chunk_input,
                character_json,
                chapter_text,
                occurrence,
            )
            .await?;

            sample_reports.push(json!({
                "occurrence": occurrence,
                "confirmed": confirmed,
                "confirmation": confirmation,
                "response": response,
            }));

            if confirmed {
                confirmed_samples.push(occurrence.clone());
            } else {
                rejected_sample_count += 1;
            }
        }

        let accept_all = !samples.is_empty()
            && rejected_sample_count == 0
            && surface_sample_confirmation_can_accept_all(&surface_text);

        if accept_all {
            for occurrence in occurrences {
                mentions.push(scanned_occurrence_to_mention(occurrence));
            }
        } else {
            for occurrence in confirmed_samples {
                mentions.push(scanned_occurrence_to_mention(occurrence));
            }
        }

        let confirmed_sample_count = sample_reports
            .iter()
            .filter(|report| {
                report
                    .get("confirmed")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
            })
            .count();

        occurrence_reports.push(json!({
            "mode": "llm_surface_sample_confirmation",
            "surface_key": group_key,
            "surface_text": surface_text,
            "occurrence_count": occurrence_count,
            "sample_limit": CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT,
            "sample_count": samples.len(),
            "confirmed_sample_count": confirmed_sample_count,
            "rejected_sample_count": rejected_sample_count,
            "decision": if accept_all { "accept_all_occurrences" } else { "accept_confirmed_samples_only" },
            "samples": sample_reports,
        }));
    }

    mentions.sort_by(|left, right| {
        left.start_char
            .cmp(&right.start_char)
            .then_with(|| right.end_char.cmp(&left.end_char))
    });

    let report = json!({
        "mode": "backend_surface_scan_with_sampled_llm_confirmation",
        "surface_count": surfaces.len(),
        "scanned_occurrence_count": scanned_count,
        "confirmed_mention_count": mentions.len(),
        "confirmation_sample_limit": CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT,
        "occurrences": occurrence_reports,
    });

    Ok((mentions, report))
}

async fn confirm_character_occurrence_with_llm(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    character_json: &str,
    chapter_text: &str,
    occurrence: &ScannedCharacterOccurrence,
) -> Result<
    (
        bool,
        Option<CharacterOccurrenceConfirmation>,
        serde_json::Value,
    ),
    ApiError,
> {
    let context = character_occurrence_context(
        chapter_text,
        occurrence.start_char,
        occurrence.end_char,
        CHARACTER_OCCURRENCE_CONTEXT_CHARS,
    );
    let parent_prior_context = chunk_input.prior_context.as_deref().unwrap_or("").trim();
    let confirmation_prior_context = if parent_prior_context.is_empty() {
        "Backend đã exact-scan surface bằng boundary ký tự trước khi hỏi xác nhận.".to_string()
    } else {
        format!(
            "{parent_prior_context}\n\nBackend đã exact-scan surface bằng boundary ký tự trước khi hỏi xác nhận."
        )
    };
    let confirmation_input = DraftExtractionInput {
        chapter_num: chunk_input.chapter_num,
        title: chunk_input.title.clone(),
        source_language: chunk_input.source_language.clone(),
        text: context,
        prior_context: Some(confirmation_prior_context),
    };
    let prompt = build_character_occurrence_confirmation_prompt(
        &confirmation_input,
        character_json,
        &occurrence.text,
    );
    let (confirmations, response) = call_local_json_array::<CharacterOccurrenceConfirmation>(
        state,
        &prompt,
        CHARACTER_OCCURRENCE_CONFIRMATION_MAX_TOKENS,
    )
    .await?;
    let confirmation = confirmations.into_iter().next();
    let confirmed = confirmation.as_ref().is_some_and(|item| {
        item.is_character_mention
            && item.confidence.unwrap_or(1.0) >= CHARACTER_OCCURRENCE_CONFIRMATION_MIN_CONFIDENCE
    });

    Ok((confirmed, confirmation, json!(response)))
}

fn character_occurrence_group_key(occurrence: &ScannedCharacterOccurrence) -> String {
    format!(
        "{}:{}",
        occurrence.mention_type,
        normalized_text_key(&occurrence.text)
    )
}

fn sample_character_occurrences_for_confirmation(
    occurrences: &[ScannedCharacterOccurrence],
) -> Vec<ScannedCharacterOccurrence> {
    if occurrences.len() <= CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT {
        return occurrences.to_vec();
    }

    let last_index = occurrences.len() - 1;
    let sample_slots = CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT.saturating_sub(1);
    let mut samples = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sample_index in 0..CHARACTER_OCCURRENCE_CONFIRMATION_SAMPLE_LIMIT {
        let occurrence_index = if sample_slots == 0 {
            0
        } else {
            sample_index * last_index / sample_slots
        };

        if seen.insert(occurrence_index) {
            samples.push(occurrences[occurrence_index].clone());
        }
    }

    samples
}

fn surface_sample_confirmation_can_accept_all(surface: &str) -> bool {
    let tokens = surface.split_whitespace().collect::<Vec<_>>();
    let char_count = surface.chars().filter(|ch| ch.is_alphanumeric()).count();

    char_count >= 5 && tokens.len() >= 2
}

fn scanned_occurrence_to_mention(occurrence: ScannedCharacterOccurrence) -> StoryCharacterMention {
    StoryCharacterMention {
        text: occurrence.text,
        start_char: occurrence.start_char,
        end_char: occurrence.end_char,
        mention_type: Some(occurrence.mention_type),
    }
}

fn character_occurrence_context(
    chapter_text: &str,
    start_char: i64,
    end_char: i64,
    radius: i64,
) -> String {
    let chars = chapter_text.chars().collect::<Vec<_>>();
    let context_start = start_char.saturating_sub(radius).max(0) as usize;
    let context_end = (end_char + radius).max(0).min(chars.len() as i64) as usize;
    let mention_start = start_char.max(0) as usize;
    let mention_end = end_char.max(0).min(chars.len() as i64) as usize;

    let mut context = String::new();
    for (index, ch) in chars
        .iter()
        .enumerate()
        .take(context_end)
        .skip(context_start)
    {
        if index == mention_start {
            context.push_str("[[");
        }
        context.push(*ch);
        if index + 1 == mention_end {
            context.push_str("]]");
        }
    }
    context
}
