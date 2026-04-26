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
- [ ] License selection.
- [ ] Rust workspace decision.
- [ ] SvelteKit package manager decision.
- [ ] API contract generation strategy.

## Phase 1 - Backend Foundation

- [ ] Rust workspace.
- [ ] Axum health endpoint.
- [ ] Config modes: `web`, `desktop`, `demo`.
- [ ] SQLx migrations.
- [ ] SQLite local development database.
- [ ] PostgreSQL-ready schema.
- [ ] Typed API errors.
- [ ] Job event contract.

## Phase 2 - BYOK and Provider Layer

- [ ] Provider abstraction.
- [ ] OpenAI-compatible client.
- [ ] Anthropic client.
- [ ] llama.cpp local client.
- [ ] Session-only BYOK.
- [ ] Masked key display.
- [ ] Usage accounting.
- [ ] Secret redaction tests.

## Phase 3 - Import and Chapter Splitting

- [ ] TXT/Markdown upload.
- [ ] Chapter split preview.
- [ ] Confirm import flow.
- [ ] Source text storage.
- [ ] Regression fixtures.

## Phase 4 - Evidence-First Extraction

- [ ] Chapter extraction schema.
- [ ] Evidence span validation.
- [ ] Observation persistence.
- [ ] Review item generation.
- [ ] Old-vs-new sample regression harness.

## Phase 5 - Minimal Workspace UI

- [ ] SvelteKit shell.
- [ ] Project/bookshelf view.
- [ ] Reading view.
- [ ] Analysis progress view.
- [ ] BYOK settings view.
- [ ] Review queue view.

