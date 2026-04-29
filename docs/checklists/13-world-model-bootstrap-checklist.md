# Checklist World Model Bootstrap Và Dynamic Taxonomy

Ngày lập kế hoạch: 2026-04-30
Phạm vi: nâng cấp pipeline phân tích truyện để AI bootstrap thế giới quan/taxonomy từ các chương đầu, sau đó dùng taxonomy động để điều chỉnh prompt phân tích từng chương.
Mục tiêu chính: giảm hardcode domain text trong BE, phân biệt đúng `Nhân vật`, `Tổ chức`, `Địa điểm`, `Vật phẩm`, `Khái niệm`, `Chức danh`, và kiểm soát graph bằng validator thay vì rule tiếng Việt rải rác.

## Mục Tiêu

- [ ] Tạo bước `world_model_bootstrap` đọc 5-10 chương đầu để dựng taxonomy ban đầu.
- [ ] Lưu taxonomy/world model vào DB theo version, có evidence và `first_seen_chapter`.
- [ ] Chuyển chapter extraction từ schema chỉ có `characters[]` sang schema entity graph rộng hơn:
  - [ ] `entities[]`
  - [ ] `relationships[]`
  - [ ] `observations[]`
  - [ ] `taxonomy_updates[]`
  - [ ] `review_items[]`
- [ ] Prompt phân tích từng chương được build động từ taxonomy hiện có.
- [ ] BE không hardcode các nhãn domain như `môn phái`, `bang phái`, `hộ pháp`, `đường chủ`.
- [ ] BE vẫn giữ schema lõi ổn định để validate output của AI.
- [ ] UI tách rõ entity kind:
  - [ ] Nhân vật
  - [ ] Tổ chức
  - [ ] Địa điểm
  - [ ] Vật phẩm
  - [ ] Khái niệm
  - [ ] Chức danh / vai trò
- [ ] Không lộ spoiler: UI và prompt từng chương chỉ dùng facts/entity instances được phép thấy tới chương hiện tại.

## Nguyên Tắc Không Hardcode

- [ ] BE được hardcode enum lõi rất ít và ổn định:
  - [ ] `person`
  - [ ] `organization`
  - [ ] `place`
  - [ ] `artifact`
  - [ ] `concept`
  - [ ] `role_reference`
  - [ ] `event`
  - [ ] `unknown`
- [ ] BE không hardcode domain subtype:
  - [ ] Không hardcode `Thất huyền môn = môn phái`.
  - [ ] Không hardcode `Dã Lang bang = bang phái`.
  - [ ] Không hardcode `Bách đoán đường = phân đường`.
  - [ ] Không hardcode `hộ pháp`, `đường chủ`, `đệ tử`, `sư huynh` là role gì trong code.
- [ ] AI bootstrap đề xuất subtype và label:
  - [ ] `entity_subtype = sect`, `display_label = Môn phái`.
  - [ ] `entity_subtype = gang`, `display_label = Bang phái`.
  - [ ] `entity_subtype = branch_hall`, `display_label = Phân đường`.
  - [ ] `role_title = hộ pháp`, `role_title = đường chủ`.
- [ ] BE validate theo taxonomy rows trong DB, không theo keyword tiếng Việt.
- [ ] Nếu AI đề xuất subtype mới, BE lưu vào `taxonomy_updates` hoặc review trước khi dùng rộng.

## Kiến Trúc Đích

- [ ] Luồng bootstrap:
  - [ ] Import novel.
  - [ ] Chọn `bootstrap_chapter_count`, mặc định 10 hoặc min(total_chapters, 10).
  - [ ] Gửi raw text các chương bootstrap cho Gemini.
  - [ ] Gemini trả world model/taxonomy ban đầu.
  - [ ] BE validate evidence, normalize keys, lưu version `story_world_model.v1`.
- [ ] Luồng phân tích chương:
  - [ ] BE lấy taxonomy hiện hành.
  - [ ] BE lấy entity/alias/relationship đã visible tới chương đang chạy.
  - [ ] BE build prompt động cho chương N.
  - [ ] Gemini trả entities/relationships/observations theo taxonomy.
  - [ ] BE validate endpoint kind/subtype/evidence/stability.
  - [ ] BE persist dữ liệu hợp lệ.
  - [ ] BE lưu `taxonomy_updates` nếu chương mới phát sinh khái niệm/subtype mới.
