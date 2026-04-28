use std::collections::HashMap;

use crate::*;

pub(crate) fn merge_character_identity_records(
    document: &mut StoryExtractionDocument,
    identities: &[CharacterIdentity],
) {
    for identity in identities {
        let mut record = identity_to_record(identity, document.chapter_num);
        let exact_index = document.records.iter().position(|record| {
            record.group_key == "character"
                && normalized_text_key(&record.display_name) == normalized_text_key(&identity.name)
        });
        let matched_index = exact_index.or_else(|| {
            document
                .records
                .iter()
                .position(|record| payload_record_matches_identity(record, identity))
        });

        if let Some(index) = matched_index {
            let target = &mut document.records[index];
            target.group_key = "character".to_string();
            target.group_label = "Nhân Vật".to_string();
            target.display_name = identity.name.clone();
            target.entity_key = record.entity_key.take();
            merge_character_record(target, record);
        } else {
            document.records.push(record);
        }
    }

    dedupe_character_records_by_identity(document);
}

fn dedupe_character_records_by_identity(document: &mut StoryExtractionDocument) {
    let mut index = 0;
    while index < document.records.len() {
        if document.records[index].group_key != "character" {
            index += 1;
            continue;
        }

        let identity = CharacterIdentity {
            name: document.records[index].display_name.clone(),
            aliases: aliases_from_payload_record(&document.records[index]),
        };
        let duplicate_index = document
            .records
            .iter()
            .enumerate()
            .skip(index + 1)
            .find(|(_, record)| {
                record.group_key == "character"
                    && payload_record_matches_identity(record, &identity)
            })
            .map(|(duplicate_index, _)| duplicate_index);

        if let Some(duplicate_index) = duplicate_index {
            let duplicate = document.records.remove(duplicate_index);
            merge_character_record(&mut document.records[index], duplicate);
        } else {
            index += 1;
        }
    }
}

fn identity_to_record(
    identity: &CharacterIdentity,
    chapter_num: i64,
) -> StoryExtractionRecordPayload {
    let field_values = identity
        .aliases
        .iter()
        .filter_map(|alias| {
            if !is_persistable_character_alias_type(&alias.alias_type) {
                return None;
            }

            let value = clean_character_surface(&alias.text);
            if value.is_empty() {
                return None;
            }

            Some(StoryExtractionFieldValuePayload {
                value,
                confidence: Some(1.0),
                evidence: alias
                    .evidence
                    .iter()
                    .filter(|evidence| evidence.chapter_num == chapter_num)
                    .cloned()
                    .collect(),
                related_character: None,
                relationship_type: None,
                relationship_label: None,
                relationship_direction: None,
            })
        })
        .collect::<Vec<_>>();

    let mut fields = Vec::new();
    if !field_values.is_empty() {
        fields.push(StoryExtractionFieldPayload {
            field_key: "other_alias".to_string(),
            field_label: "Tên gọi khác".to_string(),
            values: field_values,
        });
    }

    StoryExtractionRecordPayload {
        group_key: "character".to_string(),
        group_label: "Nhân Vật".to_string(),
        display_name: identity.name.clone(),
        entity_key: Some(normalize_ascii_snake_key(&identity.name)),
        mentions: Vec::new(),
        fields,
    }
}

pub(crate) fn working_identities_for_chunk(
    working_document: &StoryExtractionDocument,
    chunk_identities: &[CharacterIdentity],
) -> Vec<CharacterIdentity> {
    let mut merged = chunk_identities.to_vec();

    for record in &working_document.records {
        if record.group_key != "character" || record.mentions.is_empty() {
            continue;
        }

        if merged
            .iter()
            .any(|identity| payload_record_matches_identity(record, identity))
        {
            continue;
        }

        merged.push(CharacterIdentity {
            name: record.display_name.clone(),
            aliases: aliases_from_payload_record(record),
        });
    }

    merged
}

