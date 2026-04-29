# Checklist Nhánh LLM Cloud Gemini

Ngày lập kế hoạch: 2026-04-29
Nhánh đề xuất: `feature/llm-cloud-gemini`
Mục tiêu chính: chạy phân tích chương bằng Gemini cloud với số API call thấp nhất có thể, nhưng vẫn giữ chất lượng dữ liệu đủ cao để ghi vào graph/evidence store.

## Mục Tiêu Định Lượng

- [ ] Mặc định mỗi chương chỉ dùng 1 API call Gemini cho extraction chính.
- [ ] Chỉ dùng call thứ 2 khi output lỗi schema, thiếu trường bắt buộc, evidence không map được, hoặc validator phát hiện rủi ro cao.
- [ ] Chỉ dùng call thứ 3 cho targeted verifier rất hẹp, không chạy lại toàn chương.
- [x] Ghi `api_call_count`, `provider`, `model`, `input_tokens`, `output_tokens`, `estimated_cost`, `profile`, `schema_version`, và `trace_id` cho mỗi chương.
- [x] Không để cloud profile phá đường local llama.cpp hiện có.
- [ ] Mục tiêu chất lượng tối thiểu cho fixture đầu tiên:
  - [ ] JSON/schema parse success >= 99%.
  - [ ] Evidence quote match với chapter text >= 98%.
  - [ ] Không tái diễn lỗi alias owner nghiêm trọng đã biết như nhập `Mặc lão`/`Mặc phu` vào Hàn Lập.
  - [x] Relationship persist chỉ nhận `kinship`, `organization_hierarchy`, hoặc `stable_relationship`.
  - [x] Field appearance không nhận trạng thái/hành động/biểu cảm tạm thời.

## Nguyên Tắc Thiết Kế

- [x] Tạo profile mới `cloud_gemini_one_shot`, không sửa profile local thành cloud.
- [x] Dùng structured output/JSON schema native của Gemini thay vì chỉ nhét schema vào prompt.
- [x] Model trả evidence quote, không bắt model trả `start_char`/`end_char` làm nguồn sự thật.
- [x] Backend tự map quote về offset, scan mention, dedupe, normalize key, và quyết định persist.
- [ ] Mọi dữ liệu mơ hồ đi vào `review_items` hoặc bị bỏ, không tự persist vì confidence cao.
- [ ] Không chạy verifier đại trà. Verifier chỉ chạy theo danh sách suspect do validator tạo.
- [ ] Tối ưu call count trước, nhưng không hy sinh evidence-first và write gate.
- [x] Không đưa API key, auth header, hoặc raw provider error chứa secret vào prompt trace/log.

## Nguồn Tham Khảo Kỹ Thuật

- Gemini 2.5 Pro model docs: `https://ai.google.dev/gemini-api/docs/models/gemini-2.5-pro`
- Gemini 2.5 Flash model docs: `https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash`
- Gemini structured outputs: `https://ai.google.dev/gemini-api/docs/structured-output`
- Repo module target: `docs/module-architecture.md`
- BYOK checklist: `docs/checklists/03-byok-ai-checklist.md`
- Major extraction checklist: `docs/checklists/10-major-group-extraction-checklist.md`
- Semantic audit baseline: `docs/semantic-extraction-audit-2026-04-28.txt`

## Phase 0 - Chốt Phạm Vi Nhánh

- [ ] Tạo nhánh riêng từ branch chính hiện tại.
- [ ] Ghi rõ nhánh này là spike/feature branch cho Gemini cloud, chưa thay thế local llama.cpp.
- [ ] Chọn model mặc định ban đầu:
  - [ ] `gemini-2.5-flash` cho price/performance và batch lớn.
  - [ ] `gemini-2.5-pro` cho benchmark chất lượng hoặc chương khó.
- [ ] Chọn API path chính:
  - [ ] Ưu tiên direct Gemini API nếu cần structured output JSON schema đầy đủ.
  - [ ] Giữ OpenAI-compatible path cho provider generic nếu không cần schema native.
- [ ] Chốt call budget:
  - [ ] `normal_max_calls_per_chapter = 1`.
  - [ ] `repair_max_calls_per_chapter = 1`.
  - [ ] `targeted_verifier_max_calls_per_chapter = 1`.
- [ ] Chốt không-goals:
  - [ ] Không xóa local llama.cpp runtime.
  - [ ] Không chuyển toàn bộ app sang Gemini-only.
  - [ ] Không triển khai billing public trong nhánh này.

