# Implementation Plan

Mục tiêu: bắt đầu rewrite bằng nền tảng đúng, chưa port UI lớn quá sớm.

## Định Hướng Realtime

- Mọi tác vụ ghi DB làm thay đổi dữ liệu hiển thị trên UI phải tạo event bền vững trong cùng transaction hoặc ngay sau transaction thành công.
- UI không được phụ thuộc vào thao tác refresh thủ công để thấy kết quả analysis, translation, inline edit hoặc stale marking mới.
- Cầu nối hiện tại có thể dùng snapshot invalidation ngắn hạn cho lát nhỏ, nhưng kiến trúc chính thức phải là project event stream bằng SSE hoặc WebSocket với reconnect và resume theo sequence.
- Module trích xuất nhân vật hiện tại là lát đầu tiên áp dụng chuẩn này: persist character records, persist mention spans, ghi event riêng, rồi Reading workspace tự đồng bộ để hiển thị highlight mới.

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
- Project event stream cho mọi dữ liệu người dùng nhìn thấy sau khi DB thay đổi.
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
- Add an agentic run contract so extraction can continue without human approval.
- Add stale markers for observations, translations, and evidence when source chapter text changes.

## Phase 5 - Parallel Translation Foundation

- Add source segment model shared by analysis and translation.
- Add translation job and translation segment model.
- Add glossary and style guide model.
- Add translation review item model.
- Draft translation prompt contract.
- Add side-by-side reading UI placeholder.
- Allow translation jobs to run in parallel with analysis when dependencies are satisfied.
- Mark translation segments stale when raw text, glossary, alias, or entity canonical names change.

## Phase 6 - Minimal Workspace UI

- SvelteKit shell with sidebar and project routes.
- Settings page for local llama.cpp runtime and BYOK.
- Upload/import page.
- Analysis progress page.
- Reading page with chapter navigation.
- Current slice: typed API wiring is live for bookshelf, import, reading, analysis, and local LLM settings; bookshelf delete modes, local model picker/preset downloads, and reading typography controls are also live; review remains placeholder-only until observation APIs exist.
- Current realtime bridge: reading workspace tự invalidate snapshot để thấy character highlights mới sau khi analysis persist dữ liệu; cần thay bằng SSE/WebSocket chính thức ở milestone kế tiếp.
- Inline editing foundation for reading, entity, alias, relationship, glossary, and translation fields.
- Double-click enters edit mode; blur or Enter saves through typed API; Escape cancels.
- User corrections must persist to DB and update downstream stale/projection state.
- Add copy catalog/i18n and prompt registry foundations before adding more hardcoded UI/prompt text.

## Early Non-Goals

- No full map renderer in the first milestone.
- No billing system until BYOK and quota model are stable.
- No multi-organization enterprise features until single-user hosted workspace works.
