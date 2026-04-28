use std::collections::HashMap;

use sqlx::Row;

use crate::sqlite::*;
use crate::StorageResult;

#[derive(Debug, Clone)]
struct StoryCharacterAliasUpsert {
    project_id: String,
    novel_id: String,
    job_id: String,
    entity_key: String,
    display_name: String,
    alias_text: String,
    alias_key: String,
    alias_type: String,
    alias_label: String,
    confidence: Option<f64>,
    first_chapter_num: i64,
    evidence_json: String,
}

pub(super) async fn rebuild_story_character_aliases_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    analysis_job_id: &str,
) -> StorageResult<usize> {
    sqlx::query(
        "DELETE FROM story_character_aliases
         WHERE project_id = ? AND job_id = ?",
    )
    .bind(project_id)
    .bind(analysis_job_id)
    .execute(&mut **tx)
    .await?;

    let record_rows = sqlx::query(
        "SELECT id, project_id, novel_id, job_id, chapter_num, entity_key, display_name
         FROM story_extraction_records
         WHERE project_id = ? AND job_id = ? AND group_key = 'character'
         ORDER BY chapter_num ASC, display_name ASC, id ASC",
    )
    .bind(project_id)
    .bind(analysis_job_id)
    .fetch_all(&mut **tx)
    .await?;

    let mut aliases = HashMap::<String, StoryCharacterAliasUpsert>::new();
    for record_row in record_rows {
        let record_id: String = record_row.get("id");
        let display_name: String = record_row.get("display_name");
        let entity_key = record_row
            .get::<Option<String>, _>("entity_key")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| normalized_story_alias_key(&display_name));
        if entity_key.trim().is_empty() {
            continue;
        }

        let project_id_value: String = record_row.get("project_id");
        let novel_id: String = record_row.get("novel_id");
        let job_id: String = record_row.get("job_id");
        let chapter_num: i64 = record_row.get("chapter_num");

        push_story_character_alias(
            &mut aliases,
            StoryCharacterAliasUpsert {
                project_id: project_id_value.clone(),
                novel_id: novel_id.clone(),
                job_id: job_id.clone(),
                entity_key: entity_key.clone(),
                display_name: display_name.clone(),
                alias_text: display_name.clone(),
                alias_key: normalized_story_alias_key(&display_name),
                alias_type: "canonical_name".to_string(),
                alias_label: "Tên chính".to_string(),
                confidence: Some(1.0),
                first_chapter_num: chapter_num,
                evidence_json: "[]".to_string(),
            },
        );

        let alias_rows = sqlx::query(
            "SELECT f.field_key, f.field_label, v.value_text, v.confidence, v.evidence_json
             FROM story_extraction_fields f
             JOIN story_extraction_values v ON v.field_id = f.id
             WHERE f.record_id = ?
             ORDER BY f.field_key ASC, v.created_at ASC, v.id ASC",
        )
        .bind(&record_id)
        .fetch_all(&mut **tx)
        .await?;

        for alias_row in alias_rows {
            let field_key: String = alias_row.get("field_key");
            if !is_story_character_alias_field_key(&field_key) {
                continue;
            }

            let alias_text: String = alias_row.get("value_text");
            let alias_key = normalized_story_alias_key(&alias_text);
            if alias_key.is_empty() || alias_key == normalized_story_alias_key(&display_name) {
                continue;
            }

            push_story_character_alias(
                &mut aliases,
                StoryCharacterAliasUpsert {
                    project_id: project_id_value.clone(),
                    novel_id: novel_id.clone(),
                    job_id: job_id.clone(),
                    entity_key: entity_key.clone(),
                    display_name: display_name.clone(),
                    alias_text,
                    alias_key,
                    alias_type: normalize_story_alias_type(&field_key),
                    alias_label: alias_row.get("field_label"),
                    confidence: alias_row.get("confidence"),
                    first_chapter_num: chapter_num,
                    evidence_json: alias_row.get("evidence_json"),
                },
            );
        }
    }

    let alias_count = aliases.len();
    for alias in aliases.into_values() {
        sqlx::query(
            "INSERT INTO story_character_aliases (
                id, project_id, novel_id, job_id, entity_key, display_name,
                alias_text, alias_key, alias_type, alias_label, confidence,
                first_chapter_num, evidence_json
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(prefixed_id("sali"))
        .bind(alias.project_id)
        .bind(alias.novel_id)
        .bind(alias.job_id)
        .bind(alias.entity_key)
        .bind(alias.display_name)
        .bind(alias.alias_text)
        .bind(alias.alias_key)
        .bind(alias.alias_type)
        .bind(alias.alias_label)
        .bind(alias.confidence)
        .bind(alias.first_chapter_num)
        .bind(alias.evidence_json)
        .execute(&mut **tx)
        .await?;
    }

    Ok(alias_count)
}