pub(crate) fn hydrate_identities_with_alias_map(
    mut identities: Vec<CharacterIdentity>,
    db_aliases: &[StoryCharacterAliasView],
) -> Vec<CharacterIdentity> {
    let mut aliases_by_name = std::collections::HashMap::<String, Vec<CharacterAlias>>::new();
    for alias in db_aliases {
        let name_key = normalized_text_key(&alias.display_name);
        if name_key.is_empty() {
            continue;
        }

        aliases_by_name
            .entry(name_key)
            .or_default()
            .push(CharacterAlias {
                text: alias.alias_text.clone(),
                alias_type: alias.alias_type.clone(),
                alias_label: alias.alias_label.clone(),
                is_primary: alias.confidence.unwrap_or(0.0) >= 1.0,
                evidence: alias.evidence.clone(),
            });
    }

    for identity in &mut identities {
        let name_key = normalized_text_key(&identity.name);
        let Some(aliases) = aliases_by_name.get(&name_key) else {
            continue;
        };

        for alias in aliases {
            push_character_alias_if_valid(&mut identity.aliases, alias.clone(), &identity.name);
        }
    }

    identities
}

pub(crate) fn aliases_from_record(record: &StoryExtractionRecordView) -> Vec<CharacterAlias> {
    let mut aliases = Vec::new();

    for field in &record.fields {
        let field_key = normalize_character_alias_type(&field.field_key);
        if !is_character_alias_field_key(&field_key) {
            continue;
        }

        for value in &field.values {
            let alias_type = normalize_character_alias_type(&field_key);
            if !is_persistable_character_alias_type(&alias_type) {
                continue;
            }
            aliases.push(CharacterAlias {
                text: value.value.clone(),
                alias_type,
                alias_label: normalize_character_alias_label(&field_key, &field.field_label),
                is_primary: value.confidence.unwrap_or(0.0) >= 1.0,
                evidence: value.evidence.clone(),
            });
        }
    }

    aliases
}

pub(crate) fn aliases_from_payload_record(
    record: &StoryExtractionRecordPayload,
) -> Vec<CharacterAlias> {
    let mut aliases = Vec::new();

    for field in &record.fields {
        let field_key = normalize_character_alias_type(&field.field_key);
        if !is_character_alias_field_key(&field_key) {
            continue;
        }

        for value in &field.values {
            let alias_type = normalize_character_alias_type(&field_key);
            if !is_persistable_character_alias_type(&alias_type) {
                continue;
            }
            aliases.push(CharacterAlias {
                text: value.value.clone(),
                alias_type,
                alias_label: normalize_character_alias_label(&field_key, &field.field_label),
                is_primary: value.confidence.unwrap_or(0.0) >= 1.0,
                evidence: value.evidence.clone(),
            });
        }
    }

    aliases
}

fn is_character_alias_field_key(field_key: &str) -> bool {
    matches!(
        field_key,
        "alias" | "aliases" | "other_alias" | "other_name" | "other_names" | "nickname"
    )
}

pub(crate) fn payload_record_matches_identity(
    record: &StoryExtractionRecordPayload,
    identity: &CharacterIdentity,
) -> bool {
    let identity_names = character_identity_surface_keys(identity);
    if identity_names.contains(&normalized_text_key(&record.display_name)) {
        return true;
    }

    for field in &record.fields {
        let field_key = normalize_ascii_snake_key(&field.field_key);
        if !is_character_alias_field_key(&field_key) {
            continue;
        }

        for value in &field.values {
            if identity_names.contains(&normalized_text_key(&value.value)) {
                return true;
            }
        }
    }

    false
}

pub(crate) fn view_record_matches_identity(
    record: &StoryExtractionRecordView,
    identity: &CharacterIdentity,
) -> bool {
    let identity_names = character_identity_surface_keys(identity);
    if identity_names.contains(&normalized_text_key(&record.display_name)) {
        return true;
    }

    aliases_from_record(record)
        .iter()
        .any(|alias| identity_names.contains(&normalized_text_key(&alias.text)))
}

