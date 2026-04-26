# Roadmap

This roadmap is intentionally foundation-first. The project should not rush into complex visualizations before the data and job model are stable.

## Realtime-First Architecture Requirement

- [x] Mọi thay đổi DB làm đổi dữ liệu người dùng nhìn thấy phải có event bền vững để UI có thể đồng bộ lại, không dựa vào refresh thủ công như đường đi chính.
- [x] Module trích xuất nhân vật hiện tại phải ghi event riêng sau khi character records và mention spans được persist thành công.
- [x] Reading workspace phải tự đồng bộ lại snapshot để highlight nhân vật mới có thể xuất hiện khi người dùng đang mở trang.
- [ ] Thay cơ chế đồng bộ ngắn hạn bằng SSE/WebSocket chính thức cho project events, có reconnect và resume theo sequence.
- [ ] Chuẩn hóa event schema versioning cho analysis, translation, inline edit, stale marking và review queue.

## Phase 0 - Repository Foundation

- [x] Project name and README.
- [x] Architecture analysis.
- [x] Implementation plan.
- [x] BYOK security notes.
- [x] First ADR.
- [x] GitHub workflow and templates.
- [x] Codex operating context.
- [x] Layered implementation checklists.
- [x] License selection.
- [x] Rust workspace decision.
- [x] SvelteKit package manager decision.
- [ ] API contract generation strategy.

## Phase 1 - Backend Foundation

- [x] Rust workspace.
- [x] Axum health endpoint.
- [x] Config modes: `web`, `desktop`, `demo`.
- [x] SQLx migrations.
- [x] SQLite local development database.
- [x] PostgreSQL-ready schema.
- [x] Typed API errors.
- [x] Job event contract.
- [x] Job state machine.
- [x] Job read/cancel endpoints.

## Phase 2 - BYOK and Provider Layer

- [ ] Provider abstraction.
- [ ] OpenAI-compatible client.
- [ ] Anthropic client.
- [x] llama.cpp local client.
- [x] Local LLM health endpoint.
- [x] Local LLM model listing endpoint.
- [x] Local LLM chat completion endpoint.
- [x] Local model picker and preset download manager.
- [ ] Session-only BYOK.
- [ ] Masked key display.
- [ ] Usage accounting.
- [ ] Secret redaction tests.

## Phase 3 - Import and Chapter Splitting

- [ ] TXT/Markdown upload.
- [x] Chapter split preview.
- [x] Confirm import flow.
- [x] Source text storage.
- [ ] Regression fixtures.

## Phase 4 - Evidence-First Extraction

- [x] Chapter extraction schema.
- [ ] Evidence span validation.
- [ ] Observation persistence.
- [ ] Review item generation.
- [x] Local draft extraction endpoint.
- [x] Character extraction persistence event for realtime UI sync.
- [ ] Old-vs-new sample regression harness.
- [ ] Agentic extraction run contract without human approval gates.
- [ ] Stale markers for observations and evidence after raw source edits.

## Phase 5 - Parallel Translation

- [x] Source segment model shared by analysis and translation.
- [x] Translation job model.
- [x] Translation segment persistence.
- [x] Glossary entry model.
- [x] Style guide model.
- [x] Translation review items.
- [ ] Side-by-side source/target reading plan.
- [ ] Translation quality checks.
- [ ] Parallel analysis/translation scheduler with dependency-aware resume.
- [ ] Translation stale markers after raw text, glossary, alias, or entity edits.

## Phase 6 - Minimal Workspace UI

- [x] SvelteKit shell.
- [x] Project/bookshelf view.
- [x] Typed API client and aggregate workspace snapshot wiring.
- [x] Project create action.
- [x] Project archive or purge action.
- [x] Import preview/confirm wiring.
- [x] Reading view.
- [x] Reading font size and line-height settings.
- [x] Analysis progress view.
- [x] Analysis cancel action.
- [x] BYOK settings view.
- [x] Local llama.cpp settings view.
- [x] Light, dark, and system color modes.
- [x] Review route placeholder.
- [ ] App theme presets beyond color mode.
- [x] Reading workspace auto-sync for character extraction highlights.
- [ ] Realtime event streaming client.
- [ ] Review-item API integration.
- [ ] Inline editing foundation for chapter text, entities, aliases, relationships, glossary, and translation segments.
- [ ] Double-click edit, blur/Enter save, Escape cancel, and optimistic DB sync.
- [ ] Copy catalog/i18n foundation so UI strings are not hardcoded.
