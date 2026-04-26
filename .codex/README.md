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
- Change hygiene: every code change must account for changelog and version metadata.

## Current Priority

Foundation slices in progress:

- Keep `CHANGELOG.md`, `VERSION`, workspace `Cargo.toml`, and `crates/core/src/version.rs` aligned when behavior changes.
- Continue durable job orchestration before adding broad provider execution.
- Prioritize local llama.cpp execution before hosted BYOK provider execution.
- Grow the SvelteKit workspace shell with typed API wiring, request tracing, and reusable split-pane components.
- Keep the web shell visually aligned with the future desktop shell.
- Do not start complex graph/map/timeline rendering yet.
