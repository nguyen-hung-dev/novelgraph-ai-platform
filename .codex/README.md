# Codex Operating Guide

This directory contains project-specific operating context for AI coding agents working on NovelGraph AI Platform.

Read order:

1. `AGENTS.md`
2. `.codex/project-context.md`
3. `.codex/implementation-rules.md`
4. Relevant phase task in `.codex/tasks/`
5. Relevant prompt contract or checklist

Keep this directory free of secrets, API keys, private model credentials, user uploads, and generated databases.

## Mission

Build a hybrid web/desktop AI novel analysis platform:

- Web: hosted multi-user workspace with BYOK LLM keys.
- Desktop: Tauri app with local storage and local AI.
- Shared UI: dense desktop-style workspace.
- Core pipeline: evidence-first extraction and reviewable projections.

## Current Priority

Phase 0 foundation:

- Establish repo structure.
- Define contracts and ADRs.
- Scaffold Rust backend and SvelteKit UI after stack decisions are locked.
- Do not start complex graph/map/timeline rendering yet.

