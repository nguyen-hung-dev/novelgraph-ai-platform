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
- [ ] Lưu prompt trong prompt registry có version.
- [ ] Truyền glossary đã duyệt vào prompt.
- [ ] Truyền style guide vào prompt.
- [ ] Truyền entity memory liên quan.
- [ ] Không truyền future chapter nếu dịch incremental.
- [ ] Track token usage cho translation job.
- [ ] Không hardcode template dịch hoặc nhãn trạng thái trong code gọi provider.

## Agentic Pipeline

- [ ] Job dịch chạy được mà không cần người dùng duyệt từng segment.
- [ ] Job dịch có thể chạy song song với analysis theo dependency rõ ràng.
- [ ] Review item không làm pipeline dừng hẳn.
- [ ] Có resume sau khi backend hoặc local LLM hoạt động lại.
- [ ] Có force rerun để ghi đè dữ liệu AI, nhưng không ghi đè bản user edit nếu chưa xác nhận rõ.

## UI

- [ ] Reading view hỗ trợ source/target song song.
- [ ] Cho phép bật/tắt bản dịch.
- [ ] Cho phép chọn target language.
- [ ] Hiển thị trạng thái dịch theo chương.
- [ ] Review bản dịch theo segment.
- [ ] Review glossary candidate.
- [ ] Nháy đúp translation segment để sửa trực tiếp.
- [ ] Nháy đúp glossary term để sửa trực tiếp.
- [ ] Blur hoặc Enter lưu vào DB; Escape hủy.
- [ ] Chuỗi UI lấy từ copy catalog/i18n registry.

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
- [ ] Đánh dấu stale khi raw text, alias, entity canonical name hoặc glossary thay đổi.
