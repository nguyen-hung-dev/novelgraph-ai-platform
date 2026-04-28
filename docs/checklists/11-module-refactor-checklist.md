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

- [x] Ghi nhận line count hiện tại cho các file vượt soft/hard limit.
- [ ] Thêm command kiểm tra thủ công vào hướng dẫn phát triển.
- [ ] Đánh dấu các file hard-limit là legacy split targets, không nhận workflow mới.
- [ ] Chọn thứ tự chuyển đổi theo rủi ro: API boundary, storage, core extraction, frontend route lớn.
- [ ] Với mỗi vùng lớn, xác định smoke check trước khi split để biết hành vi còn giữ được.

Hotspot hiện tại:

- [ ] `crates/api/src/lib.rs` khoảng 895 dòng: đã dưới hard-limit, còn vượt soft-limit; tiếp tục tối giản router/app wiring.
- [ ] `crates/storage/src/sqlite.rs` khoảng 1020+ dòng: đã dưới hard-limit, còn vượt soft-limit; tiếp tục tách `project/novel/story`.
- [ ] `crates/core/src/extraction.rs` khoảng 1200+ dòng: hard-limit, ưu tiên số 3.
- [ ] `apps/web/src/routes/projects/[projectId]/reading/+page.svelte` khoảng 1100+ dòng: soft-limit gần hard-limit, ưu tiên số 4.

## Phase 1 - Tách API Boundary

- [ ] Tạo `crates/api/src/app.rs` cho app state, router tổng và layer setup.
- [x] Tạo `crates/api/src/errors.rs` cho `ApiError` và response mapping.
- [x] Tạo `crates/api/src/routes/health.rs` và chuyển health route.
- [x] Tạo `crates/api/src/routes/byok.rs` và chuyển BYOK handlers.
- [x] Tạo `crates/api/src/routes/local_runtime.rs` và chuyển local llama.cpp runtime routes.
- [x] Tạo `crates/api/src/routes/projects.rs` cho project CRUD và workspace read.
- [x] Tạo route module cho project realtime WebSocket.
- [x] Tạo `crates/api/src/routes/novels.rs` và `crates/api/src/services/novels.rs` cho import, metadata và chapters.
- [x] Tạo `crates/api/src/routes/translation.rs` cho translation job routes.
- [x] Tạo `crates/api/src/routes/jobs.rs` cho job event routes.
- [x] Tạo `crates/api/src/routes/analysis.rs` cho analysis jobs/run/step routes.
- [x] Tạo `crates/api/src/services/analysis.rs` cho analysis job get/run/reset/pause/cancel và snapshot read model.
- [x] Chuyển helper analysis chapter range/next chapter/finish range vào `services/analysis.rs`.
- [x] Tạo `crates/api/src/services/analysis_step.rs` cho orchestration preflight + stop/fail helpers: force reset, mark running, health gate, chọn chapter, pause on error.
- [x] Tạo `crates/api/src/services/analysis_pipeline.rs` và chuyển `run_next_analysis_chapter` pipeline sang service.
- [x] Tạo `crates/api/src/services/analysis_relationships.rs` và chuyển relationship pass entrypoint khỏi `lib.rs`.
- [x] Chuyển cụm helper relationship (`resolve/normalize/verify/persist record`) khỏi `lib.rs` sang `services/analysis_relationships.rs`.
- [x] Chuyển helper chia chunk character extraction sang `services/analysis_pipeline.rs`.
- [x] Tạo `crates/api/src/services/analysis_identity.rs` và chuyển cụm identity resolution xuyên chương khỏi `lib.rs`.
- [x] Tách `services/analysis_identity.rs` thành `analysis_identity.rs` (orchestration/matching) và `analysis_identity_review.rs` (review + scoring + LLM confirm).
- [x] Tạo `crates/api/src/services/analysis_alias.rs` và chuyển known-alias hints + alias ownership + quoted-alias context khỏi `lib.rs`.
- [x] Tạo `crates/api/src/services/analysis_mentions.rs` và chuyển backend mention scan + sampled LLM confirmation khỏi `lib.rs`.
- [x] Tạo `crates/api/src/services/analysis_fields.rs` và chuyển target contexts + field payload normalization/verification khỏi `lib.rs`.
- [x] Tạo `crates/api/src/services/analysis_document.rs` và chuyển record merge/dedupe, alias hydration, document validation, field-key normalization khỏi `lib.rs`.
- [x] Tạo `crates/api/src/services/llm_json.rs` cho gọi local LLM, parse JSON array và repair retry dùng chung.
- [ ] Chuyển logic dài khỏi handler sang `crates/api/src/services/*`.
- [ ] Giữ `lib.rs` dưới soft limit, chỉ export builder và module declarations.
- [ ] Smoke check: `/health`, `/api/projects`, `/api/byok/config`, local runtime health, analysis step một chương.

## Phase 2 - Tách Storage Repository

- [ ] Tạo `crates/storage/src/repositories/project.rs`.
- [ ] Tạo `crates/storage/src/repositories/novel.rs`.
- [x] Tạo `crates/storage/src/repositories/analysis.rs`.
- [x] Tạo `crates/storage/src/repositories/story_aliases.rs` cho rebuild alias map và lọc alias ổn định.
- [ ] Tạo `crates/storage/src/repositories/story.rs`.
- [x] Tạo `crates/storage/src/repositories/byok.rs`.
- [x] Tách smoke test storage khỏi `sqlite.rs` sang `crates/storage/src/sqlite_tests.rs`.
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