- [ ] Luồng no-spoiler:
  - [ ] Entity instances chỉ được gửi/hiển thị nếu `first_seen_chapter <= current_chapter`.
  - [ ] Relationship facts chỉ được gửi/hiển thị nếu `first_seen_chapter <= current_chapter`.
  - [ ] Taxonomy có thể có 2 mode:
    - [ ] `full_bootstrap_taxonomy`: dùng toàn bộ taxonomy 10 chương làm hướng dẫn ẩn cho AI, không hiển thị trực tiếp.
    - [ ] `strict_visible_taxonomy`: chỉ dùng subtype/field/relation có `first_seen_chapter <= current_chapter`.
  - [ ] Mặc định UI luôn strict no-spoiler.

## Phase 0 - Baseline Và Fixture

- [ ] Chốt dataset test ban đầu:
  - [ ] Project `proj_019dda1cac2279c2b1dd6f2c3c9d871f`.
  - [ ] Novel hiện tại.
  - [ ] Chương 1 -> 5 đã audit.
  - [ ] Sau khi có bootstrap, mở rộng fixture chương 1 -> 10.
- [ ] Lưu các lỗi baseline cần sửa:
  - [ ] `Thất huyền môn` phải là tổ chức/môn phái, không phải nhân vật.
  - [ ] `Dã Lang bang` phải là tổ chức/bang phái.
  - [ ] `Bách đoán đường`, `Thất tuyệt đường` phải là tổ chức/phân đường hoặc đơn vị thuộc tổ chức.
  - [ ] `Đại ca`, `Sư huynh`, `Hàn mẫu`, `Hàn phụ` không được tự động thành canonical person nếu chỉ là role/generic reference.
  - [ ] `Lão thợ rèn <-> Đại ca` huyết thống từ "con lão thợ rèn" phải biến mất.
  - [ ] `Hàn Lập <-> Vũ Nham` không được persist là stable `đối thủ` từ một cảnh thi leo núi.
  - [ ] `Mặc đại phu <-> Hàn Lập/Trương Thiết` không được persist `sư đồ` chỉ từ câu "hai người này theo ta đi nào".
- [ ] Tạo fixtures xuất từ DB:
  - [ ] `fixtures/world_bootstrap/ch01_10_input.json`.
  - [ ] `fixtures/world_bootstrap/expected_taxonomy_minimal.json`.
  - [ ] `fixtures/chapter_extraction/ch01_expected_no_spoiler.json`.
  - [ ] `fixtures/chapter_extraction/ch05_expected_quality_flags.json`.
- [ ] Thêm audit script hoặc query notes để tái kiểm tra:
  - [ ] Evidence lệch chương.
  - [ ] Role-only canonical person.
  - [ ] Relationship từ weak evidence.
  - [ ] Appearance current-scene bị lưu vào profile.
  - [ ] Alias không có evidence.

Acceptance criteria:

- [ ] Có baseline rõ để so sánh trước/sau.
- [ ] Có danh sách lỗi ngữ nghĩa bắt buộc không tái diễn.

## Phase 1 - Core Domain Model

- [ ] Thêm domain structs trong `crates/core`:
  - [ ] `StoryWorldModel`.
  - [ ] `StoryWorldModelVersion`.
  - [ ] `StoryEntityKind`.
  - [ ] `StoryEntitySubtype`.
  - [ ] `StoryRelationType`.
  - [ ] `StoryFieldType`.
  - [ ] `StoryWorldEntity`.
  - [ ] `StoryWorldAlias`.
  - [ ] `StoryWorldRelationship`.
  - [ ] `StoryTaxonomyUpdate`.
  - [ ] `StoryObservation`.
- [ ] Định nghĩa enum lõi:
  - [ ] `EntityKind`: `person`, `organization`, `place`, `artifact`, `concept`, `role_reference`, `event`, `unknown`.
  - [ ] `TaxonomyStatus`: `active`, `proposed`, `deprecated`, `rejected`.
  - [ ] `EvidenceStrength`: `explicit`, `inferred`, `weak`.
  - [ ] `FieldStability`: `stable_profile`, `current_scene`, `uncertain`.
  - [ ] `AliasKind`: `canonical_name`, `stable_alias`, `address_form`, `typo_variant`, `title`, `surface_form`.
