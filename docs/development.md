# Development Guide

The Rust backend foundation and the first SvelteKit workspace shell are scaffolded. Durable API wiring and AI workers are still in progress.

## Prerequisites

Expected tools:

- Rust stable.
- Node.js LTS.
- `pnpm` 10+.
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

Frontend commands now run from the repository root:

```bash
pnpm install
pnpm dev:web
pnpm check:web
pnpm lint:web
pnpm test:web
pnpm build:web
```

Windows full-stack launcher:

```powershell
scripts\dev-stack.bat
# hoặc
powershell -ExecutionPolicy Bypass -File scripts/dev-stack.ps1
```

Launcher behavior:

- Preferred ports: backend `3000`, frontend `5173`.
- If a preferred port is already used by this repo's own dev process, the launcher stops that process tree and reuses the preferred port.
- If a preferred port is used by a different process, the launcher searches for the next free port.
- Child backend/frontend processes are attached to a Windows job object so they are terminated when the launcher process exits.

Dry-run mode:

```powershell
pnpm dev:stack:dry-run
```

## Development Principles

- Build vertical slices.
- Keep schemas versioned.
- Keep generated artifacts out of hand-written domain logic.
- Add tests before broad refactors.
- Keep BYOK security checks close to provider code.