Acceptance criteria:

- [ ] Có branch riêng và mô tả phạm vi trong PR/commit đầu.
- [x] Có config flag hoặc profile name để bật/tắt cloud Gemini mà không ảnh hưởng local.

## Phase 1 - Provider Gemini Và BYOK Proxy

- [x] Tách provider abstraction trong `crates/ai` theo hướng `providers/gemini.rs`, `providers/llama_cpp.rs`, `providers/openai_compatible.rs`.
- [x] Thêm contract provider chung:
  - [x] `generate_chat`.
  - [x] `generate_structured`.
  - [x] `validate_key`.
  - [x] `estimate_cost`.
  - [x] `model_capabilities`.
- [x] Tạo `GeminiProvider` hỗ trợ:
  - [x] Base URL/API version config.
  - [x] API key từ BYOK backend, không từ frontend.
  - [x] Structured output JSON schema.
  - [x] Timeout riêng cho long chapter.
  - [x] Token usage parsing.
  - [x] Error mapping user-safe.
- [x] Bổ sung provider capability:
  - [x] `supports_structured_output`.
  - [x] `max_input_tokens`.
  - [x] `max_output_tokens`.
  - [x] `supports_context_caching`.
  - [x] `supports_thinking_config`.
- [x] Thêm redaction tests cho provider errors và headers.

Acceptance criteria:

- [ ] Health/key check Gemini chạy qua backend.
- [ ] Một smoke test structured JSON đơn giản trả object parse được.
- [ ] Không log raw API key hoặc authorization header.

## Phase 2 - Structured Schema Một Call Cho Một Chương

- [x] Tạo schema `story_chapter_cloud_extraction.v2`.
- [x] Root output nên là object, không phải array trần:

```json
{
  "schema_version": "story_chapter_cloud_extraction.v2",
  "chapter_num": 1,
  "characters": [],
  "relationships": [],
  "review_items": [],
  "call_profile": "cloud_gemini_one_shot"
}
```

- [ ] Mỗi character cần có:
  - [ ] `display_name`.
  - [ ] `aliases`.
  - [ ] `entity_nature`: `individual_character`, `group`, `role_title`, `temporary_reference`, `uncertain`.
  - [x] `fields` với `semantic_class`.
  - [ ] `evidence_quotes`.
  - [ ] `confidence`.
- [ ] Mỗi relationship cần có:
  - [ ] `source_name`.
  - [ ] `target_name`.
  - [x] `relationship_scope`.
  - [ ] `relationship_type`.
  - [ ] `source_to_target_label`.
  - [ ] `target_to_source_label`.
  - [ ] `evidence_quotes`.
  - [ ] `confidence`.
- [ ] Schema bắt buộc model phân biệt:
  - [x] Alias thật với role/generic phrase.
  - [x] Appearance ổn định với action/state/emotion tạm thời.
  - [x] Stable relationship với temporary interaction/shared event/co-presence.
- [x] Không yêu cầu `start_char`/`end_char` trong schema chính.
- [ ] Thêm `review_items` cho mọi mục model không chắc.

Acceptance criteria:

- [ ] Prompt + schema đủ để Gemini trả toàn bộ extraction trong 1 call với một chương fixture.
- [ ] Backend parse được object root và reject schema version sai.

## Phase 3 - Context Builder Tối Ưu Token Và Chất Lượng

- [x] Tạo context builder riêng cho cloud profile.
- [ ] Input vào call chính gồm:
  - [x] Novel metadata ngắn.
  - [x] Chapter text đầy đủ nếu nằm trong budget.
  - [x] Known characters/aliases đã persist nhưng chỉ những surface xuất hiện hoặc gần xuất hiện trong chương hiện tại.
  - [ ] Relationship/alias warnings từ các chương trước nếu có liên quan.
- [ ] Không nhồi toàn bộ DB character history vào prompt.
- [x] Dùng deterministic pre-scan để tìm known alias surfaces trong chapter trước khi gọi Gemini.
- [ ] Nếu chapter quá dài hoặc output risk quá lớn:
  - [ ] Chia theo section lớn, không chia 2,400 chars như local profile.
  - [ ] Gắn `split_reason = input_too_large` hoặc `output_too_large`.
  - [ ] Ghi call budget bị vượt vì lý do kỹ thuật.
- [ ] Chuẩn bị chỗ cho context caching sau, nhưng không phụ thuộc cache trong MVP.

Acceptance criteria:

- [ ] Context builder tạo prompt ổn định và log được token estimate.
- [ ] Cùng một chapter + same context tạo cùng prompt hash.