- [ ] Không dùng enum Rust cho subtype domain động:
  - [ ] `sect` lưu DB.
  - [ ] `gang` lưu DB.
  - [ ] `branch_hall` lưu DB.
  - [ ] `cultivation_method` lưu DB.
  - [ ] `role_title` lưu DB.
- [ ] Thêm contracts validate:
  - [ ] subtype phải thuộc một kind lõi.
  - [ ] relation type phải khai báo allowed source/target kind.
  - [ ] field type phải khai báo allowed entity kinds.
  - [ ] alias kind phải có policy merge riêng.

Acceptance criteria:

- [ ] Domain model compile được và chưa phá schema cũ.
- [ ] Có unit tests cho normalize key/kind/subtype/relation compatibility.

## Phase 2 - SQLite Schema Và Repository

- [ ] Thêm migrations SQLite:
  - [ ] `story_world_models`.
  - [ ] `story_world_model_versions`.
  - [ ] `story_entity_kinds`.
  - [ ] `story_entity_subtypes`.
  - [ ] `story_relation_types`.
  - [ ] `story_field_types`.
  - [ ] `story_world_entities`.
  - [ ] `story_world_entity_aliases`.
  - [ ] `story_world_relationships`.
  - [ ] `story_world_observations`.
  - [ ] `story_taxonomy_updates`.
- [ ] Thiết kế keys:
  - [ ] `project_id`.
  - [ ] `novel_id`.
  - [ ] `model_version_id`.
  - [ ] `entity_key`.
  - [ ] `entity_kind`.
  - [ ] `entity_subtype_key`.
  - [ ] `first_seen_chapter`.
  - [ ] `last_seen_chapter`.
  - [ ] `evidence_json`.
- [ ] Repository functions:
  - [ ] `create_world_model_version`.
  - [ ] `get_active_world_model`.
  - [ ] `list_visible_world_entities(project_id, novel_id, chapter_num)`.
  - [ ] `list_taxonomy_for_prompt(project_id, novel_id, chapter_num, visibility_mode)`.
  - [ ] `upsert_world_entity`.
  - [ ] `upsert_world_alias`.
  - [ ] `upsert_world_relationship`.
  - [ ] `insert_taxonomy_update`.
- [ ] Migration/backcompat:
  - [ ] Không xóa `story_extraction_records` cũ trong phase đầu.
  - [ ] Có mapper đọc old character/relationship records để UI không vỡ.
  - [ ] Có command backfill thử từ records cũ sang world entities nếu cần.

Acceptance criteria:

- [ ] `cargo sqlx prepare` hoặc compile-time query path không lỗi nếu repo dùng offline metadata.
- [ ] Storage tests pass với in-memory SQLite.
- [ ] Migration không phá DB hiện tại.

## Phase 3 - Prompt Và Schema World Bootstrap

- [ ] Tạo prompt registry mới:
  - [ ] `story_world_bootstrap.v1/system.md`.
  - [ ] `story_world_bootstrap.v1/user.md`.
  - [ ] `story_world_bootstrap.v1/response_schema.json`.
- [ ] Input bootstrap:
  - [ ] Novel metadata.
  - [ ] Chapter range markers.
  - [ ] Raw text chương 1 -> N, mặc định N = 10.
  - [ ] Audit warnings nếu bootstrap rerun.
- [ ] Output bootstrap root:

```json
{
  "schema_version": "story_world_bootstrap.v1",
  "novel_id": "...",
  "chapter_range": { "from": 1, "to": 10 },
  "entity_kinds": [],
  "entity_subtypes": [],
  "relation_types": [],
  "field_types": [],
  "entities": [],
  "relationships": [],
  "review_items": []
}
```

- [ ] `entity_subtypes[]` cần có:
  - [ ] `subtype_key`.
  - [ ] `entity_kind`.
  - [ ] `display_label`.
  - [ ] `description`.
  - [ ] `first_seen_chapter`.
  - [ ] `evidence_quotes`.
- [ ] `relation_types[]` cần có:
  - [ ] `relation_type_key`.
  - [ ] `display_label`.
  - [ ] `source_kind`.
  - [ ] `target_kind`.
  - [ ] `inverse_label`.
  - [ ] `evidence_strength_policy`.
- [ ] `field_types[]` cần có:
  - [ ] `field_type_key`.
  - [ ] `display_label`.
  - [ ] `allowed_entity_kinds`.
  - [ ] `stability_policy`.
