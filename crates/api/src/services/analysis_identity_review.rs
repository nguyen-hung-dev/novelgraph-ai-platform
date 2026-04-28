use crate::services::llm_json::call_local_json_array;
use crate::*;
use serde_json::json;

pub(crate) fn character_identity_creation_review_candidates(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
    chunk_text: &str,
) -> Option<Vec<CharacterIdentityMergeCandidate>> {
    let mut candidates = std::collections::HashMap::new();

    for mut candidate in character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if !identity_creation_candidate_is_relevant(
            identity,
            &candidate_identity,
            chunk_text,
            score,
        ) {
            continue;
        }
        candidate.score = score;
        push_character_merge_review_candidate(&mut candidates, candidate);
    }

    for record in db_records {
        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if !identity_creation_candidate_is_relevant(
            identity,
            &candidate_identity,
            chunk_text,
            score,
        ) {
            continue;
        }

        push_character_merge_review_candidate(
            &mut candidates,
            CharacterIdentityMergeCandidate {
                target_key: record
                    .entity_key
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                display_name: candidate_identity.name,
                aliases: candidate_identity.aliases,
                score,
                source: "db_creation_review".to_string(),
                chapter_num: Some(record.chapter_num),
            },
        );
    }

    for record in &working_document.records {
        if record.group_key != "character" {
            continue;
        }

        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_payload_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if !identity_creation_candidate_is_relevant(
            identity,
            &candidate_identity,
            chunk_text,
            score,
        ) {
            continue;
        }

        push_character_merge_review_candidate(
            &mut candidates,
            CharacterIdentityMergeCandidate {
                target_key: record
                    .entity_key
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                display_name: candidate_identity.name,
                aliases: candidate_identity.aliases,
                score,
                source: "working_creation_review".to_string(),
                chapter_num: Some(working_document.chapter_num),
            },
        );
    }

    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.chapter_num.cmp(&right.chapter_num))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    candidates.truncate(12);
    Some(candidates)
}

fn identity_creation_candidate_is_relevant(
    observed: &CharacterIdentity,
    candidate: &CharacterIdentity,
    chunk_text: &str,
    score: f64,
) -> bool {
    score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
        || character_identity_any_surface_appears(candidate, chunk_text)
        || observed_identity_contains_candidate_surface(observed, candidate)
}

fn character_identity_any_surface_appears(identity: &CharacterIdentity, text: &str) -> bool {
    character_identity_surfaces(identity)
        .into_iter()
        .any(|(surface, _)| {
            !find_surface_occurrences(text, &surface, "identity_review", false).is_empty()
        })
}

fn observed_identity_contains_candidate_surface(
    observed: &CharacterIdentity,
    candidate: &CharacterIdentity,
) -> bool {
    let observed_key = normalized_folded_text_key(&observed.name);
    if observed_key.is_empty() {
        return false;
    }

    character_identity_surfaces(candidate)
        .into_iter()
        .any(|(surface, _)| {
            let candidate_key = normalized_folded_text_key(&surface);
            !candidate_key.is_empty()
                && candidate_key != observed_key
                && (observed_key.starts_with(&(candidate_key.clone() + "_"))
                    || observed_key.ends_with(&("_".to_string() + &candidate_key)))
        })
}

