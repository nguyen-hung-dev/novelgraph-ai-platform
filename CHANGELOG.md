# Changelog

All notable changes to this project will be documented in this file.

This project follows semantic versioning while it is still pre-1.0.0. Version `0.x.y` changes may still include breaking changes while the architecture is being established.

## Unreleased

No unreleased changes yet.

## [0.5.0] - 2026-04-26

### Added

- Added draft chapter extraction prompt builder for evidence-first local testing.
- Added `draft.chapter_extraction.v0` schema version marker.
- Added local llama.cpp draft chapter extraction endpoint that returns prompt metadata and raw LLM response without persisting observations.
- Added prompt builder tests for current-chapter boundary and review item requirements.
- Added root `pnpm` workspace configuration and frontend scripts.
- Added a SvelteKit `apps/web` workspace with Node adapter, lint, typecheck, and unit test setup.
- Added a desktop-style workspace shell with bookshelf, overview, import, reading, analysis, review, settings, and BYOK routes.
- Added reading split-pane state with local reading-position persistence.
- Added draft import and BYOK form surfaces for later Rust API integration.
- Added frontend navigation unit tests for route matching.
- Added `scripts/dev-stack.ps1` and `scripts/dev-stack.bat` to launch backend and frontend together on Windows.
- Added automatic dev-port handling that restarts repo-owned listeners on preferred ports or picks the next free port when a different process is already listening.
- Added Windows job-object cleanup so the launcher stops child backend/frontend processes when the CLI session ends.

### Changed

- Updated app version metadata to `0.5.0`.
- Selected `pnpm` as the frontend package manager.
- Kept draft extraction non-mutating so local prompt quality can be evaluated before observation persistence.
- Updated roadmap, implementation plan, development guide, and frontend checklists to reflect the first UI foundation slice.
- Added root package scripts for `dev:stack` and `dev:stack:dry-run`.

## [0.3.0] - 2026-04-26

### Added

- Added `crates/ai` with local-first llama.cpp client support.
- Added OpenAI-compatible local chat completion request and response types.
- Added local LLM health, model listing, and chat completion endpoints.
- Added local LLM config fields for base URL, default model, and timeout.
- Added unit tests for local llama.cpp URL/config validation and OpenAI-compatible JSON shapes.

### Changed

- Updated app version metadata to `0.3.0`.
- Prioritized local llama.cpp integration before cloud BYOK provider execution.
- Kept storage schema version at `2026-04-26.foundation.v2` because no database schema changes were added for local LLM.

## [0.2.0] - 2026-04-26

### Added

- Added `crates/jobs` with job kind/status types and explicit state transition validation.
- Added SQLite and PostgreSQL `0002_job_state` migrations for job lifecycle timestamps and safe error fields.
- Added analysis job read and cancel APIs.
- Added translation job read and cancel APIs.
- Added persisted cancellation events for analysis and translation jobs.
- Added tests for job state transitions and job cancellation persistence.

### Changed

- Updated app version metadata to `0.2.0`.
- Updated storage schema version to `2026-04-26.foundation.v2`.
- Extended analysis and translation job response models with `started_at`, `finished_at`, `error_code`, and `error_message`.

## [0.1.1] - 2026-04-26

### Added

- Added root `VERSION` file as the single visible release version marker.
- Added backend version constants for app version, API version, release channel, and storage schema version.
- Added `/health` metadata for API version and storage schema version.
- Added `.codex/versioning.md` with version bump and changelog rules.

### Changed

- Updated `AGENTS.md`, `.codex/README.md`, `.codex/implementation-rules.md`, phase task validation, and release readiness checklist so future code changes must update `CHANGELOG.md` and review version metadata.
- Updated README files and API contract documentation with the current version and health response shape.

## [0.1.0] - 2026-04-26

### Added

- Added PolyForm Noncommercial 1.0.0 licensing and noncommercial use notices.
- Added parallel translation planning, glossary, and quality documents.
- Added layered implementation checklists for foundation, backend, BYOK, import, extraction, frontend, desktop, release readiness, and translation.
- Added initial Rust workspace with `core`, `storage`, and `api` crates.
- Added Axum `/health` endpoint with app mode and version metadata.
- Added initial SQLite and PostgreSQL foundation migrations.
- Added SQLite repository foundation for projects, novel import, chapter splitting, source segments, analysis jobs, translation jobs, and job events.
- Added foundation API endpoints for projects, novel import preview/confirm, novel/chapter reads, translation job creation, and job event history.
- Added chapter splitting support for English, Vietnamese, Chinese, Markdown headings, and no-heading fallback.
- Added tests for config parsing, chapter splitting, SQLite repository import, translation job creation, and job events.

### Changed

- Updated README files, roadmap, API contract, data model, development guide, product requirements, and testing strategy to reflect the backend foundation.
- Updated GitHub workflow to run Rust checks when backend files change.
- Updated `.gitignore` to keep generated data, model files, target build output, and local architecture analysis out of Git.

## [0.0.1] - 2026-04-26

### Added

- Initialized planning repository.
- Added architecture and rewrite analysis.
- Added hybrid web/desktop direction.
- Added BYOK security notes.
- Added implementation plan and roadmap.
- Added Codex operating context.
- Added GitHub workflow and issue/PR templates.
