# Implementation Plan

Mục tiêu: bắt đầu rewrite bằng nền tảng đúng, chưa port UI lớn quá sớm.

## Phase 0 - Repo Foundation

- Chốt tên project, README, license, coding conventions.
- Tạo workspace Rust + SvelteKit với `pnpm` ở root repo.
- Tạo schema folder cho shared API contracts.
- Tạo ADR đầu tiên về stack hybrid web/desktop.

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

## Phase 5 - Parallel Translation Foundation

- Add source segment model shared by analysis and translation.
- Add translation job and translation segment model.
- Add glossary and style guide model.
- Add translation review item model.
- Draft translation prompt contract.
- Add side-by-side reading UI placeholder.

## Phase 6 - Minimal Workspace UI

- SvelteKit shell with sidebar and project routes.
- Settings page for BYOK.
- Upload/import page.
- Analysis progress page.
- Reading page with chapter navigation.
- Current slice: mock-backed workspace shell ready for typed API wiring.

## Early Non-Goals

- No full map renderer in the first milestone.
- No billing system until BYOK and quota model are stable.
- No multi-organization enterprise features until single-user hosted workspace works.
