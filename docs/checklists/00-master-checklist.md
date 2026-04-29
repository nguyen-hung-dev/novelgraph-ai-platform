# Checklist Tổng Thể

Checklist này theo dõi tiến độ cấp cao cho toàn bộ dự án.

## Tầng 0 - Định Hướng Và Repo

- [x] Chốt license.
- [x] Chốt package manager cho frontend.
- [x] Chốt Rust workspace layout.
- [ ] Chốt kiến trúc module phân cấp theo `docs/module-architecture.md`.
- [ ] Áp dụng soft limit 800 dòng và hard limit 1200 dòng cho file viết tay.
- [ ] Hoàn thành phase baseline trong `docs/checklists/11-module-refactor-checklist.md`.
- [ ] Chốt chiến lược API contract: OpenAPI-first hoặc Rust-schema-first.
- [ ] Kiểm tra README tiếng Anh và tiếng Việt trước khi public rộng.
- [ ] Đảm bảo không commit `docs/architecture-and-rewrite.md`.

## Tầng 1 - Nền Tảng Kỹ Thuật

- [x] Scaffold Rust workspace.
  - [x] Tạo crate `core`.
  - [x] Tạo crate `storage`.
  - [x] Tạo crate `ai`.
  - [x] Tạo crate `jobs`.
  - [x] Tạo crate `api`.
- [x] Scaffold SvelteKit app.
  - [x] Tạo `apps/web`.
  - [x] Chọn adapter cho web.
  - [ ] Chuẩn bị build static cho desktop.
- [ ] Chuẩn bị CI thật.
  - [x] Rust check/test.
  - [ ] Frontend typecheck/lint.
  - [ ] Secret scan.
- [ ] Chốt copy catalog/i18n registry để không hardcode chuỗi UI.
- [ ] Chốt prompt registry có version để không hardcode prompt trong code.

## Tầng 2 - Web Hosted

- [ ] Thiết kế auth boundary.
- [ ] Thiết kế workspace/project ownership.
- [ ] Thiết kế PostgreSQL schema.
- [ ] Thiết kế object storage.
- [ ] Thiết kế BYOK provider proxy.
- [ ] Thiết kế quota/rate limit.

## Tầng 3 - Desktop Local

- [ ] Thiết kế Tauri shell.
- [ ] Thiết kế SQLite local storage.
- [ ] Thiết kế local data directory.
- [ ] Thiết kế llama.cpp sidecar.
- [ ] Thiết kế import/export offline.

## Tầng 4 - AI Pipeline

- [ ] Chốt extraction schema đầu tiên.
- [ ] Chốt translation schema đầu tiên.
- [ ] Chốt evidence span schema.
- [ ] Chốt observation model.
- [ ] Chốt glossary model.
- [ ] Chốt review item model.
- [ ] Chốt agentic pipeline contract để analysis/translation chạy không cần người dùng duyệt từng bước.
- [ ] Chốt stale marker model cho dữ liệu phụ thuộc sau khi user sửa trực tiếp.
- [x] Chốt job event model.
- [ ] Chốt regression fixture strategy.
- [ ] Chốt nhánh Gemini cloud one-shot theo `docs/checklists/12-llm-cloud-gemini-checklist.md`.
- [ ] Chốt world model bootstrap và dynamic taxonomy theo `docs/checklists/13-world-model-bootstrap-checklist.md`.

## Tầng 5 - MVP UI

- [x] Project/bookshelf.
- [x] Upload/import preview.
- [x] Reading view.
- [x] Analysis progress.
- [x] BYOK settings.
- [x] Review route placeholder.
- [ ] Inline edit raw chapter text từ reading UI và ghi DB ngay.
- [ ] Inline edit entity, alias, relationship, glossary và translation segment.
- [ ] Reading song ngữ source/target.

## Tầng 6 - Release

- [ ] Hoàn thành checklist BYOK security.
- [ ] Hoàn thành checklist release readiness.
- [ ] Kiểm tra docs không claim tính năng chưa có.
- [ ] Tag release đầu tiên khi có app chạy được.