fn character_identity_surface_keys(
    identity: &CharacterIdentity,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    keys.insert(normalized_text_key(&identity.name));
    for alias in &identity.aliases {
        keys.insert(normalized_text_key(&alias.text));
    }
    keys
}

pub(crate) fn merge_character_identity_mentions(
    document: &mut StoryExtractionDocument,
    identity: &CharacterIdentity,
    mentions: Vec<StoryCharacterMention>,
) {
    let target = ensure_character_record(document, identity);
    merge_character_mentions(&mut target.mentions, mentions);
}

pub(crate) fn merge_character_identity_fields(
    document: &mut StoryExtractionDocument,
    identity: &CharacterIdentity,
    fields: Vec<StoryExtractionFieldPayload>,
) {
    let target = ensure_character_record(document, identity);
    for field in fields {
        merge_character_field(&mut target.fields, field);
    }
}

fn field_value_has_relationship_metadata(value: &StoryExtractionFieldValuePayload) -> bool {
    value
        .related_character
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        || value
            .relationship_type
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        || value
            .relationship_label
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        || value
            .relationship_direction
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
}

pub(crate) fn is_relationship_field_key(field_key: &str) -> bool {
    matches!(
        field_key,
        "relationship"
            | "relationships"
            | "relation"
            | "relations"
            | "family_relation"
            | "social_relation"
            | "character_relationship"
            | "character_relationships"
    )
}

fn is_valid_relationship_field_value(value: &StoryExtractionFieldValuePayload) -> bool {
    value
        .related_character
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        && value
            .relationship_type
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        && value
            .relationship_label
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        && value
            .relationship_direction
            .as_deref()
            .is_some_and(|direction| matches!(direction, "self_to_related" | "related_to_self"))
        && value.evidence.iter().any(|evidence| {
            evidence
                .quote
                .as_deref()
                .is_some_and(|quote| !quote.trim().is_empty())
        })
}

fn ensure_character_record<'a>(
    document: &'a mut StoryExtractionDocument,
    identity: &CharacterIdentity,
) -> &'a mut StoryExtractionRecordPayload {
    let key = normalized_text_key(&identity.name);
    if let Some(index) = document.records.iter().position(|record| {
        normalized_text_key(&record.display_name) == key
            || payload_record_matches_identity(record, identity)
    }) {
        return &mut document.records[index];
    }

    document
        .records
        .push(identity_to_record(identity, document.chapter_num));
    document
        .records
        .last_mut()
        .expect("record was just inserted")
}

fn merge_character_record(
    target: &mut StoryExtractionRecordPayload,
    mut source: StoryExtractionRecordPayload,
) {
    if target
        .entity_key
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        target.entity_key = source.entity_key.take();
    }

    merge_character_mentions(&mut target.mentions, source.mentions);

    for source_field in source.fields {
        merge_character_field(&mut target.fields, source_field);
    }
}

fn merge_character_mentions(
    target: &mut Vec<StoryCharacterMention>,
    source: Vec<StoryCharacterMention>,
) {
    let mut seen = target
        .iter()
        .map(|mention| {
            format!(
                "{}:{}:{}",
                mention.start_char,
                mention.end_char,
                mention.text.trim()
            )
        })
        .collect::<std::collections::HashSet<_>>();

    for mention in source {
        let key = format!(
            "{}:{}:{}",
            mention.start_char,
            mention.end_char,
            mention.text.trim()
        );
        if seen.insert(key) {
            target.push(mention);
        }
    }

    target.sort_by(|left, right| {
        left.start_char
            .cmp(&right.start_char)
            .then_with(|| right.end_char.cmp(&left.end_char))
    });
}

