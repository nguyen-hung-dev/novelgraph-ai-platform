# Phase 6 - Frontend Workspace Tasks

Goal: turn the SvelteKit shell into a real client for the Rust backend without losing desktop-style density.

## Current State

- [x] `apps/web` scaffolded with SvelteKit, TypeScript, lint, unit test, and Node adapter.
- [x] Root `pnpm` workspace configured.
- [x] Workspace shell with sidebar, top toolbar, and project tabs.
- [x] Bookshelf, overview, import, reading, analysis, review, settings, and BYOK routes.
- [x] API-backed bookshelf, project overview, import, reading, and analysis surfaces.
- [x] Typed API client.
- [x] Request and error surface for current server actions.
- [x] Bookshelf delete modal with archive or purge modes.
- [x] Light, dark, and system color modes.
- [x] Reading typography settings persisted locally.
- [x] Local llama.cpp settings with runtime state, GGUF picker, preset downloads, and repo model activation.
- [ ] Realtime job event client.
- [ ] Shared contract package in `packages/`.

## Next Focus

- Keep using the aggregate `/api/projects/{project_id}/workspace` snapshot instead of scattering many small reads across the UI.
- Add realtime job event streaming on top of the current persisted event history.
- Replace the review placeholder once observation persistence and review-item APIs exist.
- Keep UI state local and modular; do not collapse API logic into one file.
- Preserve the same layout conventions for future Tauri reuse.
