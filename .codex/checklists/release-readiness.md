# Release Readiness Checklist

- [ ] README explains current status accurately.
- [ ] `CHANGELOG.md` has a complete entry for the release version.
- [ ] `VERSION`, `Cargo.toml`, and `crates/core/src/version.rs` agree.
- [ ] `.env.example` is up to date.
- [ ] CI passes.
- [ ] No secrets, model files, databases, uploads, or exports are committed.
- [ ] ADRs exist for major decisions.
- [ ] BYOK security checklist is complete for any web release.
- [ ] Desktop mode has local data path documented.
- [ ] Web mode has database and object storage path documented.
- [ ] Import/export compatibility is documented.
