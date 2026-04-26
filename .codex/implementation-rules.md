# Implementation Rules

## General

- Prefer small vertical slices over broad scaffolding.
- Keep documentation updated when architecture changes.
- Add or update ADRs for consequential decisions.
- Do not commit secrets, model files, databases, uploads, or generated exports.
- Use ASCII for code, identifiers, commands, slugs, file paths, and protocol examples where practical.
- Vietnamese prose must be written with proper Vietnamese diacritics. Do not write unaccented Vietnamese in documentation or user-facing text.

## Changelog and Versioning

- Every code change must update `CHANGELOG.md`.
- Documentation-only changes may skip a version bump, but should still update `CHANGELOG.md` when they change project behavior, setup, security posture, or release notes.
- Public API changes, storage schema changes, migrations, breaking behavior, release behavior, and user-visible behavior changes must also review version metadata.
- Keep these version locations aligned when a version changes:
  - `VERSION`
  - workspace package `version` in `Cargo.toml`
  - `crates/core/src/version.rs`
  - `README.md` and `README.vi.md` current version text
- `/health` must expose the app version and relevant API/storage schema version metadata.
- Prefer adding changes to `Unreleased` during active development. Move them into a concrete version section only when preparing a release or explicit version milestone.

## Backend

- Rust backend should use explicit domain modules rather than one large service object.
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

## AI and Extraction

- Every extracted fact must have evidence or a clear reason why it is inferred.
- Favor schema-constrained output.
- Do not feed future chapters into current-chapter extraction.
- Treat uncertain facts as review items.
- Track provider, model, token usage, and trace id for each LLM call.

## BYOK

- Do not store API keys in browser local storage.
- Backend must proxy provider requests.
- Keys must be masked in UI.
- Persistent keys require encryption at rest.
- Session-only key mode should come first.
