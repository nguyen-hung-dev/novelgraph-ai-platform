# Data Model

This document sketches the target logical data model. The SQLite/PostgreSQL migrations now implement the foundation identity, project, novel source, analysis job, job event, translation job, glossary, usage, and job lifecycle fields.

## Principles

- Source text is preserved.
- Observations are evidence-linked.
- UI views are projections, not source truth.
- Desktop and web should share logical models even if physical databases differ.

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
analysis_runs
prompt_runs
llm_usage_events
job_events
```

Current implementation:

- `analysis_jobs` stores pending analysis jobs created by import confirmation.
- `job_events` stores sequenced events for both analysis and translation jobs.
- `analysis_jobs` and `translation_jobs` expose `started_at`, `finished_at`, `error_code`, and `error_message`.
- Job state transitions are validated in `crates/jobs`.

## Evidence and Observations

```text
evidence_spans
observations
review_items
user_corrections
```

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
