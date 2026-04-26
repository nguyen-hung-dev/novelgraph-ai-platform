# Checklist Phase 5 - Extraction Và Evidence

## Schema

- [ ] Chapter extraction schema.
- [ ] Evidence span schema.
- [ ] Observation schema.
- [ ] Review item schema.
- [ ] Prompt run schema.
- [ ] Version cho schema.

## Prompt Contract

- [ ] Prompt chỉ dùng current chapter và prior context cho phép.
- [ ] Cấm dùng future chapters.
- [ ] Yêu cầu evidence spans.
- [ ] Yêu cầu confidence.
- [ ] Yêu cầu review item cho fact không chắc chắn.
- [ ] Yêu cầu output JSON hợp lệ.

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