pub(crate) fn observed_identity_is_known_name_phrase(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> bool {
    let observed_key = normalized_folded_text_key(&identity.name);
    if observed_key.is_empty() {
        return false;
    }

    let mut known_surfaces = Vec::new();
    known_surfaces.extend(db_records.iter().map(|record| record.display_name.clone()));
    known_surfaces.extend(
        working_document
            .records
            .iter()
            .filter(|record| record.group_key == "character")
            .map(|record| record.display_name.clone()),
    );

    known_surfaces.into_iter().any(|surface| {
        let known_key = normalized_folded_text_key(&surface);
        !known_key.is_empty()
            && known_key != observed_key
            && (observed_key.starts_with(&(known_key.clone() + "_"))
                || observed_key.ends_with(&("_".to_string() + &known_key)))
    })
}

pub(crate) fn find_character_merge_review_candidate(
    identity: &CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    db_aliases: &[StoryCharacterAliasView],
    working_document: &StoryExtractionDocument,
) -> Option<CharacterIdentityMergeCandidate> {
    let mut candidates = std::collections::HashMap::new();

    for mut candidate in character_alias_map_candidates(db_aliases) {
        let candidate_identity = CharacterIdentity {
            name: candidate.display_name.clone(),
            aliases: candidate.aliases.clone(),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
            && score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE
        {
            candidate.score = score;
            push_character_merge_review_candidate(&mut candidates, candidate);
        }
    }

    for record in db_records {
        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
            && score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE
        {
            push_character_merge_review_candidate(
                &mut candidates,
                CharacterIdentityMergeCandidate {
                    target_key: record
                        .entity_key
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                    display_name: candidate_identity.name,
                    aliases: candidate_identity.aliases,
                    score,
                    source: "db".to_string(),
                    chapter_num: Some(record.chapter_num),
                },
            );
        }
    }

    for record in &working_document.records {
        if record.group_key != "character" {
            continue;
        }

        let candidate_identity = CharacterIdentity {
            name: record.display_name.clone(),
            aliases: services::analysis_document::aliases_from_payload_record(record),
        };
        let score = character_identity_candidate_score(identity, &candidate_identity);
        if score >= CHARACTER_CANONICAL_REVIEW_MIN_SCORE
            && score < CHARACTER_CANONICAL_AUTO_MERGE_SCORE
        {
            push_character_merge_review_candidate(
                &mut candidates,
                CharacterIdentityMergeCandidate {
                    target_key: record
                        .entity_key
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| normalize_ascii_snake_key(&record.display_name)),
                    display_name: candidate_identity.name,
                    aliases: candidate_identity.aliases,
                    score,
                    source: "working_document".to_string(),
                    chapter_num: Some(working_document.chapter_num),
                },
            );
        }
    }

    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });

    let best = candidates.first()?;
    if candidates
        .get(1)
        .is_some_and(|next| best.score - next.score < CHARACTER_CANONICAL_REVIEW_SCORE_GAP)
    {
        return None;
    }

    Some(best.clone())
}

pub(crate) fn character_alias_map_candidates(
    db_aliases: &[StoryCharacterAliasView],
) -> Vec<CharacterIdentityMergeCandidate> {
    let mut candidates =
        std::collections::HashMap::<String, CharacterIdentityMergeCandidate>::new();

    for alias in db_aliases {
        let target_key = alias.entity_key.trim();
        if target_key.is_empty() {
            continue;
        }

        let entry = candidates.entry(target_key.to_string()).or_insert_with(|| {
            CharacterIdentityMergeCandidate {
                target_key: target_key.to_string(),
                display_name: alias.display_name.clone(),
                aliases: Vec::new(),
                score: 0.0,
                source: "alias_map".to_string(),
                chapter_num: Some(alias.first_chapter_num),
            }
        });

        if alias.alias_type == "canonical_name" {
            entry.display_name = alias.alias_text.clone();
        } else {
            push_character_alias_if_valid(
                &mut entry.aliases,
                CharacterAlias {
                    text: alias.alias_text.clone(),
                    alias_type: alias.alias_type.clone(),
                    alias_label: alias.alias_label.clone(),
                    is_primary: alias.confidence.unwrap_or(0.0) >= 1.0,
                    evidence: alias.evidence.clone(),
                },
                &entry.display_name,
            );
        }

        if entry
            .chapter_num
            .is_none_or(|chapter_num| alias.first_chapter_num < chapter_num)
        {
            entry.chapter_num = Some(alias.first_chapter_num);
        }
    }

    candidates.into_values().collect()
}

pub(crate) fn identity_from_alias_map_candidate(
    candidate: CharacterIdentityMergeCandidate,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) -> CharacterIdentity {
    let mut identity = CharacterIdentity {
        name: candidate.display_name,
        aliases: candidate.aliases,
    };
    merge_observed_identity_aliases(&mut identity, observed_identity, known_name_keys);
    identity
}

fn push_character_merge_review_candidate(
    candidates: &mut std::collections::HashMap<String, CharacterIdentityMergeCandidate>,
    candidate: CharacterIdentityMergeCandidate,
) {
    if let Some(existing) = candidates.get_mut(&candidate.target_key) {
        if candidate.score > existing.score {
            existing.score = candidate.score;
        }
        if existing.chapter_num.is_none_or(|chapter_num| {
            candidate
                .chapter_num
                .is_some_and(|candidate_chapter_num| candidate_chapter_num < chapter_num)
        }) {
            existing.chapter_num = candidate.chapter_num;
        }
        for alias in candidate.aliases {
            push_character_alias_if_valid(&mut existing.aliases, alias, &existing.display_name);
        }
        return;
    }

    candidates.insert(candidate.target_key.clone(), candidate);
}

