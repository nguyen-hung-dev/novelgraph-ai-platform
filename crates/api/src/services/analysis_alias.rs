use crate::*;
use serde::Serialize;
use serde_json::json;

pub(crate) fn known_alias_map_identities_for_chunk(
    chunk_text: &str,
    db_aliases: &[StoryCharacterAliasView],
) -> Vec<CharacterIdentity> {
    let mut identities = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for candidate in services::analysis_identity_review::character_alias_map_candidates(db_aliases)
    {
        let mut surfaces = Vec::new();
        surfaces.push(candidate.display_name.clone());
        surfaces.extend(candidate.aliases.iter().map(|alias| alias.text.clone()));

        let appears_in_chunk = surfaces.iter().any(|surface| {
            !surface.trim().is_empty()
                && !find_surface_occurrences(chunk_text, surface, "known_alias", false).is_empty()
        });
        if !appears_in_chunk || !seen.insert(candidate.target_key.clone()) {
            continue;
        }

        identities.push(CharacterIdentity {
            name: candidate.display_name,
            aliases: candidate.aliases,
        });
    }

    identities
}

pub(crate) fn apply_character_alias_ownerships(
    identities: &mut Vec<CharacterIdentity>,
    ownerships: Vec<CharacterAliasOwnership>,
    chapter_num: i64,
) -> Vec<serde_json::Value> {
    let mut applications = Vec::new();
    let mut remove_identity_keys = std::collections::HashSet::new();

    for ownership in ownerships {
        let confidence = ownership.confidence.unwrap_or(0.0);
        if confidence < CHARACTER_ALIAS_OWNERSHIP_MIN_CONFIDENCE {
            continue;
        }

        let owner_name = clean_character_surface(&ownership.owner_name);
        let alias_text = clean_character_surface(&ownership.alias_text);
        let owner_key = normalized_text_key(&owner_name);
        let alias_key = normalized_text_key(&alias_text);
        if owner_key.is_empty()
            || alias_key.is_empty()
            || owner_key == alias_key
            || remove_identity_keys.contains(&owner_key)
        {
            continue;
        }

        let Some(mut owner_index) =
            find_character_identity_index_by_surface(identities, &owner_name)
        else {
            continue;
        };
        if let Some(redirected_owner_index) =
            better_alias_owner_by_surface(identities, owner_index, &alias_text)
        {
            owner_index = redirected_owner_index;
        }
        let target_name = identities[owner_index].name.clone();
        let target_key = normalized_text_key(&target_name);
        if remove_identity_keys.contains(&target_key) {
            continue;
        }

        let alias_type = normalize_character_alias_type(&ownership.alias_type);
        if !is_persistable_character_alias_type(&alias_type) {
            continue;
        }
        let alias_label = normalize_character_alias_label(&alias_type, &ownership.alias_label);
        let evidence = normalize_alias_ownership_evidence(ownership.evidence, chapter_num);
        if !alias_ownership_can_be_applied(identities, owner_index, &alias_text, &evidence) {
            applications.push(json!({
                "mode": "alias_ownership",
                "applied": false,
                "reason": "alias owner is not grounded by evidence",
                "owner_name": target_name,
                "alias_text": alias_text,
                "alias_type": alias_type,
                "alias_label": alias_label,
                "confidence": confidence,
            }));
            continue;
        }

        push_character_alias_if_valid(
            &mut identities[owner_index].aliases,
            CharacterAlias {
                text: alias_text.clone(),
                alias_type: alias_type.clone(),
                alias_label: alias_label.clone(),
                is_primary: confidence >= 0.95,
                evidence,
            },
            &target_name,
        );

        if identities.iter().enumerate().any(|(index, identity)| {
            index != owner_index && normalized_text_key(&identity.name) == alias_key
        }) {
            remove_identity_keys.insert(alias_key.clone());
        }

        applications.push(json!({
            "mode": "alias_ownership",
            "applied": true,
            "owner_name": target_name,
            "alias_text": alias_text,
            "alias_type": alias_type,
            "alias_label": alias_label,
            "confidence": confidence,
        }));
    }

    if !remove_identity_keys.is_empty() {
        identities.retain(|identity| {
            !remove_identity_keys.contains(&normalized_text_key(&identity.name))
        });
    }

    applications
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QuotedAliasCandidateContext {
    surface: String,
    context: String,
    nearby_identity_names: Vec<String>,
    nearest_identity_before_quote: Option<String>,
}

pub(crate) fn quoted_alias_candidate_context(
    identities: &[CharacterIdentity],
    chunk_text: &str,
) -> Vec<QuotedAliasCandidateContext> {
    let mut candidates = Vec::new();

    for span in quoted_alias_spans(chunk_text) {
        let surface = clean_character_surface(&span.text);
        if surface.is_empty() {
            continue;
        }

        let sentence_start = sentence_start_before(chunk_text, span.start_char);
        let sentence_end = sentence_end_after(chunk_text, span.end_char);
        let context = slice_text_by_char_range(chunk_text, sentence_start, sentence_end);
        let nearby_identity_names = identity_names_in_text(identities, &context);
        if nearby_identity_names.is_empty() {
            continue;
        }

        let before_quote = slice_text_by_char_range(chunk_text, sentence_start, span.start_char);
        let nearest_identity_before_quote =
            nearest_identity_before_alias_quote(identities, &before_quote)
                .map(|identity_index| identities[identity_index].name.clone());

        candidates.push(QuotedAliasCandidateContext {
            surface,
            context,
            nearby_identity_names,
            nearest_identity_before_quote,
        });
    }

    candidates
}

fn identity_names_in_text(identities: &[CharacterIdentity], text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for identity in identities {
        let identity_key = normalized_text_key(&identity.name);
        if identity_key.is_empty() || seen.contains(&identity_key) {
            continue;
        }

        let found = character_identity_surfaces(identity)
            .into_iter()
            .any(|(surface, _)| {
                !find_surface_occurrences(text, &surface, "alias_owner_candidate", false).is_empty()
            });
        if found {
            seen.insert(identity_key);
            names.push(identity.name.clone());
        }
    }

    names
}

fn nearest_identity_before_alias_quote(
    identities: &[CharacterIdentity],
    before_quote: &str,
) -> Option<usize> {
    let mut best: Option<(usize, i64)> = None;

    for (identity_index, identity) in identities.iter().enumerate() {
        for (surface, _) in character_identity_surfaces(identity) {
            for occurrence in
                find_surface_occurrences(before_quote, &surface, "alias_owner_candidate", false)
            {
                match best {
                    Some((_, best_end)) if occurrence.end_char <= best_end => {}
                    _ => best = Some((identity_index, occurrence.end_char)),
                }
            }
        }
    }

    best.map(|(identity_index, _)| identity_index)
}

fn sentence_start_before(text: &str, char_index: i64) -> i64 {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = char_index.max(0).min(chars.len() as i64) as usize;
    while index > 0 {
        let previous = chars[index - 1];
        if is_sentence_boundary_for_quote_context(previous) {
            break;
        }
        index -= 1;
    }

    index as i64
}

fn sentence_end_after(text: &str, char_index: i64) -> i64 {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = char_index.max(0).min(chars.len() as i64) as usize;
    while index < chars.len() {
        let current = chars[index];
        index += 1;
        if is_sentence_boundary_for_quote_context(current) {
            break;
        }
    }

    index as i64
}

fn is_sentence_boundary_for_quote_context(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?' | '。' | '！' | '？' | '\n' | '\r')
}

fn slice_text_by_char_range(text: &str, start_char: i64, end_char: i64) -> String {
    let start = start_char.max(0) as usize;
    let end = end_char.max(start_char).max(0) as usize;
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

fn normalize_alias_ownership_evidence(
    evidence: Vec<StoryEvidenceSpan>,
    chapter_num: i64,
) -> Vec<StoryEvidenceSpan> {
    evidence
        .into_iter()
        .map(|span| StoryEvidenceSpan {
            chapter_num,
            start_char: None,
            end_char: None,
            quote: span.quote,
            reason: span.reason,
        })
        .collect()
}

fn alias_ownership_can_be_applied(
    identities: &[CharacterIdentity],
    owner_index: usize,
    alias_text: &str,
    evidence: &[StoryEvidenceSpan],
) -> bool {
    if evidence.is_empty() || !alias_evidence_mentions_surface(evidence, alias_text) {
        return false;
    }

    let owner = &identities[owner_index];
    let owner_surfaces = character_identity_surfaces(owner)
        .into_iter()
        .map(|(surface, _)| surface)
        .collect::<Vec<_>>();
    if evidence_mentions_any_surface(evidence, &owner_surfaces) {
        return true;
    }

    if owner_surfaces
        .iter()
        .any(|surface| character_surfaces_share_distinctive_token(alias_text, surface))
    {
        return true;
    }

    false
}

fn alias_evidence_mentions_surface(evidence: &[StoryEvidenceSpan], surface: &str) -> bool {
    let surface_key = normalized_folded_text_key(surface);
    if surface_key.is_empty() {
        return false;
    }

    evidence.iter().any(|span| {
        span.quote
            .as_deref()
            .is_some_and(|quote| normalized_folded_text_key(quote).contains(&surface_key))
    })
}

fn evidence_mentions_any_surface(evidence: &[StoryEvidenceSpan], surfaces: &[String]) -> bool {
    surfaces
        .iter()
        .any(|surface| alias_evidence_mentions_surface(evidence, surface))
}

fn character_surfaces_share_distinctive_token(left: &str, right: &str) -> bool {
    let left_tokens = distinctive_surface_tokens(left);
    if left_tokens.is_empty() {
        return false;
    }
    let right_tokens = distinctive_surface_tokens(right);
    left_tokens.iter().any(|token| right_tokens.contains(token))
}

fn distinctive_surface_tokens(value: &str) -> std::collections::HashSet<String> {
    normalized_folded_text_key(value)
        .split('_')
        .filter(|token| token.chars().count() >= 2)
        .map(str::to_string)
        .collect()
}

fn find_character_identity_index_by_surface(
    identities: &[CharacterIdentity],
    surface: &str,
) -> Option<usize> {
    let key = normalized_text_key(surface);
    if key.is_empty() {
        return None;
    }

    identities.iter().position(|identity| {
        normalized_text_key(&identity.name) == key
            || identity
                .aliases
                .iter()
                .any(|alias| normalized_text_key(&alias.text) == key)
    })
}

fn better_alias_owner_by_surface(
    identities: &[CharacterIdentity],
    owner_index: usize,
    alias_text: &str,
) -> Option<usize> {
    let alias_key = normalized_text_key(alias_text);
    if alias_key.is_empty() {
        return None;
    }

    if identities.iter().enumerate().any(|(index, identity)| {
        index != owner_index && normalized_text_key(&identity.name) == alias_key
    }) {
        return None;
    }

    let alias_identity = CharacterIdentity {
        name: alias_text.to_string(),
        aliases: Vec::new(),
    };
    let owner_score = services::analysis_identity_review::character_identity_candidate_score(
        &alias_identity,
        &identities[owner_index],
    );
    let mut best: Option<(usize, f64)> = None;

    for (index, identity) in identities.iter().enumerate() {
        if index == owner_index {
            continue;
        }

        let score = services::analysis_identity_review::character_identity_candidate_score(
            &alias_identity,
            identity,
        );
        if score < CHARACTER_ALIAS_OWNER_REDIRECT_MIN_SCORE {
            continue;
        }

        match best {
            Some((_, best_score)) if score > best_score => best = Some((index, score)),
            None => best = Some((index, score)),
            _ => {}
        }
    }

    let (best_index, best_score) = best?;
    if best_score > owner_score + CHARACTER_CANONICAL_REVIEW_SCORE_GAP {
        Some(best_index)
    } else {
        None
    }
}
