# Phase 6 - Frontend Workspace Tasks

Goal: turn the SvelteKit shell into a real client for the Rust backend without losing desktop-style density.

## Current State

- [x] `apps/web` scaffolded with SvelteKit, TypeScript, lint, unit test, and Node adapter.
- [x] Root `pnpm` workspace configured.
- [x] Workspace shell with sidebar, top toolbar, and project tabs.
- [x] Bookshelf, overview, import, reading, analysis, review, settings, and BYOK routes.
- [x] Mock-driven split-pane reading and review surfaces.
- [ ] Typed API client.
- [ ] Request id and error surface.
- [ ] Realtime job event client.
- [ ] Shared contract package in `packages/`.

## Next Focus

- Replace mock bookshelf/project data with `/api/projects` and project detail endpoints.
- Attach reading and analysis screens to real chapters and job events.
- Keep UI state local and modular; do not collapse API logic into one file.
- Preserve the same layout conventions for future Tauri reuse.
