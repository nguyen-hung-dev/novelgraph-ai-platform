# Testing Strategy

Testing should focus on correctness, privacy, and regression safety before UI polish.

## Backend Tests

- Config mode parsing.
- Database migrations.
- Repository CRUD.
- Typed API error responses.
- Job state transitions.
- Provider error normalization.
- BYOK redaction.

## Extraction Tests

- JSON schema validation.
- Evidence span validation.
- Quote exists in source text.
- No future chapter evidence.
- Review item generation for low-confidence facts.
- Retry and partial failure behavior.

## Frontend Tests

- Route shell renders.
- BYOK settings never display full saved key.
- Analysis progress updates from typed events.
- Import preview flow.
- Reading navigation.

## Regression Fixtures

Use small public-domain or synthetic texts. Do not commit copyrighted full novels.

Fixture goals:

- Chapter splitting.
- Entity extraction.
- Relationship extraction.
- Timeline projection.
- Review item behavior.

## CI Expectations

Early CI should check:

- Required docs exist.
- No obvious secrets are committed.
- Markdown links where practical.
- Rust tests after Rust workspace exists.
- Frontend checks after SvelteKit app exists.

