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
