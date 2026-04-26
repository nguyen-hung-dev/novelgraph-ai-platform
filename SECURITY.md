# Security Policy

NovelGraph AI Platform is not production-ready yet. Security review is required before any hosted web release.

## Supported Versions

No stable release exists yet. Security reports should target the `main` branch until the first versioned release.

## Reporting a Vulnerability

Open a private security advisory on GitHub if available, or contact the maintainer privately before publishing details.

Do not include real API keys, private novels, user databases, or provider credentials in public issues.

## High-Risk Areas

- BYOK provider keys.
- Prompt traces and LLM request logs.
- Public project sharing.
- File upload and import.
- Object storage permissions.
- Desktop local data paths.
- Export/import formats.

## BYOK Rules

- API keys must never be stored in browser local storage.
- API keys must never be logged.
- Provider auth headers must not appear in traces or error responses.
- Persistent keys must be encrypted at rest.
- Session-only key mode should be implemented before persistent key storage.
- Public/shared projects must not spend an owner's key without explicit permission.

See [docs/security-byok.md](docs/security-byok.md).