- [ ] `entities[]` cần có:
  - [ ] `canonical_name`.
  - [ ] `entity_kind`.
  - [ ] `entity_subtype_key`.
  - [ ] `display_subtype_label`.
  - [ ] `aliases`.
  - [ ] `first_seen_chapter`.
  - [ ] `evidence_quotes`.
  - [ ] `confidence`.
- [ ] Prompt rule:
  - [ ] Không coi tổ chức là nhân vật.
  - [ ] Role-only/generic surface là `role_reference` hoặc review, không phải `person`.
  - [ ] Chỉ tạo entity canonical nếu nó có identity ổn định.
  - [ ] Taxonomy subtype là đề xuất có evidence, không phải code schema.

Acceptance criteria:

- [ ] Bootstrap 10 chương trả JSON parse được bằng Gemini structured output.
- [ ] `Thất huyền môn` được bootstrap là `organization` subtype `sect` hoặc tương đương.
- [ ] `Dã Lang bang` được bootstrap là `organization` subtype bang phái.
- [ ] `Bách đoán đường`/`Thất tuyệt đường` được bootstrap là organization unit/sub-organization.

## Phase 4 - Bootstrap Job Và API

- [ ] Thêm job type:
  - [ ] `world_model_bootstrap`.
  - [ ] `world_model_refresh`.
- [ ] Thêm service:
  - [ ] `services/world_bootstrap.rs`.
  - [ ] `services/world_context.rs`.
  - [ ] `services/world_validation.rs`.
- [ ] API routes:
  - [ ] `POST /projects/:projectId/world-model/bootstrap`.
  - [ ] `GET /projects/:projectId/world-model`.
  - [ ] `GET /projects/:projectId/world-model/taxonomy`.
  - [ ] `POST /projects/:projectId/world-model/refresh`.
- [ ] Job behavior:
  - [ ] Nếu chưa có world model thì Analysis UI đề xuất chạy bootstrap trước.
  - [ ] Nếu user chạy analysis không bootstrap, cloud profile có thể tự chạy bootstrap nhỏ hoặc fallback schema cũ.
  - [ ] Bootstrap output lưu `raw_response_preview`, telemetry, suspect/review counts.
- [ ] Error handling:
  - [ ] Schema mismatch -> fail job rõ.
  - [ ] Evidence không map được -> suspect/review, không persist.
  - [ ] Taxonomy conflict -> `taxonomy_updates` status `proposed`, không auto-active.

Acceptance criteria:

- [ ] Có thể chạy bootstrap từ API.
- [ ] DB có active world model version sau khi chạy.
- [ ] Analysis job sau bootstrap đọc được taxonomy.

## Phase 5 - Dynamic Prompt Context Cho Chapter Extraction

- [ ] Tạo context builder mới:
  - [ ] `build_world_taxonomy_context`.
  - [ ] `build_visible_entity_context`.
  - [ ] `build_visible_relationship_context`.
  - [ ] `build_chapter_world_context`.
- [ ] Context cho chương N gồm:
  - [ ] Novel metadata ngắn.
  - [ ] Taxonomy active.
  - [ ] Entity kinds/subtypes/field types/relation types liên quan.
  - [ ] Entities visible có `first_seen_chapter <= N`.
  - [ ] Aliases visible có `first_seen_chapter <= N`.
  - [ ] Relationships visible có `first_seen_chapter <= N`.
  - [ ] Current chapter text.
- [ ] Không gửi:
  - [ ] Entity fact có first_seen_chapter > N.
  - [ ] Relationship fact có first_seen_chapter > N.
  - [ ] Evidence quote từ chương sau.
- [ ] Token budgeting:
  - [ ] Giới hạn taxonomy context theo top relevant subtypes.
  - [ ] Giới hạn entities theo surface xuất hiện trong current chapter.
  - [ ] Nếu context quá lớn, ưu tiên visible entities/relation types hơn full history.
- [ ] Prompt trace:
  - [ ] Ghi prompt template id/version.
  - [ ] Ghi world model version id.
  - [ ] Ghi taxonomy visibility mode.
  - [ ] Ghi context counts.

Acceptance criteria:

- [ ] Chạy chương 1 sau bootstrap không thấy facts từ chương 2-10 trong output.
- [ ] Prompt chương 1 vẫn biết taxonomy chung để phân loại `Thất huyền môn` là organization nếu text chương 1 có evidence.
- [ ] Prompt hash ổn định với cùng chapter + same world model version.

