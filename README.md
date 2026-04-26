# NovelGraph AI Platform

Desktop-style AI novel analysis for the web and local desktop.

[Vietnamese README](README.vi.md)

Current foundation version: `0.3.0`.

## License

This repository is source-available under the PolyForm Noncommercial License 1.0.0. Noncommercial use, study, modification, and redistribution are allowed under the license terms. Commercial use is prohibited unless you obtain a separate commercial license from the copyright holder.

NovelGraph AI Platform is a planned rewrite of an AI-powered novel analysis workspace. It will turn long-form fiction into character knowledge graphs, story maps, timelines, entity encyclopedias, scene indexes, and retrieval-augmented chat.

The product direction is hybrid:

- Hosted web app with user-owned API keys (BYOK).
- Local desktop app with offline storage and local AI support.
- One shared desktop-style interface across web and desktop.

## Keywords

AI novel analysis, novel knowledge graph, story map generator, character relationship graph, timeline visualization, local AI reader, BYOK AI app, fiction analysis platform, RAG novel chat, Tauri desktop AI.

## Product Goals

- Import TXT/Markdown novels and split chapters reliably.
- Extract grounded facts from each chapter with evidence spans.
- Build entity profiles for characters, locations, organizations, items, and concepts.
- Generate relationship graphs, world maps, timelines, factions, scene indexes, and encyclopedias.
- Translate novels in parallel with analysis while preserving glossary and source alignment.
- Support private hosted projects and offline desktop projects.
- Let web users bring their own LLM API key instead of forcing platform-funded inference.
- Keep the UI dense, practical, and workspace-oriented like a desktop tool.

## Proposed Stack

| Layer | Direction |
|---|---|
| Frontend | SvelteKit 2 + Svelte 5 + TypeScript |
| Desktop | Tauri 2 |
| Backend | Rust + Axum + Tokio |
| Database | SQLite for desktop, PostgreSQL for hosted web |
| Search/RAG | SQLite FTS/vector locally, PostgreSQL full-text + pgvector or Qdrant on web |
| AI Web | BYOK proxy for OpenAI-compatible providers and Anthropic |
| AI Desktop | llama.cpp `llama-server` with GGUF models |
| Storage Web | S3/R2/MinIO-compatible object storage |

## Architecture Direction

The new system should be evidence-first. LLM output should not be the primary source of truth. Instead, extraction should produce observations linked to source chapter spans, and projections should build UI-ready graph/map/timeline data from those observations.

```text
Import -> Split -> Prescan -> ExtractChapter[n] -> Normalize -> Aggregate
       -> IndexRAG -> BuildWorld -> BuildTimeline -> BuildVisualCache -> Review
```

Deployment modes:

- `desktop`: Tauri shell, local Rust backend, SQLite, local files, optional llama.cpp sidecar.
- `web`: hosted SvelteKit app, Rust API, PostgreSQL, object storage, BYOK LLM proxy.
- `demo`: static precomputed datasets for public browsing.

## Repository Layout

```text
apps/
  web/                 # Future SvelteKit app
  desktop/             # Future Tauri app shell
crates/
  api/                 # Future Axum API crate
  core/                # Domain models, jobs, extraction contracts
  storage/             # SQLite/PostgreSQL adapters
packages/
  schemas/             # Shared JSON schemas/OpenAPI generated types
docs/
  implementation-plan.md
  security-byok.md
  adr/
```

## First Milestone

The first implementation milestone should not start with visualization. It should establish the foundation:

- Workspace/project schema for desktop and web.
- Auth boundary for web.
- BYOK secret model and provider abstraction.
- Import + chapter splitting.
- Durable analysis job queue.
- One extraction contract with evidence spans.
- WebSocket/SSE progress events.

## Current Status

Planning repository initialized. The Rust backend foundation now includes `core`, `storage`, `jobs`, `ai`, and `api` crates, an Axum `/health` endpoint, SQLite migrations/repository code, project endpoints, novel import preview/confirm, source segment persistence, pending analysis jobs, translation job creation, job state validation, cancellation endpoints, persisted job events, and local llama.cpp health/models/chat endpoints. The product application is not usable yet.

See:

- [Implementation plan](docs/implementation-plan.md)
- [Product requirements](docs/product-requirements.md)
- [Roadmap](ROADMAP.md)
- [Implementation checklists](docs/checklists/README.md)
- [Development guide](docs/development.md)
- [API contract](docs/api-contract.md)
- [Data model](docs/data-model.md)
- [Deployment model](docs/deployment.md)
- [Testing strategy](docs/testing-strategy.md)
- [Parallel translation design](docs/translation/README.md)
- [BYOK security notes](docs/security-byok.md)
- [ADR 0001: Hybrid web and desktop stack](docs/adr/0001-hybrid-web-desktop-stack.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
