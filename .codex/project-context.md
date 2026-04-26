# Project Context

NovelGraph AI Platform is a new rewrite inspired by AI Reader V2.

The previous system had:

- Python FastAPI backend.
- React/Vite frontend.
- Tauri desktop shell.
- SQLite and ChromaDB.
- Ollama local LLM.
- Novel analysis pipeline with chapter extraction, entity profiles, graph, map, timeline, encyclopedia, RAG chat, export, and agentic review experiments.

The rewrite should not copy the old architecture directly. It should preserve the product value while fixing the foundation for:

- Hosted website.
- User-provided LLM API keys.
- Desktop/offline mode.
- Safer evidence-grounded extraction.
- Parallel translation with glossary and source alignment.
- Cleaner storage and job orchestration.

## Target Stack

- Frontend: SvelteKit 2 + Svelte 5 + TypeScript.
- Desktop: Tauri 2.
- Backend: Rust + Axum + Tokio.
- Storage:
  - Desktop: SQLite.
  - Web: PostgreSQL.
- AI:
  - Web: BYOK provider proxy.
  - Desktop: llama.cpp `llama-server`.
- Search/RAG:
  - Desktop: SQLite FTS and local vector adapter.
  - Web: PostgreSQL full-text + pgvector or Qdrant.

## Product Shape

The app is a workspace, not a marketing site.

Expected primary screens:

- Bookshelf/projects.
- Reading.
- Analysis progress.
- Graph.
- Map.
- Timeline.
- Encyclopedia.
- Chat.
- Translation.
- Review queue.
- Settings/BYOK.

## Data Principle

Do not make raw LLM JSON the source of truth.

Store:

- Source chapter text.
- Evidence spans.
- Observations.
- Entity aliases and mentions.
- Review decisions.
- Translation segments and glossary revisions.
- Generated projections/cache for UI.