## Phase 6 - Chapter Extraction Schema V3

- [ ] Tạo prompt/schema mới:
  - [ ] `story_chapter_world_extraction.v3/system.md`.
  - [ ] `story_chapter_world_extraction.v3/user.md`.
  - [ ] `story_chapter_world_extraction.v3/response_schema.json`.
- [ ] Root output:

```json
{
  "schema_version": "story_chapter_world_extraction.v3",
  "chapter_num": 1,
  "world_model_version": "...",
  "entities": [],
  "relationships": [],
  "observations": [],
  "taxonomy_updates": [],
  "review_items": [],
  "call_profile": "cloud_gemini_world_one_shot"
}
```

- [ ] `entities[]`:
  - [ ] `canonical_name`.
  - [ ] `entity_kind`.
  - [ ] `entity_subtype_key`.
  - [ ] `display_subtype_label`.
  - [ ] `aliases` as objects, không chỉ string.
  - [ ] `fields`.
  - [ ] `mentions`.
  - [ ] `first_seen_chapter`.
  - [ ] `evidence_quotes`.
  - [ ] `identity_status`: `known`, `new`, `candidate`, `role_reference`.
- [ ] `aliases[]` object:
  - [ ] `alias_text`.
  - [ ] `alias_kind`: `stable_alias`, `address_form`, `typo_variant`, `title`, `surface_form`.
  - [ ] `evidence_quotes`.
  - [ ] `confidence`.
- [ ] `fields[]`:
  - [ ] `field_type_key`.
  - [ ] `field_label`.
  - [ ] `value`.
  - [ ] `field_stability`: `stable_profile`, `current_scene`, `uncertain`.
  - [ ] `evidence_quotes`.
- [ ] `relationships[]`:
  - [ ] `source_name`.
  - [ ] `target_name`.
  - [ ] `source_entity_kind`.
  - [ ] `target_entity_kind`.
  - [ ] `relation_type_key`.
  - [ ] `source_to_target_label`.
  - [ ] `target_to_source_label`.
  - [ ] `relationship_status`: `known`, `new`, `changed`, `candidate`.
  - [ ] `evidence_strength`: `explicit`, `inferred`, `weak`.
  - [ ] `evidence_quotes`.
- [ ] `observations[]`:
  - [ ] Temporary scene state.
  - [ ] One-scene rivalry/interaction.
  - [ ] Temporary injury/sweat/posture/current action.
  - [ ] Non-persisted but useful facts.
- [ ] `taxonomy_updates[]`:
  - [ ] Proposed subtype.
  - [ ] Proposed relation type.
  - [ ] Proposed field type.
  - [ ] Evidence and rationale.

Acceptance criteria:

- [ ] V3 output parse được.
- [ ] Organization/person/place không lẫn vào nhau.
- [ ] Weak relationship đi vào observations/review, không persist relationship graph.

## Phase 7 - Backend Validators Và Write Gates

- [ ] Entity validator:
  - [ ] Reject entity_kind không nằm trong core enum.
  - [ ] Reject entity_subtype_key không active trong taxonomy, trừ khi đi qua taxonomy_updates.
  - [ ] Reject `person` nếu chỉ là role/generic phrase và identity_status không đủ mạnh.
  - [ ] Organization không được persist vào group/card Nhân vật.
  - [ ] Role reference không auto-merge vào person cũ nếu chỉ match generic surface.
- [ ] Relationship validator:
  - [ ] source/target entity phải resolve duy nhất.
  - [ ] relation_type_key phải active trong taxonomy.
  - [ ] source/target kind phải khớp relation type.
  - [ ] evidence_strength phải là `explicit` để persist graph.
  - [ ] `inferred`/`weak` chuyển sang observations hoặc review_items.
  - [ ] Known relationship labels được canonicalize theo prior hints.
  - [ ] Không persist one-scene contest/command/greeting/flattery/co-presence.
- [ ] Field validator:
  - [ ] field_type_key phải hợp lệ với entity kind.
  - [ ] field_stability phải là `stable_profile` để persist profile.
  - [ ] `current_scene` đi vào observations.
  - [ ] value phải là display label ngắn.
  - [ ] evidence quote phải map được vào current chapter.