pub(crate) async fn confirm_character_identity_merge(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: &CharacterIdentity,
    candidate: &CharacterIdentityMergeCandidate,
) -> (CharacterIdentityMergeDecision, serde_json::Value) {
    let observed_identity_json =
        serde_json::to_string(identity).unwrap_or_else(|_| "{}".to_string());
    let candidate_identity_json =
        serde_json::to_string(candidate).unwrap_or_else(|_| "{}".to_string());
    let prompt = build_character_identity_merge_confirmation_prompt(
        chunk_input,
        &observed_identity_json,
        &candidate_identity_json,
    );

    match call_local_json_array::<CharacterIdentityMergeDecision>(
        state,
        &prompt,
        CHARACTER_IDENTITY_MERGE_CONFIRMATION_MAX_TOKENS,
    )
    .await
    {
        Ok((decisions, response)) => (
            decisions.into_iter().next().unwrap_or_else(|| {
                character_identity_merge_decision("create_new", 0.0, "LLM trả mảng rỗng.")
            }),
            json!(response),
        ),
        Err(error) => (
            character_identity_merge_decision(
                "create_new",
                0.0,
                "Không parse được JSON xác nhận merge; giữ nhân vật riêng để tránh nhập sai.",
            ),
            json!({
                "mode": "merge_confirmation_failed_non_blocking",
                "error": error.message,
            }),
        ),
    }
}

pub(crate) async fn confirm_character_identity_creation(
    state: &AppState,
    chunk_input: &DraftExtractionInput,
    identity: &CharacterIdentity,
    candidates: &[CharacterIdentityMergeCandidate],
) -> (CharacterIdentityCreationDecision, serde_json::Value) {
    let observed_identity_json =
        serde_json::to_string(identity).unwrap_or_else(|_| "{}".to_string());
    let candidates_json = serde_json::to_string(candidates).unwrap_or_else(|_| "[]".to_string());
    let prompt = build_character_identity_creation_review_prompt(
        chunk_input,
        &observed_identity_json,
        &candidates_json,
    );

    match call_local_json_array::<CharacterIdentityCreationDecision>(
        state,
        &prompt,
        CHARACTER_IDENTITY_CREATION_REVIEW_MAX_TOKENS,
    )
    .await
    {
        Ok((decisions, response)) => (
            decisions.into_iter().next().unwrap_or_else(|| {
                character_identity_creation_decision(
                    "create_new",
                    None,
                    None,
                    0.0,
                    "LLM trả mảng rỗng.",
                )
            }),
            json!(response),
        ),
        Err(error) => (
            character_identity_creation_decision(
                "create_new",
                None,
                None,
                0.0,
                "Không parse được JSON kiểm tra nhân vật mới; giữ nhân vật riêng để tránh nhập sai.",
            ),
            json!({
                "mode": "identity_creation_review_failed_non_blocking",
                "error": error.message,
            }),
        ),
    }
}

fn character_identity_creation_decision(
    action: &str,
    target_key: Option<String>,
    target_name: Option<String>,
    confidence: f64,
    reason: &str,
) -> CharacterIdentityCreationDecision {
    CharacterIdentityCreationDecision {
        action: action.to_string(),
        target_key,
        target_name,
        confidence: Some(confidence),
        reason: Some(reason.to_string()),
        evidence: Vec::new(),
    }
}

fn character_identity_merge_decision(
    action: &str,
    confidence: f64,
    reason: &str,
) -> CharacterIdentityMergeDecision {
    CharacterIdentityMergeDecision {
        action: action.to_string(),
        confidence: Some(confidence),
        reason: Some(reason.to_string()),
    }
}

pub(crate) fn normalize_character_identity_merge_action(action: &str) -> &'static str {
    match normalize_ascii_snake_key(action).as_str() {
        "merge_existing" | "merge" | "merge_into_existing" => "merge_existing",
        "ignore" | "skip" => "ignore",
        _ => "create_new",
    }
}

pub(crate) fn normalize_character_identity_creation_action(action: &str) -> &'static str {
    match normalize_ascii_snake_key(action).as_str() {
        "merge_existing" | "merge" | "merge_into_existing" => "merge_existing",
        "reject" | "ignore" | "skip" => "reject",
        _ => "create_new",
    }
}

pub(crate) fn find_creation_review_target_candidate<'a>(
    decision: &CharacterIdentityCreationDecision,
    candidates: &'a [CharacterIdentityMergeCandidate],
) -> Option<&'a CharacterIdentityMergeCandidate> {
    if let Some(target_key) = decision
        .target_key
        .as_deref()
        .map(normalize_ascii_snake_key)
        .filter(|value| !value.is_empty())
    {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| normalize_ascii_snake_key(&candidate.target_key) == target_key)
        {
            return Some(candidate);
        }
    }

    let target_name = decision
        .target_name
        .as_deref()
        .map(normalized_text_key)
        .filter(|value| !value.is_empty())?;

    candidates.iter().find(|candidate| {
        normalized_text_key(&candidate.display_name) == target_name
            || candidate
                .aliases
                .iter()
                .any(|alias| normalized_text_key(&alias.text) == target_name)
    })
}

