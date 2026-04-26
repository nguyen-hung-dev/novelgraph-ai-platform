# Agentic Automation And Inline Editing

Tài liệu này khóa nguyên tắc sản phẩm cho rewrite: AI phải có khả năng tự phân tích và dịch truyện theo luồng tự động, còn người dùng chỉ can thiệp khi muốn chỉnh dữ liệu. Khi người dùng chỉnh, dữ liệu phải được ghi trực tiếp vào DB qua API có kiểu rõ ràng.

## Nguyên Tắc Bắt Buộc

- Không hardcode chuỗi hiển thị, thông báo lỗi, nhãn trạng thái, prompt, template dịch, tên preset provider hoặc nội dung UX dài trong code xử lý.
- Chuỗi UI phải đi qua copy catalog hoặc i18n registry.
- Prompt và template AI phải đi qua prompt registry có tên, version và mô tả mục đích.
- Giá trị kỹ thuật như route path, DB enum, migration id, slug nội bộ, test fixture và protocol token được phép là literal nếu chúng là hợp đồng kỹ thuật.
- DB là nguồn sự thật chính cho chương, thực thể, alias, quan hệ, bằng chứng, glossary, bản dịch và trạng thái pipeline.
- Raw LLM output chỉ dùng để debug/audit, không được dùng làm nguồn sự thật chính cho UI.

## Luồng Agent AI

AI agent phải có thể chạy end-to-end mà không cần con người xác nhận từng bước:

- Import hoặc đọc chương.
- Chuẩn hóa raw text.
- Phân tích thực thể, alias, quan hệ, timeline, địa điểm, sự kiện và bằng chứng.
- Dịch chương song song với phân tích khi dữ liệu phụ trợ đã đủ hoặc có thể chạy speculative rồi tự sửa sau.
- Tạo review item cho dữ liệu thiếu tự tin nhưng không chặn toàn bộ pipeline.
- Tự retry, tự repair JSON, tự resume khi backend hoặc local LLM sẵn sàng trở lại.
- Đánh dấu stale cho dữ liệu phụ thuộc khi nguồn bị sửa.

Con người không phải là một bước bắt buộc trong pipeline. Con người là lớp hiệu chỉnh dữ liệu sau hoặc trong khi pipeline chạy.

## UX Inline Editing

Mọi dữ liệu hiển thị có ý nghĩa nghiệp vụ nên chỉnh được trực tiếp tại nơi nó xuất hiện.

- Nháy đúp chuột vào field để vào trạng thái edit.
- `Enter` hoặc blur để lưu.
- `Escape` để hủy.
- Khi lưu thành công, UI cập nhật optimistic rồi xác nhận bằng dữ liệu từ API.
- Khi lưu lỗi, field giữ trạng thái edit và hiển thị lỗi gần field.
- Không làm nhảy layout khi chuyển giữa view và edit.
- Không mở modal cho field ngắn như tên, alias, loại thực thể, mô tả ngắn, nhãn quan hệ hoặc ghi chú.
- Raw text chương và đoạn dịch dài có thể dùng editor panel, nhưng thao tác vẫn phải là sửa trực tiếp và ghi DB.

## Vùng Dữ Liệu Cần Edit Trực Tiếp

- Reading: raw text chương, đoạn tách, ghi chú chương, trạng thái đã đọc.
- Entity: tên chính, alias, loại thực thể, mô tả, trạng thái merge, thuộc tính tùy chỉnh.
- Relationship: loại quan hệ, hướng quan hệ, mức tin cậy, bằng chứng liên quan.
- Timeline: thời điểm, thứ tự, mô tả sự kiện, chapter range.
- Location: tên địa điểm, alias, mô tả, liên kết với sự kiện.
- Translation: đoạn dịch, thuật ngữ, ghi chú phong cách, trạng thái cần dịch lại.
- Glossary: source term, target term, domain, ưu tiên áp dụng.
- Review queue: quyết định chấp nhận, sửa, bỏ qua hoặc gộp.

## Đồng Bộ DB

Mọi chỉnh sửa từ UI phải đi qua typed API và được ghi vào DB ngay.

- Sửa raw chapter text phải tạo revision hoặc correction event.
- Sửa raw chapter text phải đánh dấu stale cho extraction, translation và evidence phụ thuộc.
- Sửa alias hoặc entity name phải cập nhật bảng alias/entity và làm mới projection liên quan.
- Sửa glossary phải đánh dấu stale cho translation segment chịu ảnh hưởng.
- Sửa relationship hoặc timeline phải giữ audit trail để biết dữ liệu do AI tạo hay do người dùng sửa.
- API trả về bản ghi đã lưu hoặc projection mới nhất, không chỉ trả `{ ok: true }`.

## Ranh Giới MVP

MVP chưa cần hoàn thiện toàn bộ editor nâng cao, nhưng kiến trúc không được khóa vào luồng thủ công.

- Cần có typed patch API cho ít nhất chapter raw text, entity alias, relationship note, glossary term và translation segment.
- Cần có cơ chế stale marker cho dữ liệu phụ thuộc.
- Cần có audit/correction log tối thiểu.
- Cần có nền tảng copy catalog và prompt registry trước khi UI/prompt phình lớn.
- Cần có checklist UX để mọi field mới cân nhắc inline edit ngay từ đầu.
