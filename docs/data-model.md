# Data Model

This document sketches the target logical data model. The SQLite/PostgreSQL migrations now implement the foundation identity, project, novel source, analysis job, per-chapter analysis run state, parsed story extraction records, job event, translation job, glossary, usage, and job lifecycle fields.

## Principles

- Source text is preserved.
- Observations are evidence-linked.
- UI views are projections, not source truth.
- Desktop and web should share logical models even if physical databases differ.
- User corrections from inline editing are first-class writes, not temporary UI state.
- Dependent projections must be marked stale when source text, aliases, glossary, relationships, or translations change.

## Identity and Workspace

```text
users
workspaces
workspace_members
projects
project_shares
```

Current implementation:

- `projects` now supports `deleted_at` for retention-aware archive behavior.
- Archived projects are hidden from the bookshelf and normal project reads.
- Hard deletion still cascades to novels, chapters, segments, jobs, and related records.

## Novel Source

```text
novels
chapters
source_files
source_segments
```

Current implementation:

- `chapters` stores source chapter text.
- `source_segments` stores paragraph-level spans with `chapter_id`, `segment_index`, `start_char`, `end_char`, `segment_kind`, and `text`.
- `source_files` is still planned.

## Analysis Jobs

```text
analysis_jobs
analysis_chapter_runs
prompt_runs
llm_usage_events
job_events
```

Current implementation:

- `analysis_jobs` stores pending analysis jobs created by import confirmation.
- `analysis_chapter_runs` stores one row per job/chapter with status, attempt count, prompt schema version, raw draft output JSON, error fields, and started/finished timestamps.
- `job_events` stores sequenced events for both analysis and translation jobs.
- `analysis_jobs` and `translation_jobs` expose `started_at`, `finished_at`, `error_code`, and `error_message`.
- Analysis jobs can enter `paused` when local execution is interrupted or local llama.cpp is unreachable; completed chapter runs are skipped on resume.
- Job state transitions are validated in `crates/jobs`.

## Evidence and Observations

```text
evidence_spans
observations
review_items
user_corrections
stale_marks
story_extraction_records
story_extraction_fields
story_extraction_values
```

Rules:

- `story_extraction_records`, `story_extraction_fields`, and `story_extraction_values` currently persist the focused `character` extraction slice.
- `story_extraction_values` includes optional relationship metadata columns (`related_character`, `relationship_type`, `relationship_label`, `relationship_direction`) so future relationship extraction can use typed values instead of free-form character fields.
- `user_corrections` records who or what changed raw text, aliases, entities, relationships, glossary entries, or translation segments.
- `stale_marks` records dependent data that needs rerun, refresh, or human review after a correction.
- AI-generated observations and user-corrected observations must remain distinguishable.
- Raw LLM output is audit/debug data and should not be treated as the main source of truth.

## Translation

```text
translation_jobs
translation_segments
glossary_entries
style_profiles
translation_review_items
```

Rules:

- `translation_segments` reference `source_segments`.
- Translation output is versioned.
- Approved glossary entries can be used by both translation and analysis.
- Source text remains authoritative for evidence.
- User-edited translation segments should be protected from automatic overwrite unless a force rerun is explicit.
- Glossary, alias, and source text edits should mark affected translation segments stale.
- The first implementation persists `translation_jobs`, `translation_segments`, `glossary_entries`, `style_profiles`, and `translation_review_items`; execution and review workflows are still planned.

Observation example:

```json
{
  "subject_ref": "entity:han-li",
  "predicate": "appears_at",
  "object_json": {
    "location_ref": "location:yellow-maple-valley"
  },
  "confidence": 0.92,
  "evidence_span_ids": ["span_..."]
}
```

## Knowledge Projections

```text
entities
entity_aliases
entity_mentions
relationships
locations
world_edges
timeline_events
scenes
factions
visual_cache
```

## RAG

```text
memory_chunks
embeddings
retrieval_traces
chat_conversations
chat_messages
```

## Secrets

```text
llm_provider_configs
```

Rules:

- Do not store raw keys unencrypted.
- Do not expose encrypted blobs to frontend.
- Prefer session-only keys for the first web MVP.