fn push_story_character_alias(
    aliases: &mut HashMap<String, StoryCharacterAliasUpsert>,
    alias: StoryCharacterAliasUpsert,
) {
    if alias.alias_key.is_empty() {
        return;
    }
    if alias.alias_type != "canonical_name"
        && (!is_story_alias_type_persistable(&alias.alias_type)
            || !is_stable_story_character_alias_surface(&alias.alias_text))
    {
        return;
    }

    let key = format!("{}:{}", alias.entity_key, alias.alias_key);
    if let Some(existing) = aliases.get_mut(&key) {
        if alias.first_chapter_num < existing.first_chapter_num {
            existing.first_chapter_num = alias.first_chapter_num;
            existing.alias_text = alias.alias_text;
        }
        if alias.confidence.unwrap_or(0.0) > existing.confidence.unwrap_or(0.0) {
            existing.confidence = alias.confidence;
        }
        if existing.evidence_json.trim() == "[]" && alias.evidence_json.trim() != "[]" {
            existing.evidence_json = alias.evidence_json;
        }
        return;
    }

    aliases.insert(key, alias);
}

fn is_stable_story_character_alias_surface(value: &str) -> bool {
    let surface = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if surface.is_empty() {
        return false;
    }

    let key = normalized_story_alias_key(&surface);
    if key.is_empty() {
        return false;
    }

    let tokens = surface.split_whitespace().collect::<Vec<_>>();
    let token_count = tokens.len();
    let char_count = surface.chars().filter(|ch| ch.is_alphanumeric()).count();
    let has_uppercase_token = tokens
        .iter()
        .any(|token| token.chars().next().is_some_and(char::is_uppercase));

    if token_count == 0 || char_count <= 3 {
        return false;
    }

    if !has_uppercase_token && token_count == 1 && char_count <= 4 {
        return false;
    }

    if !has_uppercase_token && token_count <= 2 && char_count <= 6 {
        return false;
    }

    if !has_uppercase_token && token_count > 3 {
        return false;
    }

    true
}

fn is_story_character_alias_field_key(field_key: &str) -> bool {
    matches!(
        normalize_story_alias_type(field_key).as_str(),
        "alias" | "aliases" | "other_alias" | "other_name" | "other_names" | "nickname"
    )
}

fn is_story_alias_type_persistable(alias_type: &str) -> bool {
    matches!(
        normalize_story_alias_type(alias_type).as_str(),
        "alias" | "aliases" | "other_alias" | "other_name" | "other_names" | "nickname"
    )
}

fn normalize_story_alias_type(field_key: &str) -> String {
    match normalized_story_alias_key(field_key).as_str() {
        "nickname" | "biet_danh" => "nickname".to_string(),
        "alias" | "aliases" => "alias".to_string(),
        "other_name" | "other_names" | "ten_goi_khac" | "ten_khac" => "other_name".to_string(),
        "other_alias" => "other_alias".to_string(),
        "pronoun"
        | "personal_pronoun"
        | "dai_tu"
        | "dai_tu_nhan_xung"
        | "temporary_reference"
        | "grammatical_reference"
        | "descriptive_phrase"
        | "event_phrase"
        | "group_reference"
        | "possessive_phrase"
        | "generic_reference"
        | "unstable_reference" => "unstable_reference".to_string(),
        value => value.to_string(),
    }
}
