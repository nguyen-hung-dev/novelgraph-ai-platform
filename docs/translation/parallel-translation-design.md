# Parallel Translation Design

## Mục Tiêu

Pipeline dịch cần chạy song song với pipeline phân tích, nhưng không được làm mất tính đúng đắn của dữ liệu phân tích.

Luồng mục tiêu:

```text
Import -> Split -> Segment
       -> Analysis Job
       -> Translation Job
       -> Glossary/Memory Update
       -> Review
       -> Reading Projection
```

## Các Đơn Vị Dữ Liệu

- `chapter`: chương gốc.
- `source_segment`: đoạn nhỏ trong chương, dùng cho dịch và evidence alignment.
- `translation_segment`: bản dịch của một source segment.
- `glossary_entry`: thuật ngữ, tên riêng, địa danh, tổ chức, vật phẩm.
- `style_profile`: quy tắc giọng văn, xưng hô, cách giữ Hán Việt/Anh Việt.
- `translation_job`: job dịch theo chương, range hoặc toàn truyện.
- `translation_review_item`: điểm cần người dùng duyệt lại.

## Chạy Song Song Với Analysis

Translation có thể bắt đầu sau khi hoàn tất split và segmentation. Analysis và translation có thể chạy độc lập:

- Analysis tạo entity memory, relationship memory và world memory.
- Translation dùng entity memory đã có để giữ tên riêng nhất quán.
- Translation có thể tạo glossary candidate mới.
- Glossary candidate cần review trước khi áp dụng toàn truyện.
- Translation không được chờ người dùng duyệt từng candidate mới có thể chạy tiếp.
- Khi analysis tạo thêm alias/entity canonical name, translation job phải có cơ chế nhận memory mới và đánh dấu segment liên quan cần kiểm tra hoặc dịch lại.
- Khi local LLM/backend mất kết nối, job nên pause ở trạng thái có thể resume thay vì fail vĩnh viễn.

## Agentic Automation

Luồng dịch và phân tích phải ưu tiên tự động hóa:

- Agent tự chọn bước tiếp theo dựa trên trạng thái chương, segment, glossary và dependency.
- Agent tự retry lỗi tạm thời và tự repair output sai schema.
- Review item chỉ là tín hiệu cần kiểm tra, không phải cổng chặn pipeline.
- Prompt dịch phải nằm trong prompt registry có version và không hardcode trong code gọi provider.
- Tên chế độ, trạng thái job, lỗi UX và mô tả model phải lấy từ copy catalog hoặc i18n registry.

## Không Dùng Bản Dịch Là Evidence Chính

Nếu source text còn khả dụng, fact extraction phải trích evidence từ source text. Bản dịch có thể dùng để hiển thị hoặc hỗ trợ người đọc, nhưng không nên thay source text trong evidence spans.

## Chế Độ Dịch

- Dịch từng chương.
- Dịch theo range chương.
- Dịch incremental khi người dùng đọc.
- Dịch nền sau khi analysis hoàn tất.
- Dịch lại khi glossary/style guide thay đổi.
- Dịch lại một phần khi raw text, alias, glossary hoặc entity canonical name bị sửa.

## Inline Editing Và Stale State

Người dùng có thể sửa trực tiếp bản dịch, glossary, alias hoặc raw text ngay tại UI. Mọi chỉnh sửa phải ghi DB và cập nhật trạng thái phụ thuộc:

- Sửa raw text của chương đánh dấu stale cho source segment, analysis observation, evidence span và translation segment liên quan.
- Sửa glossary entry đánh dấu stale cho các translation segment dùng thuật ngữ đó.
- Sửa entity name hoặc alias cập nhật entity memory và đánh dấu segment liên quan cần kiểm tra lại.
- Sửa translation segment tạo correction event và không bị job tự động ghi đè nếu không có force rerun rõ ràng.
- UI cần cho phép nháy đúp để sửa, blur hoặc Enter để lưu, Escape để hủy.

## Event Progress

Translation job nên phát event cùng hệ thống job progress:

```json
{
  "type": "translation_progress",
  "project_id": "proj_...",
  "job_id": "job_...",
  "chapter_num": 12,
  "segment_done": 8,
  "segment_total": 24,
  "stage": "translating"
}
```