fn merge_character_field(
    target: &mut Vec<StoryExtractionFieldPayload>,
    source: StoryExtractionFieldPayload,
) {
    let source_key = normalize_ascii_snake_key(&source.field_key);
    if let Some(target_field) = target
        .iter_mut()
        .find(|field| normalize_ascii_snake_key(&field.field_key) == source_key)
    {
        merge_character_field_values(&mut target_field.values, source.values);
        return;
    }

    let mut values = Vec::new();
    for source_value in source.values {
        if let Some(source_value) =
            merge_duplicate_character_field_value(target, &source_key, source_value)
        {
            values.push(source_value);
        }
    }

    if values.is_empty() {
        return;
    }

    target.push(StoryExtractionFieldPayload {
        field_key: source_key,
        field_label: source.field_label,
        values,
    });
}

fn merge_duplicate_character_field_value(
    target: &mut [StoryExtractionFieldPayload],
    source_key: &str,
    source_value: StoryExtractionFieldValuePayload,
) -> Option<StoryExtractionFieldValuePayload> {
    if field_value_has_relationship_metadata(&source_value) || is_relationship_field_key(source_key)
    {
        return Some(source_value);
    }

    let source_value_key = normalized_text_key(&source_value.value);
    if source_value_key.is_empty() {
        return None;
    }

    for field in target {
        let target_key = normalize_ascii_snake_key(&field.field_key);
        if target_key == source_key || is_relationship_field_key(&target_key) {
            continue;
        }

        for target_value in &mut field.values {
            if normalized_text_key(&target_value.value) != source_value_key {
                continue;
            }

            merge_character_field_value_metadata(target_value, &source_value);
            merge_story_evidence(&mut target_value.evidence, source_value.evidence);
            return None;
        }
    }

    Some(source_value)
}

fn merge_character_field_values(
    target: &mut Vec<StoryExtractionFieldValuePayload>,
    source: Vec<StoryExtractionFieldValuePayload>,
) {
    let mut value_index_by_key = target
        .iter()
        .enumerate()
        .map(|(index, value)| (character_field_value_key(value), index))
        .collect::<HashMap<_, _>>();

    for source_value in source {
        let key = character_field_value_key(&source_value);
        if let Some(index) = value_index_by_key.get(&key).copied() {
            merge_character_field_value_metadata(&mut target[index], &source_value);
            merge_story_evidence(&mut target[index].evidence, source_value.evidence);
        } else {
            let index = target.len();
            value_index_by_key.insert(key, index);
            target.push(source_value);
        }
    }
}

fn character_field_value_key(value: &StoryExtractionFieldValuePayload) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        normalized_text_key(&value.value),
        value
            .related_character
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default(),
        value
            .relationship_type
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default(),
        value
            .relationship_label
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default(),
        value
            .relationship_direction
            .as_deref()
            .map(normalized_text_key)
            .unwrap_or_default()
    )
}

fn merge_character_field_value_metadata(
    target: &mut StoryExtractionFieldValuePayload,
    source: &StoryExtractionFieldValuePayload,
) {
    if target.related_character.is_none() {
        target.related_character = source.related_character.clone();
    }
    if target.relationship_type.is_none() {
        target.relationship_type = source.relationship_type.clone();
    }
    if target.relationship_label.is_none() {
        target.relationship_label = source.relationship_label.clone();
    }
    if target.relationship_direction.is_none() {
        target.relationship_direction = source.relationship_direction.clone();
    }
    if target.confidence.is_none() {
        target.confidence = source.confidence;
    }
}

fn merge_story_evidence(target: &mut Vec<StoryEvidenceSpan>, source: Vec<StoryEvidenceSpan>) {
    let mut seen = target
        .iter()
        .map(story_evidence_key)
        .collect::<std::collections::HashSet<_>>();

    for evidence in source {
        let key = story_evidence_key(&evidence);
        if seen.insert(key) {
            target.push(evidence);
        }
    }
}

fn story_evidence_key(evidence: &StoryEvidenceSpan) -> String {
    format!(
        "{}:{}:{}:{}",
        evidence.start_char.unwrap_or(-1),
        evidence.end_char.unwrap_or(-1),
        evidence.quote.as_deref().unwrap_or_default().trim(),
        evidence.reason.as_deref().unwrap_or_default().trim()
    )
}

