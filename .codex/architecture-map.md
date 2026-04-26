# Architecture Map

## Runtime Modes

```text
web
  SvelteKit app -> Rust API -> PostgreSQL/object storage -> BYOK provider proxy

desktop
  Tauri app -> local Rust API/core -> SQLite/local files -> llama.cpp sidecar

demo
  SvelteKit app -> static precomputed datasets
```

## Core Domains

- Identity: users, sessions, workspaces, roles.
- Project: project metadata, privacy, sharing.
- Novel: uploads, source text, chapters.
- Analysis: jobs, runs, prompt calls, model usage.
- Extraction: observations, evidence spans, review items.
- Translation: source segments, translation segments, glossary, style profiles.
- Knowledge: entities, aliases, mentions, relationships.
- World: locations, spatial edges, map projections.
- Story: timeline events, scenes, factions.
- RAG: memory chunks, embeddings, retrieval traces.
- Export: `.air`, Markdown, docx/xlsx/pdf later.

## Suggested Rust Crates

```text
crates/core
  domain models, schemas, validation

crates/storage
  SQLx repositories and migrations

crates/ai
  provider abstraction, BYOK proxy, llama.cpp client

crates/jobs
  durable job queue and progress events

crates/api
  Axum routers and websocket/sse endpoints
```

## Suggested Apps

```text
apps/web
  SvelteKit UI

apps/desktop
  Tauri shell wrapping the same UI
```
