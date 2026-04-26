# Versioning Rules

## Current Version

- App version: `0.5.0`
- API version: `v0`
- Release channel: `foundation`
- Storage schema version: `2026-04-26.foundation.v2`

## Required Files

When code changes, always check these files:

- `CHANGELOG.md`
- `VERSION`
- `Cargo.toml`
- `crates/core/src/version.rs`

When user-facing documentation mentions the current version, also check:

- `README.md`
- `README.vi.md`
- `docs/api-contract.md`
- `docs/data-model.md`
- `docs/development.md`

## Bump Policy

- Patch version: bug fixes, internal refactors, test additions, and documentation that clarifies shipped behavior.
- Minor version before 1.0: new endpoints, new storage tables, new domain models, new worker capabilities, new UI slices, or larger foundation milestones.
- Major version: reserved for the future stable `1.0.0` line.

Storage schema changes must update `STORAGE_SCHEMA_VERSION` in `crates/core/src/version.rs`.

Public REST contract changes must review `API_VERSION`; keep `v0` while the API is still foundation-stage and unstable.

## Changelog Format

Use this structure:

```text
## Unreleased

### Added
### Changed
### Fixed
### Security

## [0.1.0] - YYYY-MM-DD
```

Do not leave meaningful code changes undocumented.
