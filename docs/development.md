# Development Guide

The project is not scaffolded as a runnable application yet. This guide defines the intended development shape.

## Prerequisites

Expected future tools:

- Rust stable.
- Node.js LTS.
- A package manager to be selected: `pnpm`, `npm`, or `bun`.
- PostgreSQL for hosted web development.
- SQLite for desktop/local development.
- Optional llama.cpp server for local AI testing.

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

These commands are placeholders until scaffolding exists:

```bash
# backend
cargo test
cargo run -p novelgraph-api

# frontend
pnpm install
pnpm --filter web dev
```

## Development Principles

- Build vertical slices.
- Keep schemas versioned.
- Keep generated artifacts out of hand-written domain logic.
- Add tests before broad refactors.
- Keep BYOK security checks close to provider code.

