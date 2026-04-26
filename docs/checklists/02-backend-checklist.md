# Checklist Phase 2 - Backend Rust/Axum

## Workspace

- [x] Tạo Rust workspace root.
- [x] Tạo crate `crates/core`.
  - [x] Domain config type ban đầu.
  - [x] Shared validation.
  - [x] Error types ban đầu.
- [x] Tạo crate `crates/storage`.
  - [x] SQLx setup.
  - [x] SQLite migration folder.
  - [x] PostgreSQL migration folder.
- [x] Tạo crate `crates/api`.
  - [x] Axum router.
  - [x] Health endpoint.
  - [x] Error envelope.
- [ ] Tạo crate `crates/jobs`.
  - [ ] Job state machine.
  - [ ] Event schema.
- [ ] Tạo crate `crates/ai`.
  - [ ] Provider trait.
  - [ ] Provider errors.

## Config

- [x] Tạo config loader.
  - [x] `APP_MODE=web`.
  - [x] `APP_MODE=desktop`.
  - [x] `APP_MODE=demo`.
- [ ] Tách cấu hình database.
- [ ] Tách cấu hình object storage.
- [ ] Tách cấu hình AI provider.
- [ ] Không log secret trong config dump.

## Database

- [x] Tạo migration đầu tiên.
  - [x] `users`.
  - [x] `workspaces`.
  - [x] `workspace_members`.
  - [x] `projects`.
  - [x] `novels`.
  - [x] `chapters`.
  - [x] `source_segments`.
  - [x] `analysis_jobs`.
  - [x] `job_events`.
  - [x] `translation_jobs`.
  - [x] `translation_segments`.
  - [x] `glossary_entries`.
  - [x] `style_profiles`.
  - [x] `translation_review_items`.
  - [x] `llm_provider_configs`.
  - [x] `llm_usage_events`.
- [x] Chạy migration trên SQLite.
- [ ] Chạy migration trên PostgreSQL.
- [x] Viết repository tests.

## API Nền Tảng

- [x] `GET /health`.
- [x] `GET /api/projects`.
- [x] `POST /api/projects`.
- [x] `GET /api/projects/{project_id}`.
- [x] `POST /api/projects/{project_id}/novels/import/preview`.
- [x] `POST /api/projects/{project_id}/novels/import/confirm`.
- [x] `GET /api/projects/{project_id}/novels/{novel_id}`.
- [x] `GET /api/projects/{project_id}/novels/{novel_id}/chapters`.
- [x] `POST /api/projects/{project_id}/translation/jobs`.
- [x] Event endpoint cho job progress.
- [ ] Request id middleware.
- [ ] CORS policy cho web.

## Kiểm Thử

- [x] Unit test config.
- [ ] Unit test error envelope.
- [x] Integration test migration.
- [ ] Integration test health endpoint.
- [x] Integration test project CRUD.
- [x] Unit test chapter splitter.
