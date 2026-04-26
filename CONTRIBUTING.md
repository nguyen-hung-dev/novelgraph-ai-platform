# Contributing

Thanks for your interest in NovelGraph AI Platform.

This repository is in the foundation stage. The most valuable contributions right now are architecture review, security review, data model review, and small vertical implementation slices.

## Development Direction

Read these files before proposing code:

- [README.md](README.md)
- [AGENTS.md](AGENTS.md)
- [.codex/project-context.md](.codex/project-context.md)
- [.codex/implementation-rules.md](.codex/implementation-rules.md)
- [docs/implementation-plan.md](docs/implementation-plan.md)

## Contribution Principles

- Keep the app workspace-oriented, not marketing-page-oriented.
- Preserve the hybrid web/desktop goal.
- Treat BYOK API keys as high-risk secrets.
- Prefer typed contracts over ad hoc JSON shapes.
- Add ADRs for major technical decisions.
- Keep pull requests small enough to review.

## Pull Request Checklist

- Explain the problem and the chosen approach.
- Link related docs or ADRs.
- Include validation steps.
- Update documentation when behavior or architecture changes.
- Do not include secrets, databases, uploaded novels, exports, model files, or generated caches.

## Branch Naming

Suggested prefixes:

- `feat/...`
- `fix/...`
- `docs/...`
- `refactor/...`
- `chore/...`

## Commit Style

Use Conventional Commits where practical:

```text
feat: add backend health endpoint
docs: document BYOK security model
chore: initialize Rust workspace
```

