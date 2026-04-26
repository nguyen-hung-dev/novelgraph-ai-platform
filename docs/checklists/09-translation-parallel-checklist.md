# Checklist Phase 9 - Dịch Truyện Song Song Với Phân Tích

## Thiết Kế Dữ Liệu

- [x] Thêm `source_segments`.
  - [x] Gắn với `chapter_id`.
  - [x] Có `start_char`.
  - [x] Có `end_char`.
  - [x] Có `segment_index`.
- [x] Thêm `translation_jobs`.
  - [x] Gắn với `project_id`.
  - [x] Gắn với `novel_id`.
  - [x] Có `source_language`.
  - [x] Có `target_language`.
  - [x] Có trạng thái job.
- [x] Thêm `translation_segments`.
  - [x] Gắn với `source_segment_id`.
  - [x] Có target text.
  - [x] Có provider/model.
  - [x] Có version.
- [x] Thêm `glossary_entries`.
- [x] Thêm `translation_review_items`.

## Prompt Và Provider

- [ ] Thiết kế translation prompt contract.
- [ ] Truyền glossary đã duyệt vào prompt.
- [ ] Truyền style guide vào prompt.
- [ ] Truyền entity memory liên quan.
- [ ] Không truyền future chapter nếu dịch incremental.
- [ ] Track token usage cho translation job.

## UI

- [ ] Reading view hỗ trợ source/target song song.
- [ ] Cho phép bật/tắt bản dịch.
- [ ] Cho phép chọn target language.
- [ ] Hiển thị trạng thái dịch theo chương.
- [ ] Review bản dịch theo segment.
- [ ] Review glossary candidate.

## Chất Lượng

- [ ] Kiểm tra thiếu segment.
- [ ] Kiểm tra glossary consistency.
- [ ] Kiểm tra named entity preservation.
- [ ] Kiểm tra target language.
- [ ] Kiểm tra hallucination cơ bản.
- [ ] Tạo translation quality report.

## Tích Hợp Với Analysis

- [ ] Analysis memory đề xuất glossary candidate.
- [ ] Glossary đã duyệt hỗ trợ entity disambiguation.
- [ ] Translation không thay source evidence.
- [ ] Review queue phân biệt analysis review và translation review.
- [ ] Có job dịch lại khi glossary thay đổi.
