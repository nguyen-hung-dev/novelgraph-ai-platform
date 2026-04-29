Schema version: {schema_version}
Prompt template: {prompt_id}@{prompt_version}
Call profile: {call_profile}
Chapter number: {chapter_num}
Chapter title: {title}
Source language: {source_language}

Novel context:
{novel_context}

Known character surfaces from previous chapters that appear in this chapter:
{known_alias_surfaces_json}

Known stable relationships from earlier chapters whose endpoints appear in this chapter:
{known_relationships_json}

Current chapter text:
<<<CHAPTER_TEXT
{chapter_text}
CHAPTER_TEXT

Return one JSON object only.

Extraction policy:
1. Extract stable story graph facts evidenced by CHAPTER_TEXT only.
2. Keep "schema_version" exactly "{schema_version}".
3. Keep "chapter_num" equal to {chapter_num}.
4. Keep "call_profile" exactly "{call_profile}".
5. Every persisted character, field, and relationship must have exact evidence_quotes copied from CHAPTER_TEXT.
6. Do not output start_char or end_char. Backend maps evidence_quotes to offsets.

Character policy:
1. Use characters[] only for stable individual characters, named groups, or organizations that the story treats as entities.
2. Use entity_nature values from the schema. Prefer individual_character for people.
3. Do not create a character node for a role-only phrase, pronoun, possessive phrase, temporary description, generic noun, or lowercase scene reference.
4. If a current surface matches or nearly matches a known canonical name or alias, reuse the known canonical display_name when evidence supports the same person.
5. Put uncertain aliases, typo-like surfaces, and owner-uncertain references into review_items instead of creating a separate character.
6. aliases[] must contain stable naming surfaces only. Do not include role-only or one-scene generic references.

Field policy:
1. characters[].fields is MVP appearance data only.
2. For every character field, set field_key to "appearance".
3. semantic_class must be one of: physical_appearance, clothing, age_or_build, appearance.
4. Do not put action, posture, emotion, attitude, status, location, occupation, role, history, speech, or temporary scene state into characters[].fields.
5. If such non-appearance information matters, place it in review_items.
6. Field values must describe the target character, not another character in the same sentence.
7. Field value is a compact display label, not raw evidence text. Keep it short, usually 2-8 words and under 60 characters.
8. Do not copy a full sentence, full clause, or comma-heavy description into value. Put the exact raw sentence fragment only in evidence_quotes.
9. If one evidence quote contains multiple useful appearance facts, split them into separate compact field values.
10. Examples: evidence "Toàn thân hắn được trùm bởi một cái áo bông đã cũ, ố vàng..." -> value "áo bông cũ ố vàng"; evidence "da tay thì đen đúa" -> value "da tay đen đúa"; evidence "vừa mới mười tuổi" -> value "mười tuổi".

Relationship policy:
1. relationships[] is only for stable graph relationships.
2. relationship_scope must be one of: kinship, organization_hierarchy, stable_relationship.
3. Persist family, sect/organization hierarchy, master-disciple, peer, enemy/rival, patron, servant, or other stable social relationships only when evidence is explicit.
4. Do not persist temporary interaction, shared event, co-presence, one-scene action, conversation, travel together, payment, exam participation, observation, help, or instruction as a relationship.
5. Put temporary or uncertain relationship-like facts into review_items.
6. source_to_target_label and target_to_source_label must be concise directional Vietnamese role labels with diacritics where natural, not possessive sentence fragments.
7. For the same unordered character pair and the same stable relationship_type, output exactly one relationship object.
8. Put every current-chapter quote for that same relationship into one evidence_quotes array. Do not split duplicate labels across multiple relationship objects.
9. Do not output synonym variants as separate relationships. Choose one canonical, most specific label pair and reuse it consistently.
10. If a Known stable relationship matches the same pair, reuse its relationship_type and directional labels unless CHAPTER_TEXT clearly shows a material relationship change.
11. evidence_quotes must contain quotes copied from CHAPTER_TEXT only. Do not copy evidence from earlier chapters into the current output.

Review policy:
1. Use review_items for uncertain identity ownership, possible alias merge, non-appearance character facts, temporary interactions, evidence ambiguity, or anything important but not safe to persist.
2. Keep review item summaries short and evidence-grounded.
3. Output labels in Vietnamese with diacritics where natural, but keep machine keys in ASCII snake_case.
