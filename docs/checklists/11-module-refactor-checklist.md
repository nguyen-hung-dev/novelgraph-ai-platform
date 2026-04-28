# Checklist Chuyển Đổi Kiến Trúc Module

Checklist này dùng để chuyển code hiện tại sang cấu trúc module phân cấp rõ ràng theo `docs/module-architecture.md`. Mục tiêu là giảm file khổng lồ, làm rõ ownership, và giúp AI coding agent có thể kiểm soát từng vùng nhỏ của codebase.

## Nguyên Tắc Theo Dõi

- [ ] Áp dụng soft limit 800 dòng và hard limit 1200 dòng cho file viết tay.
- [ ] Không thêm feature mới vào file đã vượt hard limit trước khi tách module.
- [ ] Mỗi bước refactor phải giữ hành vi cũ, có kiểm tra nhẹ phù hợp, và không trộn với feature không liên quan.
- [ ] Mỗi module mới phải có tên domain rõ ràng và boundary rõ trách nhiệm.
- [ ] Khi tách file, cập nhật import/export theo hướng module sở hữu, không tạo barrel file mơ hồ nếu làm mất trace.
- [ ] Nếu thay đổi public API, schema, migration, hoặc hành vi người dùng thấy được, cập nhật `CHANGELOG.md`.

## Phase 0 - Baseline Và Luật Chặn File Lớn

- [ ] Ghi nhận line count hiện tại cho các file vượt soft/hard limit.
- [ ] Thêm command kiểm tra thủ công vào hướng dẫn phát triển.
- [ ] Đánh dấu các file hard-limit là legacy split targets, không nhận workflow mới.
- [ ] Chọn thứ tự chuyển đổi theo rủi ro: API boundary, storage, core extraction, frontend route lớn.
- [ ] Với mỗi vùng lớn, xác định smoke check trước khi split để biết hành vi còn giữ được.

Hotspot hiện tại:

- [ ] `crates/api/src/lib.rs` khoảng 6500+ dòng: hard-limit, ưu tiên số 1.
- [ ] `crates/storage/src/sqlite.rs` khoảng 2300+ dòng: hard-limit, ưu tiên số 2.
- [ ] `crates/core/src/extraction.rs` khoảng 1200+ dòng: hard-limit, ưu tiên số 3.
- [ ] `apps/web/src/routes/projects/[projectId]/reading/+page.svelte` khoảng 1100+ dòng: soft-limit gần hard-limit, ưu tiên số 4.

## Phase 1 - Tách API Boundary

- [ ] Tạo `crates/api/src/app.rs` cho app state, router tổng và layer setup.
- [ ] Tạo `crates/api/src/errors.rs` cho `ApiError` và response mapping.
- [ ] Tạo `crates/api/src/routes/health.rs` và chuyển health route.
- [ ] Tạo `crates/api/src/routes/byok.rs` và chuyển BYOK handlers.
- [ ] Tạo `crates/api/src/routes/local_runtime.rs` và chuyển local llama.cpp runtime routes.
- [ ] Tạo `crates/api/src/routes/projects.rs` cho project CRUD và workspace read.
- [ ] Tạo `crates/api/src/routes/analysis.rs` cho analysis jobs/run/step routes.
- [ ] Chuyển logic dài khỏi handler sang `crates/api/src/services/*`.
- [ ] Giữ `lib.rs` dưới soft limit, chỉ export builder và module declarations.
- [ ] Smoke check: `/health`, `/api/projects`, `/api/byok/config`, local runtime health, analysis step một chương.

## Phase 2 - Tách Storage Repository

