# Implementation Plan

Muc tieu: bat dau rewrite bang nen tang dung, chua port UI lon qua som.

## Phase 0 - Repo Foundation

- Chot ten project, README, license, coding conventions.
- Tao workspace Rust + SvelteKit sau khi da chot package manager.
- Tao schema folder cho shared API contracts.
- Tao ADR dau tien ve stack hybrid web/desktop.

## Phase 1 - Core Backend Skeleton

- Axum health endpoint.
- Config loader cho `desktop`, `web`, `demo`.
- SQLx migrations:
  - users
  - workspaces
  - projects
  - novels
  - chapters
  - analysis_jobs
  - llm_provider_configs
  - observations
  - evidence_spans
- WebSocket/SSE event contract cho job progress.
- Typed error model.

## Phase 2 - BYOK and AI Provider Layer

- Provider trait:
  - OpenAI-compatible chat/completions
  - Anthropic messages
  - llama.cpp local OpenAI-compatible endpoint
- Secret policy:
  - session-only key option
  - encrypted persisted key option
  - masked display
  - no logging
- Token usage accounting per user/project/job.

## Phase 3 - Novel Import and Split

- TXT/Markdown upload.
- Chapter splitter with deterministic heuristics first.
- Preview + confirm import flow.
- Store source text and chapter metadata.
- Add import regression fixtures.

## Phase 4 - First Extraction Contract

- JSON schema for chapter extraction.
- Evidence spans required for factual claims.
- Retry/repair policy.
- Persist observations, not raw LLM blobs as the main truth.
- Add old-vs-new regression harness against sample novels.

## Phase 5 - Minimal Workspace UI

- SvelteKit shell with sidebar and project routes.
- Settings page for BYOK.
- Upload/import page.
- Analysis progress page.
- Reading page with chapter navigation.

## Early Non-Goals

- No full map renderer in the first milestone.
- No billing system until BYOK and quota model are stable.
- No multi-organization enterprise features until single-user hosted workspace works.

