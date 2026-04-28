# Codex Operating Guide

This directory contains project-specific operating context for AI coding agents working on NovelGraph AI Platform.

Read order:

1. `AGENTS.md`
2. `.codex/project-context.md`
3. `.codex/implementation-rules.md`
4. `.codex/versioning.md`
   - For version bumps, release commits, GitHub pushes, tags, or release notes, also read `.codex/skills/novelgraph-release/SKILL.md`.
5. `docs/module-architecture.md`
6. `docs/checklists/11-module-refactor-checklist.md` when touching large files or moving module boundaries
7. Relevant phase task in `.codex/tasks/`
8. Relevant prompt contract or checklist

Keep this directory free of secrets, API keys, private model credentials, user uploads, and generated databases.

## Mission

Build a hybrid web/desktop AI novel analysis platform:

- Web: hosted multi-user workspace with BYOK LLM keys.
- Desktop: Tauri app with local storage and local AI.
- Shared UI: dense desktop-style workspace.
- Core pipeline: evidence-first extraction, parallel translation, and reviewable projections.
- Automation: analysis and translation should run as agentic pipelines without human approval gates.
- Editing: visible domain data should support direct inline correction that persists to DB.
- Change hygiene: meaningful feature milestones and release boundaries must account for changelog and version metadata.
- Module hygiene: hand-written files have a soft limit of 800 lines and a hard limit of 1200 lines; new feature logic should follow `docs/module-architecture.md`.

## Current Priority

Foundation slices in progress:

- Keep `CHANGELOG.md`, `VERSION`, package manifests, workspace `Cargo.toml`, and `crates/core/src/version.rs` aligned when a planned release or major milestone changes version.
- Continue durable job orchestration before adding broad provider execution.
- Prioritize local llama.cpp execution before hosted BYOK provider execution.
- Keep the local llama.cpp settings flow usable: existing GGUF files run in place, preset models download into repo `models/`, and `llama-server` control stays API-backed.
- Keep the SvelteKit workspace shell on the typed API path that now powers bookshelf, import, reading, and analysis screens.
- Add copy/prompt registries before adding more user-facing strings or provider prompts; do not hardcode long UI/prompt text in feature code.
- Split hard-limit files before adding more workflow logic. Current priority targets are `crates/api/src/lib.rs`, `crates/storage/src/sqlite.rs`, `crates/core/src/extraction.rs`, and large Svelte route files such as Reading.
- Design inline editing so double-click edits, blur or Enter saves, and Escape cancels across reading, entity, glossary, and translation surfaces.
- Prefer aggregate workspace reads first, then add realtime events and reusable split-pane components on top.
- Keep the web shell visually aligned with the future desktop shell.
- Do not start complex graph/map/timeline rendering yet.
