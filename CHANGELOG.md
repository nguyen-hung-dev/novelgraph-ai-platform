# Changelog

All notable changes to this project will be documented in this file.

This project follows semantic versioning while it is still pre-1.0.0. Version `0.x.y` changes may still include breaking changes while the architecture is being established.

## Unreleased

No unreleased changes yet.

## [0.12.0] - 2026-04-29

### Changed

- Split the API crate into route and service modules for health, local LLM, BYOK, projects, realtime, novels, jobs, translation, and analysis while keeping router wiring in `lib.rs`.
- Moved long analysis pipeline helpers into focused services for stepping, pipeline execution, relationship extraction, identity resolution, alias handling, mention scanning, field verification, document assembly, and local LLM JSON repair.
- Split SQLite analysis, BYOK, story-alias, and storage smoke-test logic into focused modules so legacy storage files are below the hard file-size limit and can continue shrinking by domain.
- Added a repo-scoped `novelgraph-release` Codex skill and linked it from the agent operating guide for version, changelog, commit, push, tag, and release workflows.
- Updated the module refactor checklist with completed API/storage split work and remaining soft-limit follow-up items.
- Updated app version metadata to `0.12.0`.
- Kept storage schema version at `2026-04-29.foundation.v9` because this release does not add database migrations.

## [0.11.0] - 2026-04-29

### Changed

- Added persisted BYOK settings for Google Gemini on the Settings page, including encrypted per-user DB storage, masked API key display, Gemini model dropdown presets, and a backend key health check.
- Added a creation-review gate before new character records are persisted and tightened alias ownership evidence checks so ambiguous surfaces are rejected or merged before they can poison the cross-chapter alias map.
- Added a field-value verification gate before character fields are persisted so appearance values must belong to the target character and classify as stable visual appearance, clothing, or build instead of action, state, emotion, relationship, or another character's detail.
- Added a relationship verification gate before relationship records are persisted so temporary interactions, shared events, co-presence, scene actions, and alias-only links are rejected instead of being stored as story graph relationships.
- Added current-novel metadata, including Genre and Description, to analysis prompt context so local AI can choose genre-appropriate labels and extraction style while still requiring chapter evidence.
- Tightened the character appearance field prompt so transient actions, emotions, attitudes, sounds, symptoms, expressions, relationship labels, and negative/no-data statements are rejected instead of being persisted as `Ngoại hình`.
- Changed relationship extraction to use a full-chapter candidate pass with grounded evidence and canonical character endpoint resolution instead of sending every non-duplicate character pair to the local LLM.
- Tightened the relationship candidate prompt so the local LLM infers story-specific relationship labels from current-chapter evidence without relying on a fixed taxonomy.
- Added a non-blocking character candidate coverage pass before identity extraction so local models get a broader alias checklist, alias evidence can be preserved, and candidate hints can merge into the stable identity pipeline without stopping the run when the candidate pass fails.
- Changed the Reading character detail panel to aggregate aliases, fields, and relationships across all analyzed chapters for the same canonical character while keeping highlights scoped to the currently open chapter.
- Added a high-confidence canonical character resolver that compares new identities against DB and in-memory records with accent-folded name/alias surfaces before creating a new character record.
- Added a conservative AI confirmation step for ambiguous character identity merges, returning `merge_existing`, `create_new`, or `ignore` without blocking the analysis run when the local model cannot parse the confirmation JSON.
- Added a persisted character alias map that materializes canonical names and aliases per analysis job, exposes it through workspace/run snapshots, and lets the Reading info panel prefer the alias map with a field-based fallback for older data.
- Changed character identity resolution to prefer the persisted alias map for exact alias matches, high-confidence canonical matching, and ambiguous merge candidates while retaining field-based fallback for older runs.
- Added per-chapter analysis status dots to the Reading chapter list, backed by workspace chapter-run state.
- Fixed character extraction validation so cross-chapter alias evidence used by the canonical resolver is not persisted into the current chapter document with the wrong `chapter_num`.
- Tightened canonical character merging so a new name is not auto-merged into an existing character only because it matches a stored alias; candidate coverage now adds missing names only and does not directly persist candidate aliases.
- Split character identity into non-overlapping passes: identity/candidate now create character nodes only, while the new alias ownership pass is the single owner for same-sentence alias/coreference merges before cross-chapter resolution and receives a quoted-surface checklist without backend phrase matching.
- Replaced Reading/Analysis workspace polling with project WebSocket events so analysis, extraction, and relationship updates can invalidate the UI when the backend persists new data.
- Optimized ambiguous mention confirmation by sampling only a few representative occurrences per character surface before accepting all stable matches.
- Tightened character identity and alias ownership prompts so grammatical references, pronouns, temporary references, and possessive surfaces are used only for internal reasoning and are not persisted as aliases.
- Hydrated current-chapter character mention scanning with the persisted cross-chapter alias map so known names and aliases can be highlighted in later chapters without re-discovering them first.
- Added cross-chapter alias-map highlighting in Reading so known character names and aliases can be highlighted even before the selected chapter has completed analysis.
- Added backend and alias-map hygiene filters so unstable grammatical references and very short generic surfaces are not persisted or reused as character aliases without globally rejecting valid names that contain possessive particles.
- Added a backend persist gate for character fields so appearance values require high confidence, target-marked evidence, and lexical grounding in the quoted evidence before they are written to DB.
- Tightened alias, identity, and relationship persistence guards so unstable alias reference classes are rejected, substring-only character identities are removed before canonical resolution, alias ownership can redirect to a clearly closer character surface, and relationship candidates must declare a stable relationship kind with sufficient confidence before DB write.
- Added one JSON repair retry for local LLM structured-output calls when the first response cannot be parsed as a JSON array.
- Added local JSON string sanitation so raw control characters emitted inside local LLM JSON strings are escaped before parsing.
- Added novel-level metadata fields for genre and description, source-language auto detection, manual metadata updates, and local-AI metadata filling for imported novels.
- Updated app version metadata to `0.11.0`.
- Updated storage schema version to `2026-04-29.foundation.v9`.

