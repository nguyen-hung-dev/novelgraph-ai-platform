# Versioning Rules

## Current Version

- App version: `0.13.0`
- API version: `v0`
- Release channel: `foundation`
- Storage schema version: `2026-04-29.foundation.v9`

## Required Files

When preparing a planned release, a major milestone, or an explicit hotfix release, check these files:

- `CHANGELOG.md`
- `VERSION`
- `Cargo.toml`
- root `package.json`
- app package manifests such as `apps/web/package.json`
- `crates/core/src/version.rs`

When user-facing documentation mentions the current version, also check:

- `README.md`
- `README.vi.md`
- `docs/api-contract.md`
- `docs/data-model.md`
- `docs/development.md`

## Bump Policy

- Do not bump the app version for every small bug fix, UI polish pass, dev-only adjustment, test-only change, or documentation clarification.
- Batch small changes into `Unreleased` or the active milestone section until a planned release is prepared.
- Patch version: explicit hotfix releases or release-worthy maintenance batches after a public version already exists.
- Minor version before 1.0: new endpoints, new storage tables, new domain models, new worker capabilities, new UI slices, local runtime capabilities, or larger foundation milestones.
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

Do not create a new version section until release preparation, a major milestone, or an explicit hotfix. Keep changelog entries grouped by user-visible capability instead of listing every tiny implementation step.
