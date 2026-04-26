# Phase 1 - Backend Skeleton

Goal: create the first runnable Rust backend with storage foundations.

## Scope

- Rust workspace.
- Axum health endpoint.
- Config loading.
- SQLx setup.
- SQLite development database.
- PostgreSQL-ready migrations.
- Typed error response.
- Basic job event schema.

## Initial Tables

- users
- workspaces
- projects
- novels
- chapters
- source_segments
- analysis_jobs
- job_events
- translation_jobs
- translation_segments
- glossary_entries
- style_profiles
- translation_review_items
- llm_provider_configs
- llm_usage_events

## Non-Goals

- No full LLM extraction yet.
- No graph/map/timeline generation.
- No production auth provider integration until local schema is stable.

## Validation

- Backend starts locally.
- `/health` returns mode and version.
- Migrations run against SQLite.
- Schema design document is updated.