- [ ] Alias validator:
  - [ ] alias_kind bắt buộc.
  - [ ] typo_variant/address_form không được auto-merge mạnh như stable_alias.
  - [ ] Alias phải có evidence nếu được tạo từ current chapter.
  - [ ] Confidence không auto-set 1.0 nếu thiếu evidence.
- [ ] Taxonomy update validator:
  - [ ] Không auto-active subtype/relation mới nếu conflict với active taxonomy.
  - [ ] Nếu update an toàn và có evidence rõ, status `active`.
  - [ ] Nếu mơ hồ, status `proposed` và cần review.

Acceptance criteria:

- [ ] Các lỗi audit chương 1-5 bị chặn bởi validator, không chỉ phụ thuộc prompt.
- [ ] Có suspect/review reason rõ để debug.

## Phase 8 - Storage Mapping Và Backward Compatibility

- [ ] Quyết định mapping UI/read model:
  - [ ] Cũ: `character_records`, `relationship_records`.
  - [ ] Mới: `world_entities`, `world_relationships`, `world_observations`.
- [ ] API workspace snapshot:
  - [ ] Thêm `world_model`.
  - [ ] Thêm `entity_kinds`.
  - [ ] Thêm `entity_subtypes`.
  - [ ] Thêm `world_entities`.
  - [ ] Thêm `world_relationships`.
  - [ ] Thêm `world_observations`.
  - [ ] Giữ fields cũ trong giai đoạn chuyển tiếp.
- [ ] Backfill:
  - [ ] Convert character records cũ thành `person` nếu an toàn.
  - [ ] Convert organization-like records cũ sang `organization` nếu taxonomy xác nhận.
  - [ ] Generic role-only records chuyển `role_reference` hoặc review.
  - [ ] Relationship cũ được revalidate theo endpoint kind.
- [ ] No destructive migration:
  - [ ] Không xóa records cũ trong phase đầu.
  - [ ] Có flag để UI dùng old/new source.

Acceptance criteria:

- [ ] App vẫn mở được project cũ.
- [ ] New project có thể dùng world model path.
- [ ] Có rollback bằng feature flag.

## Phase 9 - UI/UX

- [ ] Reading sidebar:
  - [ ] Tách tab/filter entity kinds.
  - [ ] Nhân vật chỉ hiện `person`.
  - [ ] Tổ chức hiện `organization`.
  - [ ] Địa điểm hiện `place`.
  - [ ] Vật phẩm/khái niệm hiện khi có dữ liệu.
- [ ] Info card:
  - [ ] Header hiển thị subtype label động từ taxonomy.
  - [ ] `Thất huyền môn` hiển thị `Môn phái`, không phải `Nhân vật`.
  - [ ] Relationship card dùng labels động.
  - [ ] Evidence disclosure giữ full width như hiện tại.
- [ ] Analysis UI:
  - [ ] Có step bootstrap world model.
  - [ ] Hiển thị active world model version.
  - [ ] Hiển thị taxonomy updates cần review.
  - [ ] Hiển thị suspect/review counts theo chapter.
- [ ] Review UI:
  - [ ] Approve/reject proposed taxonomy subtype.
  - [ ] Approve/reject proposed relation type.
  - [ ] Convert role_reference thành person nếu user xác nhận.
  - [ ] Merge/split entities.
- [ ] Copy:
  - [ ] User-facing Vietnamese copy đưa vào copy catalog, không hardcode rải trong Svelte route.

Acceptance criteria:

- [ ] Người dùng thấy `Thất huyền môn` trong khu Tổ chức.
- [ ] Click entity/alias vẫn mở đúng info.
- [ ] Không lộ entity/fact chương sau khi đang đọc chương trước.

## Phase 10 - Testing Và Evaluation

- [ ] Unit tests:
  - [ ] taxonomy key normalization.
  - [ ] relation endpoint compatibility.
  - [ ] entity kind/subtype validation.
  - [ ] alias kind merge policy.
  - [ ] no-spoiler filtering.
- [ ] Integration tests:
  - [ ] bootstrap 10 chương fixture.
  - [ ] chapter extraction v3 với taxonomy context.
  - [ ] storage migration.
  - [ ] workspace snapshot old/new compatibility.
- [ ] Golden eval:
  - [ ] Chương 1: Hàn Lập/Tam thúc/Thất huyền môn phân loại đúng.
  - [ ] Chương 3: không tạo `Lão thợ rèn <-> Đại ca` huyết thống.
  - [ ] Chương 4: Hàn Lập/Vũ Nham rivalry đi observation hoặc review.
  - [ ] Chương 5: giảm weak relationship count.
  - [ ] Alias evidence không rỗng nếu alias mới.
