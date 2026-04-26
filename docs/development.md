# Development Guide

The Rust backend foundation is scaffolded. The product UI and AI workers are not implemented yet.

## Prerequisites

Expected tools:

- Rust stable.
- Node.js LTS.
- A package manager to be selected: `pnpm`, `npm`, or `bun`.
- PostgreSQL for hosted web development later.
- SQLite for desktop/local development.
- Optional llama.cpp server for local AI testing.

Local AI defaults:

```bash
LLAMA_CPP_BASE_URL=http://127.0.0.1:8080
LLAMA_CPP_DEFAULT_MODEL=qwen3
LLAMA_CPP_TIMEOUT_SECS=120
```

## Intended Layout

```text
apps/web
  SvelteKit app

apps/desktop
  Tauri shell

crates/api
  Axum routers

crates/core
  domain models and validation

crates/storage
  database adapters and migrations

crates/ai
  provider clients and BYOK proxy logic

crates/jobs
  durable job orchestration

packages/schemas
  generated shared types and API schemas
```

## First Local Commands

The backend currently defaults to SQLite. Without environment variables it creates `data/novelgraph.sqlite3`.

```bash
# backend
cargo fmt --all --check
cargo test --workspace
cargo run -p novelgraph-api
```

Useful local API smoke test flow:

```bash
curl http://127.0.0.1:3000/health
curl -X POST http://127.0.0.1:3000/api/projects \
  -H "content-type: application/json" \
  -d "{\"name\":\"Demo\"}"
curl http://127.0.0.1:3000/api/local-llm/health
```

Frontend commands are still future placeholders:

```bash
pnpm install
pnpm --filter web dev
```

## Development Principles

- Build vertical slices.
- Keep schemas versioned.
- Keep generated artifacts out of hand-written domain logic.
- Add tests before broad refactors.
- Keep BYOK security checks close to provider code.
