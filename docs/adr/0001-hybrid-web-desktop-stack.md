# ADR 0001: Hybrid Web and Desktop Stack

Status: proposed

## Context

The project needs one desktop-style product experience that works in two deployment modes:

- Hosted web: user data can live on the website, and users bring their own LLM API key.
- Desktop/local: users can run offline with local storage and local AI.

The old project used Python FastAPI, React, SQLite/ChromaDB, Tauri, and Ollama. The rewrite should reduce desktop packaging complexity while supporting hosted multi-user infrastructure.

## Decision

Use:

- SvelteKit 2 + Svelte 5 + TypeScript for the shared UI.
- Tauri 2 for desktop shell.
- Rust + Axum + Tokio for backend services.
- SQLx with SQLite for desktop and PostgreSQL for hosted web.
- BYOK cloud provider proxy for web inference.
- llama.cpp `llama-server` for local desktop inference.

## Consequences

Benefits:

- One UI can serve web and desktop.
- Rust backend reduces Python sidecar packaging overhead.
- PostgreSQL gives web deployment proper multi-user storage.
- SQLite keeps desktop offline mode simple.
- BYOK avoids platform-funded LLM cost in the first hosted version.
- llama.cpp gives tighter local model and structured output control.

Tradeoffs:

- Rust implementation cost is higher than Python for rapid prompt tooling.
- Two database backends require disciplined storage abstractions.
- BYOK raises security requirements immediately.
- Visualization renderers should be framework-independent to avoid future UI lock-in.

