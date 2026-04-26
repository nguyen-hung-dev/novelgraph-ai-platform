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

POST   /api/projects/{project_id}/novels/import/preview
POST   /api/projects/{project_id}/novels/import/confirm
GET    /api/projects/{project_id}/novels/{novel_id}
GET    /api/projects/{project_id}/novels/{novel_id}/chapters

POST   /api/projects/{project_id}/translation/jobs

GET    /api/projects/{project_id}/jobs/{job_id}/events
```

Health response shape:

```json
{
  "status": "ok",
  "app_mode": "web",
  "version": "0.1.1",
  "api_version": "v0",
  "storage_schema_version": "2026-04-26.foundation.v1"
}
```

Implemented import behavior:

- `preview` splits text into chapter previews without persistence.
- `confirm` stores the novel, chapters, paragraph-level source segments, and a pending analysis job.
- Translation job creation is persisted, but translation execution is not implemented yet.
- `jobs/{job_id}/events` returns persisted job events in sequence order. SSE streaming is still planned.

## Core REST Endpoints

```text
GET    /health

GET    /api/projects
POST   /api/projects
GET    /api/projects/{project_id}

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
