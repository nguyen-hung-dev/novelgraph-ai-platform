# Implementation Rules

## General

- Prefer small vertical slices over broad scaffolding.
- Keep documentation updated when architecture changes.
- Add or update ADRs for consequential decisions.
- Follow `docs/module-architecture.md` for directory ownership, dependency direction, and split targets.
- Do not commit secrets, model files, databases, uploads, or generated exports.
- Use ASCII for code, identifiers, commands, slugs, file paths, and protocol examples where practical.
- Vietnamese prose must be written with proper Vietnamese diacritics. Do not write unaccented Vietnamese in documentation or user-facing text.
- Do not hardcode user-facing UI text, long status copy, prompt text, translation templates, provider preset descriptions, or model recommendation copy inside feature code.
- Route paths, DB enum values, migration ids, protocol tokens, test fixtures, and internal slugs may remain literal when they are technical contracts.

## Module Size and Ownership

- Hand-written source and docs have a soft limit of 800 lines and a hard limit of 1200 lines.
- Files at or above 800 lines need a split plan before accepting new feature logic.
- Files at or above 1200 lines are hard-limit files. Only narrow bug fixes are allowed there until route, service, repository, component, or domain modules are extracted.
- Generated files, lockfiles, migrations, fixtures, and cumulative changelogs may exceed the limit when they are clearly marked by their role.
- Do not create new "god files" such as all-domain API clients, all-domain repositories, or route pages that own state, markup, API calls, and domain transformation together.
- Prefer module names based on domain ownership: `byok`, `analysis`, `reading`, `import`, `projects`, `translation`, `local-runtime`, `story`, `jobs`.
- When touching `crates/api/src/lib.rs`, `crates/storage/src/sqlite.rs`, `crates/core/src/extraction.rs`, or large Svelte routes, check `docs/checklists/11-module-refactor-checklist.md` and extract modules before expanding behavior.

## Changelog and Versioning

- Feature milestones, release preparation, public API changes, storage schema changes, migrations, breaking behavior, release behavior, and meaningful user-visible behavior must update `CHANGELOG.md`.
- Small bug fixes, UI polish, dev-only adjustments, test-only changes, and documentation clarifications should be batched into the active milestone or `Unreleased`; do not create a new version section for each small change.
- Version metadata changes only when preparing a planned release, a major milestone, or an explicit hotfix release.
- Keep these version locations aligned when a version changes:
  - `VERSION`
  - workspace package `version` in `Cargo.toml`
  - root and app `package.json` versions
  - `crates/core/src/version.rs`
  - `README.md` and `README.vi.md` current version text
- `/health` must expose the app version and relevant API/storage schema version metadata.
- Prefer adding changes to `Unreleased` during active development. Move them into a concrete version section only when preparing a release or explicit version milestone.

## Backend

- Rust backend should use explicit domain modules rather than one large service object.
- API route handlers should stay thin. Move use cases to `services/*` and keep app/router setup separate from extraction, storage, provider, and realtime details.
- Long-running analysis must run as jobs, not as request-bound handlers.
- All job progress should emit typed events.
- Storage code must account for both SQLite and PostgreSQL.
- Keep API errors typed and user-safe.
- Never log API keys or raw provider auth headers.

## Frontend

- Build the actual workspace UI first, not a landing page.
- Keep web and desktop UI as close as possible.
- Use dense operational layouts: sidebars, tabs, split panes, tables, progress panels.
- Avoid decorative hero sections for the app surface.
- Keep graph/map/timeline renderers framework-independent where practical.
- Avoid putting API logic in one giant file.
- Keep `+page.svelte` files as route composition. Move feature panels, presenters, and local state machines into `src/lib/features/<feature>/`.
- Prefer inline editing at the point where data is displayed. Double-click enters edit mode, blur or Enter saves, and Escape cancels.
- User edits must call typed APIs, persist directly to DB, and refresh or invalidate dependent projections.
- Avoid modal-based editing for short fields such as names, aliases, relationship labels, glossary terms, and status notes.

## AI and Extraction

- Every extracted fact must have evidence or a clear reason why it is inferred.
- Favor schema-constrained output.
- Do not feed future chapters into current-chapter extraction.
- Treat uncertain facts as review items.
- Track provider, model, token usage, and trace id for each LLM call.
- Build analysis and translation as autonomous agentic pipelines. Human review can correct data, but it must not be required for the pipeline to advance.
- Store structured observations, evidence, entities, relationships, glossary terms, and translation segments as the source of truth; raw LLM output is audit/debug data only.
- When users edit source text, aliases, entities, glossary entries, relationships, or translation segments, mark affected downstream data stale.
- Keep prompts in a versioned prompt registry rather than inline in handlers or UI components.

## BYOK

- Do not store API keys in browser local storage.
- Backend must proxy provider requests.
- Keys must be masked in UI.
- Persistent keys require encryption at rest.
- Session-only key mode should come first.
