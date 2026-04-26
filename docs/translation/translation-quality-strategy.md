# Translation Quality Strategy

## Mục Tiêu Chất Lượng

Bản dịch phải:

- Giữ đúng nghĩa.
- Giữ tên riêng nhất quán.
- Giữ thuật ngữ world-building nhất quán.
- Tôn trọng style guide.
- Có thể review theo đoạn.
- Không làm mất alignment với source text.

## Các Loại Kiểm Tra

- Kiểm tra glossary consistency.
- Kiểm tra thiếu đoạn.
- Kiểm tra hallucination: bản dịch thêm nội dung không có trong source.
- Kiểm tra named entity preservation.
- Kiểm tra xưng hô.
- Kiểm tra độ dài bất thường.
- Kiểm tra dấu câu và markdown.

## Review Queue

Translation review item nên được tạo khi:

- Model báo không chắc chắn.
- Glossary conflict.
- Entity name chưa duyệt.
- Đoạn dịch quá ngắn hoặc quá dài so với source.
- Target language không đúng.
- Có dấu hiệu bỏ sót nội dung.

## Regression Fixtures

Nên có fixture nhỏ cho:

- Tiếng Trung sang tiếng Việt.
- Tiếng Anh sang tiếng Việt.
- Tiếng Việt sang tiếng Anh.
- Truyện có nhiều tên riêng.
- Truyện có thuật ngữ tu luyện.
- Markdown có heading và đoạn hội thoại.