## [0.10.0] - 2026-04-27

### Changed

- Added a persistent collapsible workspace sidebar rail so the desktop workspace can switch between full navigation and icon-only mode.
- Compact the workspace topbar into a single-row rail on desktop.
- Made the workspace sidebar sticky on desktop while preserving normal stacked behavior on mobile.
- Replaced the three-button color mode control with one cycling icon button for system, light, and dark modes.
- Simplified sidebar navigation labels by removing per-link metadata and moving Settings to the bottom gear item of the workspace sidebar.
- Merged BYOK controls into the main Settings grid and removed the standalone Settings page header.
- Changed character mention extraction so the local LLM no longer returns highlight offsets; the backend now scans confirmed character surfaces with Unicode character-boundary checks and only asks the local LLM to confirm ambiguous occurrence contexts.
- Simplified the character extraction pipeline back to three stable passes: identity aliases, backend-scanned mentions, and minimal character fields, while keeping relationships out of character fields for a separate extraction flow.
- Tightened the character fields prompt so each field request has one explicit target character and must return an empty array when evidence does not clearly belong to that target.
- Changed character field extraction to send target-marked context snippets instead of the full chunk so local models are less likely to assign another character's fields to the active target.
- Simplified the Reading character detail overlay by hiding confidence/reason text and keeping alias fields only in the mention chips.
- Added cross-chapter character identity resolution inside the analysis pipeline so later chapter names and aliases can merge into existing character records instead of creating unrelated chapter-local identities.
- Removed character `Ghi chú` and `Trạng thái` fields from the extraction pipeline; the current character field pass now keeps only clearly evidenced appearance fields.
- Documented the next candidate-based character identity merge pass for typo, near-name, and alias-pass failure cases before new character records are created.
- Updated app version metadata to `0.10.0`.
- Updated storage schema version to `2026-04-27.foundation.v6`.

## [0.9.0] - 2026-04-27

### Added

- Added persisted `analysis_chapter_runs` state so analysis jobs can track chapter-level pending, running, completed, and failed progress.
- Added analysis run API controls for snapshot reads, per-chapter stepping, manual pause, and force rerun reset.
- Added an Analysis screen runner with Start/Resume, Pause, Force rerun, progress metrics, and per-chapter status display.
- Added persisted character-only story extraction records with record, field, value, confidence, and evidence storage.
- Added Analysis screen chapter range inputs so manual runs can target one chapter or a chapter range.
- Added parsed character extraction results to the Analysis screen.
- Added Reading screen highlighting for AI evidence quotes from parsed character extraction records.
- Changed Reading screen highlighting to use character mention spans instead of generic field evidence quotes.
- Added a persisted character extraction event and Reading workspace auto-sync so new character highlights can appear without manual refresh.
- Added chunked character extraction so one chapter is processed as smaller LLM requests before records are merged and persisted.
- Added stop checks between character extraction chunks so Pause/Cancel can take effect before the next local LLM request starts.
- Added mention span repair for character extraction so invalid local LLM offsets are matched back to `mention.text` or dropped instead of failing the whole chunk.
- Added staged character extraction passes per chunk: identity and aliases first, persisted to DB, then DB-backed mention extraction, then DB-backed field extraction.
- Added llama.cpp `enable_thinking=false` request support for local JSON extraction passes.
- Added a standalone Python alias extraction test script that chunks a full chapter and writes JSON reports under `output/`.

### Changed

- Added a resumable `paused` job state for analysis jobs so local LLM or backend connection failures can stop the run without cancelling the job.
- Updated analysis execution to use the `story_character_extraction.v1` schema for the first focused extraction slice.
- Updated the character extraction prompt and persistence path to prefer and normalize ASCII snake_case `field_key` values.
- Updated the character extraction prompt to return offsets relative to each provided text chunk, then convert them back to full-chapter offsets in the backend.
- Updated the character extraction prompt to tell local models to omit uncertain mention offsets instead of guessing placeholder spans.
- Updated the character extraction prompt so highlights use minimal standalone character surface forms and relationship/context words move into fields instead of long mention phrases.
- Updated character extraction from one large all-in-one JSON prompt to smaller JSON array prompts for identity, mentions, and fields.
- Increased the local character field extraction token budget to reduce truncated JSON array responses from local models.
- Updated roadmap, implementation plan, API contract, frontend tasks, and extraction checklist to make realtime UI sync a required architecture direction.
- Updated app version metadata to `0.9.0`.
- Updated storage schema version to `2026-04-27.foundation.v5`.

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
