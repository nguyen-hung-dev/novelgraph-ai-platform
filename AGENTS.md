# Repository Guidelines

## Project Direction

NovelGraph AI Platform is a new hybrid web/desktop rewrite. Keep the product as a dense desktop-style workspace, not a landing page.

Before substantial implementation work, read `.codex/README.md`, `.codex/project-context.md`, and `.codex/implementation-rules.md`. For phase-specific work, also read the matching file in `.codex/tasks/`.

Primary goals:

- Hosted web app with BYOK LLM keys.
- Tauri desktop app with local/offline mode.
- Evidence-first extraction pipeline.
- Rust backend, SvelteKit frontend, shared typed contracts.

## Structure

- `apps/`: future runnable apps, such as `web/` and `desktop/`.
- `crates/`: future Rust crates for API, core domain, storage, jobs, and AI providers.
- `packages/`: future shared generated schemas/types.
- `docs/`: architecture, implementation plan, ADRs, and security notes.

## Engineering Rules

- Do not put secrets in the repo.
- Do not store BYOK API keys in browser local storage.
- Vietnamese text must be written with proper Vietnamese diacritics. Do not write Vietnamese as unaccented ASCII unless it is an identifier, slug, file path, command, or code token.
- Prefer typed schemas and generated clients over hand-written ad hoc API shapes.
- Keep renderers for graph/map/timeline independent from the UI framework where practical.
- Start with minimal vertical slices before porting complex visualizations.
- Document architecture decisions as ADRs in `docs/adr/`.
- For feature milestones, release preparation, public API changes, schema changes, migrations, or meaningful user-visible behavior, update `CHANGELOG.md` under `Unreleased` or the planned release section.
- Do not bump versions for every small bug fix, UI polish pass, dev-only adjustment, test-only change, or documentation clarification. Batch those changes into the active milestone or `Unreleased`.
- When a planned release or major milestone changes version, keep the relevant metadata aligned: root `VERSION`, workspace `Cargo.toml`, package manifests, README current-version text, and `crates/core/src/version.rs`.

## First Implementation Bias

Build in this order:

1. Backend health/config and storage migrations.
2. Auth/BYOK boundary for web.
3. Import and chapter splitting.
4. Durable jobs and realtime progress events.
5. First extraction contract with evidence spans.
6. Minimal workspace UI.
