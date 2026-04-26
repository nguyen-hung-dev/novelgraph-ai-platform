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

## Không Dùng Bản Dịch Là Evidence Chính

Nếu source text còn khả dụng, fact extraction phải trích evidence từ source text. Bản dịch có thể dùng để hiển thị hoặc hỗ trợ người đọc, nhưng không nên thay source text trong evidence spans.

## Chế Độ Dịch

- Dịch từng chương.
- Dịch theo range chương.
- Dịch incremental khi người dùng đọc.
- Dịch nền sau khi analysis hoàn tất.
- Dịch lại khi glossary/style guide thay đổi.

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

