# Changelog

All notable changes to this project will be documented in this file.

This project follows semantic versioning while it is still pre-1.0.0. Version `0.x.y` changes may still include breaking changes while the architecture is being established.

## Unreleased

No unreleased changes yet.

## [0.8.0] - 2026-04-27

### Added

- Added a managed local llama.cpp runtime with API endpoints to inspect runtime state, pick an existing GGUF file from disk, start the selected model, stop the local server, activate a managed model from the repo `models/` directory, and download preset models into that directory.
- Added a native file-picker flow for local GGUF selection so existing model files can be used in place without being copied into the repository.
- Added preset download support for a few small GGUF models and background runtime state tracking for queued, downloading, completed, and failed download states.
- Added a live Settings UI for local LLM runtime control, preset downloads, repo model library activation, and health/runtime status display.
- Added `LLAMA_CPP_SERVER_BIN` config support for choosing the `llama-server` executable path.
- Added support for preferring a local `tools/llama.cpp/llama-server.exe` bundle when no explicit runtime binary is configured.

### Changed

- Updated the Settings route from a mock runtime surface to a real API-backed local model manager with clearer runtime feedback while `llama-server` is starting.
- Disabled the automatic GitHub Actions CI workflow because GitHub is rejecting runner startup for the account due to a billing lock; local verification remains the gate for now.
- Updated app version metadata to `0.8.0`.
- Kept storage schema version at `2026-04-27.foundation.v3` because no database schema changes were added in this slice.

### Fixed

- Changed local runtime start failures such as a missing `llama-server` binary to return `424 Failed Dependency` instead of `500 Internal Server Error`.
- Fixed SvelteKit form actions that treated successful redirects as failures after starting a selected local model, creating/deleting/restoring projects, or confirming an import.

## [0.7.0] - 2026-04-27

### Added

- Added project deletion modes in the backend: archive from the bookshelf while retaining DB rows, or purge the entire project dataset from the database.
- Added SQLite and PostgreSQL `0003_project_retention` migrations with project-level soft deletion support.
- Added a delete-project modal to the bookshelf cards with a retention checkbox and explicit DB warning copy.
- Added global light, dark, and system color-mode controls in the workspace top bar.
- Added reading preferences with a settings modal for font size and line height, persisted locally per project.

### Changed

- Updated project listing and project reads to hide archived projects by default.
- Updated the reading view to render with configurable typography values instead of a fixed font size and line height.
- Updated app version metadata to `0.7.0`.
- Updated storage schema version to `2026-04-27.foundation.v3`.
- Added roadmap and checklist notes for future app theme presets beyond the current color-mode controls.

## [0.6.0] - 2026-04-26

### Added

- Added `GET /api/projects/{project_id}/workspace` to return one aggregate workspace snapshot for the active project shell.
- Added frontend shared API response types and a server-only typed API client for projects, workspace snapshots, import preview/confirm, and analysis job cancellation.
- Added SvelteKit server loads and form actions for bookshelf loading, project creation, import preview/confirm, and analysis job cancellation.
- Added live workspace bookshelf metrics and project cards backed by Rust API data instead of demo-only state.

### Changed

- Updated the project overview, reading, analysis, import, and review routes to use real project, chapter, and job-event data where the backend already supports it.
- Replaced the mock review queue screen with an explicit placeholder that matches the current backend boundary.
- Updated the Windows `dev-stack` launcher to set `API_BASE_URL`, `PUBLIC_API_BASE_URL`, and `VITE_API_BASE_URL` for frontend API wiring.
- Updated app version metadata to `0.6.0`.
- Updated roadmap, implementation plan, frontend checklist, README files, development guide, API contract, and `.codex` guidance to reflect the live UI wiring milestone.

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

- Updated `AGENTS.md`, `.codex/README.md`, `.codex/implementation-rules.md`, phase task validation, and release readiness checklist so future release-worthy changes must update `CHANGELOG.md` and review version metadata.
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