## Phase 4 - Cloud Pipeline Profile

- [x] Thêm `AnalysisExecutionProfile`:
  - [x] `local_small_staged`.
  - [x] `cloud_gemini_one_shot`.
- [x] Không gọi `call_local_json_array` trong cloud profile.
- [x] Thay bằng `call_structured_chapter_extraction`.
- [x] Runner cloud một chương:
  - [x] Prepare context.
  - [x] Call Gemini structured output một lần.
  - [x] Validate schema.
  - [x] Normalize identities, fields, aliases, relationships.
  - [x] Map evidence quotes về chapter offsets.
  - [x] Backend scan mentions từ display name + aliases đã accepted.
  - [x] Persist story extraction records.
  - [x] Emit job events.
- [ ] Nếu validator tạo suspect list:
  - [ ] Chỉ gọi targeted verifier khi suspect list vượt threshold.
  - [ ] Không gọi verifier cho mọi field/relationship như local profile.

Acceptance criteria:

- [ ] Một chương bình thường hoàn tất với exactly 1 Gemini call.
- [ ] Output persisted vào cùng storage shape để UI reading/analysis hiện được.
- [ ] Local profile vẫn chạy theo đường cũ.

## Phase 5 - Validator Và Targeted Verifier

- [ ] Tạo validator thuần trong `crates/core` hoặc service nhỏ không phụ thuộc provider.
- [ ] Validator phải tạo `suspect_items` thay vì tự gọi LLM.
- [ ] Suspect reasons:
  - [ ] Evidence quote không match chapter text.
  - [ ] Alias owner không resolve được.
  - [ ] Alias surface giống tên character khác.
  - [ ] Field value thuộc semantic class không persist được.
  - [ ] Relationship scope không persist được.
  - [ ] Relationship source/target không resolve unique.
  - [ ] Output quá nhiều item bất thường so với chapter length.
- [ ] Targeted verifier call nhận danh sách suspect ngắn, không nhận lại toàn bộ chương nếu không cần.
- [ ] Verifier chỉ được trả:
  - [ ] `accept`.
  - [ ] `reject`.
  - [ ] `move_to_review`.
  - [ ] `correct_owner`.
  - [ ] `reason`.
- [ ] Nếu verifier fail, default là reject hoặc review, không persist.

Acceptance criteria:

- [ ] Case không suspect không dùng call thứ 2.
- [ ] Case suspect dùng tối đa 1 verifier call.
- [ ] Lỗi alias owner/relationship/event trong semantic audit được chặn hoặc chuyển review.

## Phase 6 - Settings, Job UX Và Observability

- [x] Thêm lựa chọn analysis profile trong Settings hoặc job start:
  - [x] Local small staged.
  - [x] Gemini cloud one-shot.
- [x] Hiển thị provider/model/profile trong analysis run.
- [x] Hiển thị call count mỗi chapter.
- [ ] Hiển thị trạng thái:
  - [ ] `one_shot_completed`.
  - [ ] `repaired`.
  - [ ] `verified`.
  - [ ] `moved_to_review`.
- [ ] Ghi event khi fallback làm tăng số call.
- [ ] UI phải phân biệt lỗi provider/key/quota với lỗi validation dữ liệu.

Acceptance criteria:

- [ ] Người dùng biết chương nào tốn 1 call và chương nào phải fallback.
- [ ] Không expose API key ra frontend hoặc logs.

## Phase 7 - Benchmark Và Regression Gate

- [ ] Tạo fixture benchmark tối thiểu:
  - [ ] Phàm Nhân Tu Tiên chương 1-5.
  - [ ] Một chương dài có nhiều nhân vật.
  - [ ] Một chương ít nhân vật để đo false positive.
- [ ] Ghi metrics:
  - [ ] Calls/chapter.
  - [ ] Input/output tokens.
  - [ ] Latency.
  - [ ] Estimated cost.
  - [ ] Parse failure rate.
  - [ ] Evidence quote match rate.
  - [ ] Character recall/precision sampled.
  - [ ] Alias owner precision sampled.
  - [ ] Relationship stable precision sampled.
  - [ ] Field owner precision sampled.
- [ ] So sánh với local staged profile hiện tại.
- [ ] Chốt default:
  - [ ] Flash nếu chất lượng đủ và chi phí thấp.
  - [ ] Pro nếu Flash fail trên alias/relationship hoặc chương khó.
