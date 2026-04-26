# Roadmap

This roadmap is intentionally foundation-first. The project should not rush into complex visualizations before the data and job model are stable.

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
- [ ] Old-vs-new sample regression harness.

## Phase 5 - Parallel Translation

- [x] Source segment model shared by analysis and translation.
- [x] Translation job model.
- [x] Translation segment persistence.
- [x] Glossary entry model.
- [x] Style guide model.
- [x] Translation review items.
- [ ] Side-by-side source/target reading plan.
- [ ] Translation quality checks.

## Phase 6 - Minimal Workspace UI

- [x] SvelteKit shell.
- [x] Project/bookshelf view.
- [x] Reading view.
- [x] Analysis progress view.
- [x] BYOK settings view.
- [x] Review queue view.
