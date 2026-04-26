# Deployment Model

NovelGraph AI Platform is planned for three runtime modes.

## Hosted Web

```text
Browser
  -> SvelteKit app
  -> Rust API
  -> PostgreSQL
  -> Object storage
  -> BYOK provider proxy
```

Expected infrastructure:

- PostgreSQL.
- S3/R2/MinIO-compatible object storage.
- HTTPS.
- Background job workers.
- Secret encryption key management.
- Log redaction.

## Desktop

```text
Tauri shell
  -> shared SvelteKit UI
  -> local Rust backend/core
  -> SQLite
  -> local files
  -> optional llama.cpp sidecar
```

Expected local storage:

- App data directory.
- SQLite database.
- Uploaded source files or normalized text.
- Visual cache.
- Optional local model files outside Git.

## Demo

```text
Static site
  -> precomputed demo datasets
```

Demo mode must not require user secrets or private data.

## Deployment Risks

- BYOK key leakage through logs or traces.
- Object storage permissions too broad.
- Public sharing leaking private source text.
- Long-running jobs tied to short HTTP request lifetimes.
- Desktop model bundles too large for initial releases.

