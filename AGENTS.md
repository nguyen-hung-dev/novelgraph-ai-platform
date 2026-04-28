# Repository Guidelines

## Project Direction

NovelGraph AI Platform is a new hybrid web/desktop rewrite. Keep the product as a dense desktop-style workspace, not a landing page.

Before substantial implementation work, read `.codex/README.md`, `.codex/project-context.md`, and `.codex/implementation-rules.md`. For phase-specific work, also read the matching file in `.codex/tasks/`.

For module layout, file-size governance, and refactor sequencing, read `docs/module-architecture.md` and `docs/checklists/11-module-refactor-checklist.md`.

Primary goals:

- Hosted web app with BYOK LLM keys.
- Tauri desktop app with local/offline mode.
- Evidence-first extraction pipeline.
- Agentic analysis and translation that can run without human approval gates.
- Rust backend, SvelteKit frontend, shared typed contracts.

## Structure

- `apps/`: future runnable apps, such as `web/` and `desktop/`.
- `crates/`: future Rust crates for API, core domain, storage, jobs, and AI providers.
- `packages/`: future shared generated schemas/types.
- `docs/`: architecture, implementation plan, ADRs, and security notes.

Target module layout is documented in `docs/module-architecture.md`. Use it as the source of truth when adding new directories or splitting large files.

## Engineering Rules

- Do not put secrets in the repo.
- Do not store BYOK API keys in browser local storage.
- Vietnamese text must be written with proper Vietnamese diacritics. Do not write Vietnamese as unaccented ASCII unless it is an identifier, slug, file path, command, or code token.
- Do not hardcode user-facing copy, prompt text, translation templates, provider preset descriptions, or long status messages in feature code.
- Prefer typed schemas and generated clients over hand-written ad hoc API shapes.
- User-visible DB writes must have a realtime sync path; do not make manual page refresh the primary way to see newly persisted data.
- Keep renderers for graph/map/timeline independent from the UI framework where practical.
- Start with minimal vertical slices before porting complex visualizations.
- Develop features in small manual-testable slices. Do not bundle unrelated work into one large pack.
- Apply the file-size budget to hand-written source and docs: soft limit 800 lines, hard limit 1200 lines. Do not add feature logic to files over the hard limit. Files over the soft limit need a split plan or an in-scope extraction.
- When a feature touches `crates/api/src/lib.rs`, `crates/storage/src/sqlite.rs`, `crates/core/src/extraction.rs`, or a large `+page.svelte`, prefer extracting route/service/repository/component modules before adding more workflow logic.
- The user should be able to manually test each meaningful feature before the next feature slice starts.
- Avoid running broad automated test commands unless the user explicitly asks for them. Use only lightweight checks when necessary and say what was checked.
- Prefer inline editing for visible domain data. Double-click enters edit mode, blur or Enter saves to DB through typed API, and Escape cancels.
- Any user correction to raw chapter text, entity aliases, relationships, glossary terms, or translation segments must update DB and mark dependent data stale.
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
