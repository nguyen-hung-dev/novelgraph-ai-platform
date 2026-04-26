# API Contract

This document sketches the intended API surface. It is not implemented yet.

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
    "message": "Human-readable safe message",
    "request_id": "req_..."
  }
}
```

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

GET    /api/projects/{project_id}/review-items
POST   /api/projects/{project_id}/review-items/{item_id}/decision

POST   /api/settings/byok/session
DELETE /api/settings/byok/session
```

## Realtime Events

```text
GET /api/projects/{project_id}/events
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

## Contract Generation

Open question:

- Generate OpenAPI from Rust types, or define OpenAPI/JSON Schema first and generate Rust/TypeScript from it.

Track the final decision in an ADR.