pub(crate) fn validate_character_extraction_document(
    document: &StoryExtractionDocument,
    chapter_num: i64,
    chapter_text: &str,
) -> Result<(), ApiError> {
    if document.schema_version.trim() != CHARACTER_EXTRACTION_SCHEMA_VERSION {
        return Err(ApiError::bad_request(format!(
            "character extraction schema mismatch: expected {}, got {}",
            CHARACTER_EXTRACTION_SCHEMA_VERSION, document.schema_version
        )));
    }

    if document.chapter_num != chapter_num {
        return Err(ApiError::bad_request(
            "character extraction chapter_num does not match the running chapter",
        ));
    }

    for record in &document.records {
        validate_character_record(record, chapter_num, chapter_text)?;
    }

    Ok(())
}

fn validate_character_record(
    record: &StoryExtractionRecordPayload,
    chapter_num: i64,
    chapter_text: &str,
) -> Result<(), ApiError> {
    let group_key = record.group_key.trim();
    if !matches!(group_key, "character" | "relationship") {
        return Err(ApiError::bad_request(
            "story extraction records must use group_key character or relationship",
        ));
    }

    if record.group_label.trim().is_empty() {
        return Err(ApiError::bad_request(
            "character extraction group_label is required",
        ));
    }

    if record.display_name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "character extraction display_name is required",
        ));
    }

    if group_key == "character" {
        for mention in &record.mentions {
            if mention.text.trim().is_empty() {
                return Err(ApiError::bad_request(
                    "character extraction mention text is required",
                ));
            }

            let chapter_len = chapter_text.chars().count() as i64;
            if mention.start_char < 0
                || mention.end_char <= mention.start_char
                || mention.end_char > chapter_len
            {
                return Err(ApiError::bad_request(
                    "character extraction mention span is outside chapter bounds",
                ));
            }
        }
    }

    for field in &record.fields {
        if field.field_key.trim().is_empty() {
            return Err(ApiError::bad_request(
                "character extraction field_key is required",
            ));
        }

        if field.field_label.trim().is_empty() {
            return Err(ApiError::bad_request(
                "character extraction field_label is required",
            ));
        }

        for value in &field.values {
            if value.value.trim().is_empty() {
                return Err(ApiError::bad_request(
                    "character extraction value is required",
                ));
            }

            if is_relationship_field_key(&normalize_ascii_snake_key(&field.field_key))
                && !is_valid_relationship_field_value(value)
            {
                return Err(ApiError::bad_request(
                    "character relationship fields require related_character, relationship_type, relationship_label, relationship_direction, and quoted evidence",
                ));
            }

            if let Some(confidence) = value.confidence {
                if !(0.0..=1.0).contains(&confidence) {
                    return Err(ApiError::bad_request(
                        "character extraction confidence must be between 0 and 1",
                    ));
                }
            }

            for evidence in &value.evidence {
                if evidence.chapter_num != chapter_num {
                    return Err(ApiError::bad_request(format!(
                        "character extraction evidence chapter_num does not match the running chapter: record={}, field={}, value={}, evidence_chapter_num={}, expected_chapter_num={}",
                        record.display_name,
                        field.field_key,
                        value.value,
                        evidence.chapter_num,
                        chapter_num
                    )));
                }

                if let (Some(start), Some(end)) = (evidence.start_char, evidence.end_char) {
                    let chapter_len = chapter_text.chars().count() as i64;
                    if start < 0 || end < start || end > chapter_len {
                        return Err(ApiError::bad_request(
                            "character extraction evidence span is outside chapter bounds",
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn normalize_character_field_keys(document: &mut StoryExtractionDocument) {
    for record in &mut document.records {
        for field in &mut record.fields {
            field.field_key = normalize_ascii_snake_key(&field.field_key);
        }
    }
}