pub(crate) fn character_identity_candidate_score(
    identity: &CharacterIdentity,
    candidate: &CharacterIdentity,
) -> f64 {
    let identity_surfaces = character_resolution_surface_items(identity);
    let candidate_surfaces = character_resolution_surface_items(candidate);
    if identity_surfaces.is_empty() || candidate_surfaces.is_empty() {
        return 0.0;
    }

    let mut best_score: f64 = 0.0;
    for left in &identity_surfaces {
        for right in &candidate_surfaces {
            if left.key == right.key {
                if !left.is_canonical || !right.is_alias {
                    return 1.0;
                }

                best_score = best_score.max(CHARACTER_CANONICAL_STORED_ALIAS_NAME_MATCH_SCORE);
                continue;
            }

            let mut score = character_surface_similarity_score(&left.text, &right.text);
            if left.is_canonical && right.is_alias {
                score = score.min(CHARACTER_CANONICAL_STORED_ALIAS_NAME_MATCH_SCORE);
            }
            if score > best_score {
                best_score = score;
            }
        }
    }

    best_score
}

#[derive(Debug, Clone)]
struct CharacterResolutionSurface {
    text: String,
    key: String,
    is_canonical: bool,
    is_alias: bool,
}

fn character_resolution_surface_items(
    identity: &CharacterIdentity,
) -> Vec<CharacterResolutionSurface> {
    let mut surfaces = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let name = clean_character_surface(&identity.name);
    let name_key = normalized_folded_text_key(&name);
    if is_strong_character_resolution_key(&name_key) && seen.insert(name_key.clone()) {
        surfaces.push(CharacterResolutionSurface {
            text: name,
            key: name_key,
            is_canonical: true,
            is_alias: false,
        });
    }

    for alias in &identity.aliases {
        let alias_type = normalize_character_alias_type(&alias.alias_type);
        if !is_persistable_character_alias_type(&alias_type) {
            continue;
        }

        let surface = clean_character_surface(&alias.text);
        let key = normalized_folded_text_key(&surface);
        if is_strong_character_resolution_key(&key) && seen.insert(key.clone()) {
            surfaces.push(CharacterResolutionSurface {
                text: surface,
                key,
                is_canonical: false,
                is_alias: true,
            });
        }
    }

    surfaces
}

fn character_surface_similarity_score(left: &str, right: &str) -> f64 {
    let left_key = normalized_folded_text_key(left);
    let right_key = normalized_folded_text_key(right);
    if !is_strong_character_resolution_key(&left_key)
        || !is_strong_character_resolution_key(&right_key)
    {
        return 0.0;
    }
    if left_key == right_key {
        return 1.0;
    }

    let edit_score = levenshtein_similarity_score(&left_key, &right_key);
    let token_score = token_dice_score(&left_key, &right_key);
    let substring_score = character_substring_similarity_score(&left_key, &right_key);

    edit_score.max(token_score * 0.9).max(substring_score)
}

fn character_substring_similarity_score(left_key: &str, right_key: &str) -> f64 {
    if left_key == right_key {
        return 1.0;
    }

    let left_token_count = character_resolution_token_count(left_key);
    let right_token_count = character_resolution_token_count(right_key);
    let min_len = left_key.chars().count().min(right_key.chars().count());
    if min_len < 7 || left_token_count < 2 || right_token_count < 2 {
        return 0.0;
    }

    if left_key.contains(right_key) || right_key.contains(left_key) {
        return 0.93;
    }

    0.0
}

