---
name: novelgraph-release
description: Prepare NovelGraph AI Platform releases in E:\product\dump-site\novelgraph-ai-platform. Use when the user explicitly asks to bump version, prepare release notes, commit release changes, push to GitHub, create or push a git tag, or prepare a GitHub release. Follow repo version metadata, changelog, verification, git safety, and explicit-confirmation rules.
---

# NovelGraph Release

Use this skill only for release work in `E:\product\dump-site\novelgraph-ai-platform`.

## Guardrails

- Treat `commit`, `push`, `tag`, and GitHub release creation as explicit actions. Do not perform them from vague wording such as "prepare" or "check".
- Before any commit, push, or tag, run `git status --short` and identify unrelated changes. Do not stage or revert files outside the requested release scope.
- If the worktree contains changes you did not make, either leave them unstaged or ask before including them.
- Never use `git reset --hard`, `git checkout --`, or destructive cleanup unless the user explicitly asks.
- Prefer non-interactive git commands.
- If network or remote git operations fail because of sandboxing, rerun the same required command with escalation and a clear justification.

## Preflight

1. Read `AGENTS.md`, `.codex/README.md`, `.codex/versioning.md`, and this skill.
2. Inspect the current state:
   - `git status --short`
   - `git branch --show-current`
   - `git remote -v`
   - `git tag --list "v*"`
3. Confirm the requested version and target branch from the user's wording.
4. If the version is ambiguous, ask one short question before editing version files.

## Version Metadata

When preparing a planned release, major milestone, or explicit hotfix, keep these files aligned:

- `VERSION`
- root `Cargo.toml` workspace package version
- root `package.json`
- app package manifests, especially `apps/web/package.json`
- `crates/core/src/version.rs`
- `README.md` and `README.vi.md` if they mention the current version

Also check, when relevant:

- `docs/api-contract.md`
- `docs/data-model.md`
- `docs/development.md`

Update `STORAGE_SCHEMA_VERSION` in `crates/core/src/version.rs` only when storage schema changes. Review `API_VERSION` for public REST contract changes, but keep `v0` while the API is foundation-stage unless the user explicitly decides otherwise.

## Changelog

- Add development changes to `## Unreleased` during normal work.
- During release preparation, move relevant entries into `## [x.y.z] - YYYY-MM-DD`.
- Preserve the repo format:

```text
## Unreleased

### Added
### Changed
### Fixed
### Security

## [0.1.0] - YYYY-MM-DD
```

- Group entries by user-visible capability. Do not list every tiny implementation detail.

## Verification

Choose lightweight checks unless the user requests a broad test run. Prefer checks directly tied to files touched by the release:

- Rust metadata/version edits: `cargo check -p novelgraph-core` or the smallest affected package check.
- Svelte/package metadata edits: package manager lockfile/version consistency checks only if manifests changed.
- Docs-only release note edits: inspect files and avoid broad tests.

Always report which checks were run and any skipped checks.

## Commit Workflow

1. Re-run `git status --short`.
2. Stage only release-scope files with explicit paths.
3. Inspect staged diff with `git diff --cached --stat` and, when useful, `git diff --cached -- <path>`.
4. Use a concise release commit message, for example:

```text
chore(release): prepare v0.11.0
```

5. After committing, run `git status --short`.

## Push And Tag Workflow

Only run these when the user explicitly asks to push and tag.

1. Confirm the active branch and remote.
2. Push the branch:

```text
git push origin <branch>
```

3. Create an annotated tag unless the user asks for a lightweight tag:

```text
git tag -a vX.Y.Z -m "Release vX.Y.Z"
```

4. Push the tag:

```text
git push origin vX.Y.Z
```

5. Verify:
   - `git status --short`
   - `git tag --list "vX.Y.Z"`
   - `git ls-remote --tags origin "refs/tags/vX.Y.Z"`

## Final Report

Report:

- Version prepared.
- Files changed.
- Commit hash, if committed.
- Branch and remote pushed, if pushed.
- Tag name, if created or pushed.
- Checks run.
- Any unrelated worktree changes left untouched.