- [ ] Tạo `crates/storage/src/repositories/project.rs`.
- [ ] Tạo `crates/storage/src/repositories/novel.rs`.
- [ ] Tạo `crates/storage/src/repositories/analysis.rs`.
- [ ] Tạo `crates/storage/src/repositories/story.rs`.
- [ ] Tạo `crates/storage/src/repositories/byok.rs`.
- [ ] Tạo `crates/storage/src/repositories/local_runtime.rs` nếu storage state cần tách riêng.
- [ ] Tạo `crates/storage/src/mappers/*` cho row mapper dài.
- [ ] Giữ SQLite/Postgres parity trong từng repository hoặc adapter.
- [ ] Không đổi schema khi chỉ split repository; nếu cần schema mới thì làm migration riêng.
- [ ] Smoke check: import novel, list workspace, save BYOK config, run analysis step, đọc Reading page.

## Phase 3 - Tách Core Domain Và Extraction

- [ ] Chuyển `domain.rs` thành `domain/mod.rs` với file theo domain: `project`, `novel`, `analysis`, `story`, `byok`, `translation`, `runtime`.
- [ ] Tách `extraction.rs` thành schema, prompt contract, validation, post-processing và evidence helpers.
- [ ] Đưa constant schema version vào module gần schema sở hữu.
- [ ] Tách parser/repair helper thuần khỏi orchestration gọi LLM.
- [ ] Bảo đảm `crates/core` không import HTTP, SQL, filesystem hoặc provider client.
- [ ] Smoke check: build core, chạy extraction unit/smoke nếu có, chạy API check.

## Phase 4 - Tách Frontend Route Lớn

- [ ] Tạo `apps/web/src/lib/features/reading/`.
- [ ] Tách chapter list khỏi `reading/+page.svelte`.
- [ ] Tách reader viewport và highlight rendering.
- [ ] Tách character detail panel và relationship list.
- [ ] Tách reading action state/presenter khỏi markup route.
- [ ] Tạo `apps/web/src/lib/features/settings/byok/` cho BYOK panel nếu Settings tiếp tục tăng.
- [ ] Tách `apps/web/src/lib/server/api.ts` thành domain clients khi file chạm soft limit.
- [ ] Smoke check: mở `/settings`, mở Reading page, chọn chapter, highlight entity, refresh workspace.

## Phase 5 - Tách AI Provider Và Pipeline

- [ ] Chuyển provider-specific logic sang `crates/ai/src/providers/*`.
- [ ] Tách OpenAI-compatible request/response mapping khỏi API service.
- [ ] Tách Gemini preset/capability vào provider registry.
- [ ] Tách JSON repair, redaction, retry policy và token budget khỏi route/service lớn.
- [ ] Đưa prompt text vào prompt registry có version.
- [ ] Smoke check: local llama.cpp health, BYOK health check, một structured-output call mẫu.

## Phase 6 - Hoàn Thiện Governance

- [ ] Cập nhật `docs/module-architecture.md` nếu boundary thực tế thay đổi.
- [ ] Cập nhật `AGENTS.md` và `.codex/implementation-rules.md` nếu có rule mới.
- [ ] Thêm ADR nếu quyết định module ảnh hưởng lâu dài đến public API, storage hoặc deployment.
- [ ] Đảm bảo các file đã split không vượt lại soft limit ngay sau chuyển đổi.
- [ ] Đóng checklist phase khi smoke checks đã chạy và kết quả được ghi trong PR hoặc ghi chú triển khai.

## Definition Of Done

- [ ] Không còn file viết tay vượt hard limit 1200 dòng trừ ngoại lệ được ghi rõ.
- [ ] Các file vượt soft limit 800 dòng đều có kế hoạch split hoặc lý do ngoại lệ.
- [ ] `crates/api/src/lib.rs` chỉ còn app builder, route wiring và exports tối thiểu.
- [ ] `crates/storage/src/sqlite.rs` không còn là repository tổng hợp cho mọi domain.
- [ ] Route Svelte lớn được tách thành feature components có ownership rõ.
- [ ] AI agent có thể tìm đúng module bằng `docs/module-architecture.md`, `AGENTS.md` và checklist này.