fn token_dice_score(left_key: &str, right_key: &str) -> f64 {
    let left_tokens = left_key
        .split('_')
        .filter(|token| !token.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let right_tokens = right_key
        .split('_')
        .filter(|token| !token.is_empty())
        .collect::<std::collections::HashSet<_>>();

    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let shared_count = left_tokens.intersection(&right_tokens).count();
    (2.0 * shared_count as f64) / (left_tokens.len() + right_tokens.len()) as f64
}

fn levenshtein_similarity_score(left: &str, right: &str) -> f64 {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let max_len = left_chars.len().max(right_chars.len());
    if max_len == 0 {
        return 0.0;
    }

    let distance = levenshtein_distance(&left_chars, &right_chars);
    1.0 - (distance as f64 / max_len as f64)
}

fn levenshtein_distance(left: &[char], right: &[char]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right.iter().enumerate() {
            let insert_cost = current[right_index] + 1;
            let delete_cost = previous[right_index + 1] + 1;
            let replace_cost = previous[right_index] + usize::from(left_char != right_char);
            current[right_index + 1] = insert_cost.min(delete_cost).min(replace_cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

fn is_strong_character_resolution_key(key: &str) -> bool {
    let char_count = key.chars().filter(|ch| *ch != '_').count();
    if char_count < 4 {
        return false;
    }

    character_resolution_token_count(key) >= 2 || char_count >= 6
}

fn character_resolution_token_count(key: &str) -> usize {
    key.split('_').filter(|token| !token.is_empty()).count()
}

pub(crate) fn identity_from_db_record(
    record: &StoryExtractionRecordView,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) -> CharacterIdentity {
    let mut identity = CharacterIdentity {
        name: record.display_name.clone(),
        aliases: services::analysis_document::aliases_from_record(record),
    };
    merge_observed_identity_aliases(&mut identity, observed_identity, known_name_keys);
    identity
}

pub(crate) fn identity_from_payload_record(
    record: &StoryExtractionRecordPayload,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) -> CharacterIdentity {
    let mut identity = CharacterIdentity {
        name: record.display_name.clone(),
        aliases: services::analysis_document::aliases_from_payload_record(record),
    };
    merge_observed_identity_aliases(&mut identity, observed_identity, known_name_keys);
    identity
}

pub(crate) fn merge_observed_identity_aliases(
    target: &mut CharacterIdentity,
    observed_identity: Option<&CharacterIdentity>,
    known_name_keys: &std::collections::HashSet<String>,
) {
    let Some(observed_identity) = observed_identity else {
        return;
    };

    if normalized_text_key(&target.name) != normalized_text_key(&observed_identity.name) {
        let observed_name_key = normalized_text_key(&observed_identity.name);
        if !known_name_keys.contains(&observed_name_key) {
            push_character_alias_if_valid(
                &mut target.aliases,
                CharacterAlias {
                    text: observed_identity.name.clone(),
                    alias_type: "other_alias".to_string(),
                    alias_label: "Tên gọi khác".to_string(),
                    is_primary: false,
                    evidence: Vec::new(),
                },
                &target.name,
            );
        }
    }

    for alias in &observed_identity.aliases {
        if !is_persistable_character_alias_type(&alias.alias_type) {
            continue;
        }
        if known_name_keys.contains(&normalized_text_key(&alias.text))
            && normalized_text_key(&alias.text) != normalized_text_key(&target.name)
        {
            continue;
        }
        push_character_alias_if_valid(&mut target.aliases, alias.clone(), &target.name);
    }
}

pub(crate) fn sanitize_new_character_identity(
    identity: CharacterIdentity,
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> CharacterIdentity {
    let blocked_alias_keys = known_character_name_keys(db_records, working_document)
        .into_iter()
        .filter(|key| *key != normalized_text_key(&identity.name))
        .collect::<std::collections::HashSet<_>>();

    let mut sanitized = CharacterIdentity {
        name: identity.name,
        aliases: Vec::new(),
    };

    for alias in identity.aliases {
        let alias_type = normalize_character_alias_type(&alias.alias_type);
        if !is_persistable_character_alias_type(&alias_type) {
            continue;
        }
        if blocked_alias_keys.contains(&normalized_text_key(&alias.text)) {
            continue;
        }
        push_character_alias_if_valid(&mut sanitized.aliases, alias, &sanitized.name);
    }

    sanitized
}

pub(crate) fn known_character_name_keys(
    db_records: &[StoryExtractionRecordView],
    working_document: &StoryExtractionDocument,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();

    for record in db_records {
        keys.insert(normalized_text_key(&record.display_name));
    }
    for record in &working_document.records {
        keys.insert(normalized_text_key(&record.display_name));
    }

    keys.retain(|key| !key.is_empty());
    keys
}

pub(crate) fn merge_character_identity_into_list(
    identities: &mut Vec<CharacterIdentity>,
    source: CharacterIdentity,
) {
    if let Some(target) = identities
        .iter_mut()
        .find(|identity| normalized_text_key(&identity.name) == normalized_text_key(&source.name))
    {
        for alias in source.aliases {
            push_character_alias_if_valid(&mut target.aliases, alias, &target.name);
        }
        return;
    }

    identities.push(source);
}
