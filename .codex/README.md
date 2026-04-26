# Codex Operating Guide

This directory contains project-specific operating context for AI coding agents working on NovelGraph AI Platform.

Read order:

1. `AGENTS.md`
2. `.codex/project-context.md`
3. `.codex/implementation-rules.md`
4. `.codex/versioning.md`
5. Relevant phase task in `.codex/tasks/`
6. Relevant prompt contract or checklist

Keep this directory free of secrets, API keys, private model credentials, user uploads, and generated databases.

## Mission

Build a hybrid web/desktop AI novel analysis platform:

- Web: hosted multi-user workspace with BYOK LLM keys.
- Desktop: Tauri app with local storage and local AI.
- Shared UI: dense desktop-style workspace.
- Core pipeline: evidence-first extraction and reviewable projections.
- Change hygiene: meaningful feature milestones and release boundaries must account for changelog and version metadata.

## Current Priority

Foundation slices in progress:

- Keep `CHANGELOG.md`, `VERSION`, package manifests, workspace `Cargo.toml`, and `crates/core/src/version.rs` aligned when a planned release or major milestone changes version.
- Continue durable job orchestration before adding broad provider execution.
- Prioritize local llama.cpp execution before hosted BYOK provider execution.
- Keep the local llama.cpp settings flow usable: existing GGUF files run in place, preset models download into repo `models/`, and `llama-server` control stays API-backed.
- Keep the SvelteKit workspace shell on the typed API path that now powers bookshelf, import, reading, and analysis screens.
- Prefer aggregate workspace reads first, then add realtime events and reusable split-pane components on top.
- Keep the web shell visually aligned with the future desktop shell.
- Do not start complex graph/map/timeline rendering yet.
