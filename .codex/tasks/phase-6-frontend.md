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
- [x] Reading workspace auto-sync for newly persisted character extraction highlights.
- [ ] Realtime job event client.
- [ ] Copy catalog/i18n registry for user-facing UI strings.
- [ ] Inline edit component for short domain fields.
- [ ] Reading raw text editor flow that writes directly to DB.
- [ ] Inline edit API wiring for entity aliases, relationships, glossary terms, and translation segments.
- [ ] Shared contract package in `packages/`.

## Next Focus

- Keep using the aggregate `/api/projects/{project_id}/workspace` snapshot instead of scattering many small reads across the UI.
- Treat realtime as a required UX standard: UI-visible DB writes must become visible without manual refresh.
- Add realtime job event streaming on top of the current persisted event history, then replace short-interval snapshot invalidation bridges.
- Replace the review placeholder once observation persistence and review-item APIs exist.
- Add double-click inline editing for visible domain data; blur or Enter saves, Escape cancels.
- Keep user corrections persisted through typed APIs and reconcile UI with the DB response.
- Avoid hardcoded user-facing text in components as new UI surfaces are added.
- Keep UI state local and modular; do not collapse API logic into one file.
- Preserve the same layout conventions for future Tauri reuse.
