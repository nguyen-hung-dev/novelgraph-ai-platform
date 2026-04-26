# API Contract

This document sketches the intended API surface. Some foundation endpoints are now implemented in the Rust API crate.

## Principles

- Typed request and response schemas.
- Stable error envelope.
- WebSocket/SSE events for long-running jobs.
- No raw provider keys in frontend-visible state after setup.
- Generated client types for frontend.

## Error Envelope

```json
{
  "error": {
    "code": "invalid_request",
    "message": "Human-readable safe message"
  }
}
```

Request IDs are still planned, but not implemented yet.

## Implemented Foundation Endpoints

```text
GET    /health

GET    /api/projects
POST   /api/projects
GET    /api/projects/{project_id}
POST   /api/projects/{project_id}
GET    /api/projects/{project_id}/workspace

POST   /api/projects/{project_id}/novels/import/preview
POST   /api/projects/{project_id}/novels/import/confirm
GET    /api/projects/{project_id}/novels/{novel_id}
GET    /api/projects/{project_id}/novels/{novel_id}/chapters

POST   /api/projects/{project_id}/translation/jobs
GET    /api/projects/{project_id}/translation/jobs/{job_id}
POST   /api/projects/{project_id}/translation/jobs/{job_id}/cancel

GET    /api/projects/{project_id}/analysis/jobs/{job_id}
POST   /api/projects/{project_id}/analysis/jobs/{job_id}/cancel

GET    /api/projects/{project_id}/jobs/{job_id}/events

GET    /api/local-llm/health
GET    /api/local-llm/models
GET    /api/local-llm/runtime
POST   /api/local-llm/runtime/select-existing
POST   /api/local-llm/runtime/start-selected
POST   /api/local-llm/runtime/stop
POST   /api/local-llm/runtime/models/activate
POST   /api/local-llm/runtime/presets/{preset_id}/download
POST   /api/local-llm/chat/completions
POST   /api/local-llm/extraction/draft-chapter
```

Health response shape:

```json
{
  "status": "ok",
  "app_mode": "web",
  "version": "0.8.0",
  "api_version": "v0",
  "storage_schema_version": "2026-04-27.foundation.v3"
}
```

Implemented import behavior:

- `preview` splits text into chapter previews without persistence.
- `confirm` stores the novel, chapters, paragraph-level source segments, and a pending analysis job.
- `POST /api/projects/{project_id}` deletes a project. With `purge_data: false`, the project is archived and hidden from the bookshelf while its DB rows are retained. With `purge_data: true`, all project data is deleted by cascade.
- `workspace` returns the project record, available novels, active novel, chapter list, latest analysis job, and latest job events in one round-trip for the UI shell.
- Translation job creation is persisted, but translation execution is not implemented yet.
- Analysis and translation jobs can be read and cancelled.
- Cancelling a terminal job returns `409 invalid_job_transition`.
- `jobs/{job_id}/events` returns persisted job events in sequence order. SSE streaming is still planned.
- Local llama.cpp endpoints use the OpenAI-compatible `/v1` server surface and do not require browser-provided API keys.
- `GET /api/local-llm/runtime` returns local runtime state, selected model, repo-managed models, preset catalog, and current download status.
- `POST /api/local-llm/runtime/select-existing` opens a native file dialog on the local machine and starts the selected GGUF file in place without copying it into the repo.
- `POST /api/local-llm/runtime/presets/{preset_id}/download` downloads a small preset GGUF into the repo `models/` directory and activates it when the download completes.
- `POST /api/local-llm/runtime/models/activate` activates an already-downloaded GGUF inside the repo `models/` directory.
- `POST /api/local-llm/runtime/start-selected` and `POST /api/local-llm/runtime/stop` control the managed `llama-server` process.
- Draft chapter extraction calls local llama.cpp and returns prompt metadata plus the raw chat completion response. It does not persist observations.

## Core REST Endpoints

```text
GET    /health

GET    /api/projects
POST   /api/projects
GET    /api/projects/{project_id}
POST   /api/projects/{project_id}
GET    /api/projects/{project_id}/workspace

POST   /api/projects/{project_id}/novels/import/preview
POST   /api/projects/{project_id}/novels/import/confirm
GET    /api/projects/{project_id}/novels/{novel_id}
GET    /api/projects/{project_id}/novels/{novel_id}/chapters
GET    /api/projects/{project_id}/novels/{novel_id}/chapters/{chapter_num}

POST   /api/projects/{project_id}/analysis/jobs
GET    /api/projects/{project_id}/analysis/jobs/{job_id}
POST   /api/projects/{project_id}/analysis/jobs/{job_id}/cancel

POST   /api/projects/{project_id}/translation/jobs
GET    /api/projects/{project_id}/translation/jobs/{job_id}
POST   /api/projects/{project_id}/translation/jobs/{job_id}/cancel
GET    /api/projects/{project_id}/novels/{novel_id}/translations
GET    /api/projects/{project_id}/novels/{novel_id}/chapters/{chapter_num}/translation

GET    /api/projects/{project_id}/glossary
POST   /api/projects/{project_id}/glossary
PATCH  /api/projects/{project_id}/glossary/{entry_id}

GET    /api/projects/{project_id}/review-items
POST   /api/projects/{project_id}/review-items/{item_id}/decision

POST   /api/settings/byok/session
DELETE /api/settings/byok/session

GET    /api/projects/{project_id}/jobs/{job_id}/events

GET    /api/local-llm/health
GET    /api/local-llm/models
GET    /api/local-llm/runtime
POST   /api/local-llm/runtime/select-existing
POST   /api/local-llm/runtime/start-selected
POST   /api/local-llm/runtime/stop
POST   /api/local-llm/runtime/models/activate
POST   /api/local-llm/runtime/presets/{preset_id}/download
POST   /api/local-llm/chat/completions
POST   /api/local-llm/extraction/draft-chapter
```

## Realtime Events

```text
GET /api/projects/{project_id}/events
```

Implemented foundation event history:

```text
GET /api/projects/{project_id}/jobs/{job_id}/events
```

Event shape:

```json
{
  "type": "analysis_progress",
  "project_id": "proj_...",
  "job_id": "job_...",
  "sequence": 42,
  "payload": {}
}
```

Translation event:

```json
{
  "type": "translation_progress",
  "project_id": "proj_...",
  "job_id": "job_...",
  "sequence": 43,
  "payload": {
    "chapter_num": 12,
    "segment_done": 8,
    "segment_total": 24
  }
}
```

## Contract Generation

Open question:

- Generate OpenAPI from Rust types, or define OpenAPI/JSON Schema first and generate Rust/TypeScript from it.

Track the final decision in an ADR.
