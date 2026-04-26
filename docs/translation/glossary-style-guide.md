# Glossary Và Style Guide

Glossary là lớp dữ liệu quan trọng để dịch truyện dài ổn định. Không có glossary, model dễ dịch lệch tên riêng, cấp bậc, pháp bảo, địa danh và thuật ngữ tu luyện qua từng chương.

## Glossary Entry

Một glossary entry nên có:

- Source term.
- Target term.
- Entity type.
- Aliases.
- Scope.
- Confidence.
- Review status.
- Evidence/source references.

Ví dụ:

```json
{
  "source_term": "黄枫谷",
  "target_term": "Hoàng Phong Cốc",
  "entity_type": "location",
  "aliases": ["Hoàng Phong cốc"],
  "scope": "project",
  "confidence": 0.95,
  "status": "approved"
}
```

## Style Guide

Style guide nên lưu theo project và target language:

- Cách dịch tên riêng.
- Cách giữ hoặc Việt hóa thuật ngữ.
- Cách xử lý xưng hô.
- Giọng văn: cổ phong, hiện đại, trung tính, học thuật.
- Mức độ sát nghĩa hay mượt văn.
- Quy tắc giữ nguyên từ khóa.

## Quan Hệ Với Analysis

- Entity memory từ analysis có thể đề xuất glossary candidate.
- Glossary đã duyệt có thể giúp analysis disambiguation.
- Người dùng sửa glossary phải tạo revision để dịch lại các đoạn liên quan.

## Review

Glossary candidate cần review khi:

- Có nhiều bản dịch cho cùng một source term.
- Một target term trỏ tới nhiều source term.
- Entity type không chắc chắn.
- Tên riêng bị dịch thành nghĩa thường.
- Thuật ngữ world-building bị dịch không nhất quán.