- [ ] Ghi benchmark report vào `output/` hoặc `docs/` tùy mục đích, không commit raw user uploads/secrets.

Acceptance criteria:

- [ ] Có bảng so sánh local vs Gemini cloud.
- [ ] Có quyết định model mặc định dựa trên số liệu, không dựa trên cảm giác.

## Phase 8 - Rollout Và Merge Strategy

- [ ] Giữ feature flag off mặc định nếu branch chưa đủ regression.
- [ ] Merge từng lát:
  - [ ] Provider Gemini + structured smoke test.
  - [x] Schema/prompt registry.
  - [ ] Cloud one-shot runner.
  - [ ] Validator/targeted verifier.
  - [ ] UI/settings/metrics.
  - [ ] Benchmark report.
- [ ] Cập nhật `CHANGELOG.md` dưới `Unreleased` khi behavior user-visible sẵn sàng.
- [ ] Nếu storage schema đổi, cập nhật migration và `STORAGE_SCHEMA_VERSION`.
- [ ] Nếu API contract đổi, cập nhật docs API.

Acceptance criteria:

- [ ] Nhánh có thể merge từng phần mà không làm local analysis regress.
- [ ] Có rollback path: tắt profile cloud và quay lại local staged.

## Thiết Kế Call Budget Đề Xuất

```text
Normal chapter:
  1 call: Gemini structured one-shot extraction

Schema/JSON failure:
  1 call: extraction
  1 call: repair same schema or compact re-ask

Semantic suspect:
  1 call: extraction
  1 call: targeted verifier over suspect_items only

Worst allowed MVP:
  1 extraction + 1 repair + 1 targeted verifier = 3 calls/chapter
```

Không cho phép cloud profile quay lại kiểu local hiện tại với nhiều call theo từng character/field/value, trừ khi người dùng bật debug/deep-analysis mode riêng.

## Các File Dự Kiến Sẽ Đụng

- `crates/ai/src/lib.rs`
- `crates/ai/src/types.rs`
- `crates/ai/src/providers/gemini.rs`
- `crates/ai/src/providers/openai_compatible.rs`
- `crates/api/src/lib.rs`
- `crates/api/src/services/llm_json.rs` hoặc module thay thế `llm_structured.rs`
- `crates/api/src/services/analysis_pipeline.rs`
- `crates/api/src/services/byok.rs`
- `crates/core/src/extraction.rs` hoặc prompt/schema module mới
- `crates/storage/src/repositories/analysis.rs`
- `apps/web/src/routes/settings/+page.svelte`
- `apps/web/src/routes/projects/[projectId]/analysis/+page.svelte`
- `docs/checklists/03-byok-ai-checklist.md`
- `docs/checklists/10-major-group-extraction-checklist.md`

Trước khi thêm logic lớn vào `crates/api/src/lib.rs`, `crates/core/src/extraction.rs`, hoặc route Svelte lớn, phải kiểm tra `docs/checklists/11-module-refactor-checklist.md`.

## Rủi Ro Và Cách Giảm

- [ ] One-shot output quá lớn.
  - [ ] Giảm schema cho MVP, chỉ character/relationship/appearance/review.
  - [ ] Split theo section lớn khi cần.
- [ ] Model mạnh nhưng vẫn suy diễn relationship/event.
  - [x] Bắt buộc `relationship_scope`.
  - [x] Persist whitelist scope.
- [ ] Alias owner sai kéo hỏng mention highlight.
  - [ ] Backend gate owner resolution.
  - [ ] Suspect verifier cho alias khác họ/tên hoặc giống character khác.
- [ ] Cost tăng do prompt quá dài.
  - [ ] Context builder chỉ đưa known aliases liên quan.
  - [ ] Log token/cost theo chapter.
- [ ] Gemini API mode khác OpenAI-compatible.
  - [ ] Tách direct Gemini provider khỏi generic OpenAI-compatible provider.
- [ ] Chất lượng tốt trên một truyện nhưng kém trên truyện khác.
  - [ ] Benchmark nhiều thể loại trước khi chọn default.

## Definition Of Done

- [x] Có thể chọn Gemini cloud one-shot profile.
- [ ] Một chương fixture chạy xong với 1 API call và persist vào DB.
- [ ] Fallback repair/verifier chỉ chạy khi validator yêu cầu.
- [ ] Metrics call count/token/cost hiển thị hoặc log được.
- [ ] Local llama.cpp profile vẫn hoạt động.
- [ ] Regression report chứng minh chất lượng không thấp hơn local staged trên các lỗi trọng yếu.
