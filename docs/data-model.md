# Data Model

This document sketches the target logical data model. It is not a final migration.

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

## Novel Source

```text
novels
chapters
chapter_segments
source_files
```

## Analysis Jobs

```text
analysis_jobs
analysis_runs
prompt_runs
llm_usage_events
analysis_events
```

## Evidence and Observations

```text
evidence_spans
observations
review_items
user_corrections
```

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

