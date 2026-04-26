# Development Guide

The Rust backend foundation and the first live SvelteKit workspace wiring are scaffolded. Bookshelf, import preview/confirm, reading, and analysis screens now use real API data. The Analysis screen can now step through chapter-level local draft extraction with pause/resume controls. Durable background workers, realtime streaming, and review-item APIs are still in progress.

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
LLAMA_CPP_SERVER_BIN=llama-server
LLAMA_CPP_TIMEOUT_SECS=120
```

The Settings screen can now manage the local llama.cpp runtime:

- pick an existing `.gguf` file from the local machine and run it in place
- download supported preset models into the repo `models/` directory
- start or stop `llama-server` for the selected model

Runtime state is persisted at `data/local-llm-runtime.json`.

On Windows, if `LLAMA_CPP_SERVER_BIN` is not set, the backend now prefers the bundled repo binary:

```text
tools/llama.cpp/llama-server.exe
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
curl http://127.0.0.1:3000/api/local-llm/runtime
```

Analysis runner smoke test:

```bash
# after importing a novel, use the latest analysis job id from the workspace response
curl http://127.0.0.1:3000/api/projects/{project_id}/analysis/jobs/{job_id}/run
curl -X POST http://127.0.0.1:3000/api/projects/{project_id}/analysis/jobs/{job_id}/run/step \
  -H "content-type: application/json" \
  -d "{\"force\":false}"
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

Full-stack dev command:

```powershell
pnpm dev:stack
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
- The launcher exports `API_BASE_URL`, `PUBLIC_API_BASE_URL`, and `VITE_API_BASE_URL` for the frontend process.
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
