# Checklist Phase 5 - Extraction Và Evidence

## Schema

- [x] Chapter extraction schema.
- [ ] Evidence span schema.
- [ ] Observation schema.
- [ ] Review item schema.
- [ ] Prompt run schema.
- [x] Version cho schema.

## Prompt Contract

- [x] Prompt chỉ dùng current chapter và prior context cho phép.
- [x] Cấm dùng future chapters.
- [x] Yêu cầu evidence spans.
- [x] Yêu cầu confidence.
- [x] Yêu cầu review item cho fact không chắc chắn.
- [x] Yêu cầu output JSON hợp lệ.

## Validation

- [ ] Validate JSON schema.
- [ ] Validate quote tồn tại trong chapter text.
- [ ] Validate span nằm trong bounds.
- [ ] Validate không cite future chapter.
- [ ] Validate entity type hợp lệ.
- [ ] Validate relationship direction.

## Persistence

- [ ] Lưu prompt run.
- [ ] Lưu raw provider metadata an toàn.
- [ ] Lưu observations.
- [ ] Lưu evidence spans.
- [ ] Lưu review items.
- [ ] Lưu usage event.
- [ ] Lưu correction event khi người dùng sửa raw text, alias, entity, relationship hoặc glossary.
- [ ] Đánh dấu stale cho observation/evidence/translation phụ thuộc sau khi source text hoặc alias thay đổi.

## Agentic Automation

- [ ] Analysis job chạy được từ đầu đến cuối mà không cần người dùng xác nhận từng chương.
- [ ] Review item không chặn toàn bộ pipeline.
- [ ] Pipeline tự retry, tự repair JSON, và tự resume khi provider hoạt động lại.
- [ ] Raw LLM output chỉ dùng cho audit/debug, không làm nguồn sự thật chính.
- [ ] Prompt dùng prompt registry có version, không hardcode trong handler.

## Inline Editing

- [ ] Chapter raw text có typed API để sửa trực tiếp từ reading UI.
- [ ] Entity name và alias có typed API để sửa tại nơi hiển thị.
- [ ] Relationship label/note có typed API để sửa tại nơi hiển thị.
- [ ] Mỗi chỉnh sửa ghi DB ngay và trả về bản ghi/projection mới nhất.
- [ ] Lỗi lưu phải giữ trạng thái edit để người dùng không mất nội dung đang sửa.

## Recovery

- [ ] Retry transient errors.
- [ ] Retry JSON parse failure bằng compact prompt.
- [ ] Partial success cho một số nhóm fact.
- [ ] Mark failed chapter.
- [ ] Retry failed chapter.
- [ ] Cancel job an toàn.

## Regression

- [ ] Synthetic fixture nhỏ.
- [ ] Public-domain fixture.
- [ ] So sánh entity recall.
- [ ] So sánh relation recall.
- [ ] So sánh evidence validity.
- [ ] Theo dõi parse failure rate.
