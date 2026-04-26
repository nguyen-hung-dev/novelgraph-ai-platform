# Thiết Kế Dịch Truyện Song Song Với Phân Tích AI

Mục tiêu của nhánh translation là cho phép hệ thống vừa phân tích truyện bằng AI, vừa dịch truyện theo chương hoặc theo đoạn, dùng chung dữ liệu nền như chapter split, entity memory, glossary, evidence spans và job progress.

Translation không nên là tính năng phụ chạy sau cùng. Nó cần được thiết kế song song với analysis để:

- Giữ tên riêng, thuật ngữ, địa danh và phe phái nhất quán.
- Tận dụng entity/relationship/world memory từ pipeline phân tích.
- Cho phép người dùng đọc song ngữ source/target trong cùng workspace.
- Cho phép review bản dịch theo từng chương, từng đoạn hoặc từng thuật ngữ.
- Không làm hỏng evidence-first analysis bằng bản dịch chưa được kiểm chứng.

## Nguyên Tắc

- Source text luôn là nguồn gốc chính.
- Bản dịch là projection có version, không thay thế source text.
- Mỗi translation segment phải giữ alignment với source segment.
- Glossary và style guide phải được version hóa theo project.
- Dịch thuật phải có review queue riêng, nhưng có thể chia sẻ entity review với analysis.
- Không dùng bản dịch làm evidence cho fact extraction nếu source text còn khả dụng.

## Tài Liệu Liên Quan

- [Parallel translation design](parallel-translation-design.md)
- [Glossary and style guide](glossary-style-guide.md)
- [Translation quality strategy](translation-quality-strategy.md)
- [Translation checklist](../checklists/09-translation-parallel-checklist.md)