- [ ] UI tests:
  - [ ] Reading filters entity kinds.
  - [ ] Info card organization/person render đúng.
  - [ ] Evidence toggle không vỡ layout.
  - [ ] No-spoiler test khi mở chương 1.
- [ ] Regression audit script:
  - [ ] `scripts/audit_semantic_extraction`.
  - [ ] Output txt/json.
  - [ ] Flag role-only person, weak relationship, empty alias evidence, temporary appearance.

Acceptance criteria:

- [ ] Test suite có thể chạy trước mỗi rerun benchmark.
- [ ] Audit report sau rerun 1-10 cho thấy lỗi high-risk giảm rõ rệt.

## Phase 11 - Rollout Và Feature Flags

- [ ] Feature flags:
  - [ ] `world_model_bootstrap_enabled`.
  - [ ] `chapter_world_extraction_v3_enabled`.
  - [ ] `strict_no_spoiler_taxonomy`.
  - [ ] `world_ui_enabled`.
- [ ] Rollout sequence:
  - [ ] Implement domain/schema behind flags.
  - [ ] Implement bootstrap job.
  - [ ] Implement v3 chapter extraction in parallel with v2.
  - [ ] Run same novel with v2 and v3.
  - [ ] Compare audit reports.
  - [ ] Switch UI read model after v3 quality passes.
- [ ] Telemetry:
  - [ ] bootstrap api_call_count/token/cost.
  - [ ] taxonomy counts.
  - [ ] entity kind counts.
  - [ ] relationship persisted/review/rejected counts.
  - [ ] validator rejection reasons.
- [ ] Docs:
  - [ ] Update `docs/data-model.md`.
  - [ ] Update `docs/api-contract.md`.
  - [ ] Update `README.md` / `README.vi.md` only after feature is usable.
  - [ ] Add ADR for world model architecture.

Acceptance criteria:

- [ ] Có thể bật/tắt world model path mà không phá cloud Gemini v2.
- [ ] Có benchmark v2 vs v3 trên cùng 10 chương.

## Checklist Thứ Tự Triển Khai Đề Xuất

- [ ] 1. Tạo ADR ngắn: core schema ổn định + taxonomy động trong DB.
- [ ] 2. Thêm domain structs và SQLite migrations cho world model.
- [ ] 3. Tạo prompt/schema `story_world_bootstrap.v1`.
- [ ] 4. Tạo bootstrap service + API + job telemetry.
- [ ] 5. Tạo dynamic world context builder.
- [ ] 6. Tạo prompt/schema `story_chapter_world_extraction.v3`.
- [ ] 7. Implement validators/write gates cho entity kind, relation endpoint, evidence_strength, field_stability.
- [ ] 8. Lưu v3 output song song với records cũ hoặc qua read model tương thích.
- [ ] 9. Cập nhật Reading UI tách entity kind.
- [ ] 10. Chạy rerun chương 1-10 và tạo audit report so sánh.
- [ ] 11. Chỉ sau khi đạt acceptance criteria mới thay cloud profile mặc định từ v2 sang v3.

## Acceptance Criteria Tổng

- [ ] `Thất huyền môn` được nhận diện là `organization` subtype `Môn phái`.
- [ ] `Dã Lang bang` được nhận diện là `organization` subtype `Bang phái`.
- [ ] `Bách đoán đường`/`Thất tuyệt đường` được nhận diện là organization unit/phân đường.
- [ ] Role-only/generic surfaces không còn tự động thành canonical person.
- [ ] Relationship chỉ persist khi endpoint kind hợp lệ và evidence_strength explicit.
- [ ] Weak/inferred relationship đi vào review/observation.
- [ ] Temporary appearance không lưu vào stable character profile.
- [ ] Alias mới có evidence hoặc bị hạ confidence/review.
- [ ] Prompt chương N không nhận facts từ chương sau.
- [ ] UI không hiển thị spoiler từ world model.
- [ ] Không có hardcoded domain text kiểu `môn phái`, `bang phái`, `hộ pháp` trong BE logic.
- [ ] Có audit report chứng minh chất lượng v3 tốt hơn v2 trên cùng 10 chương.
