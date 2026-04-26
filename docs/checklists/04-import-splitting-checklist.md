# Checklist Phase 4 - Import Và Tách Chương

## Upload

- [x] API preview import.
- [x] API confirm import.
- [ ] Giới hạn kích thước file.
- [ ] Kiểm tra định dạng TXT.
- [ ] Kiểm tra định dạng Markdown.
- [ ] Normalize line endings.
- [ ] Detect encoding an toàn.

## Tách Chương

- [x] Heuristic tách chương cơ bản.
  - [x] `Chapter 1`.
  - [x] `Chương 1`.
  - [x] `第1章`.
  - [x] Markdown heading.
- [x] Preview danh sách chương.
- [ ] Cho phép chỉnh rule tách.
- [ ] Cho phép gộp/tách thủ công trong preview.
- [x] Lưu chapter order ổn định.

## Storage

- [x] Lưu metadata novel.
- [ ] Lưu source file reference.
- [x] Lưu chapter content.
- [ ] Lưu word/char count.
- [x] Lưu source language nếu có.
- [ ] Hỗ trợ desktop local path.
- [ ] Hỗ trợ web object storage.

## UI

- [ ] Upload dialog.
- [ ] Import preview table.
- [ ] Chapter count summary.
- [ ] Warning khi split không chắc chắn.
- [ ] Confirm import.
- [ ] Import progress.

## Tests

- [x] Fixture tiếng Việt.
- [x] Fixture tiếng Anh.
- [x] Fixture tiếng Trung.
- [x] Fixture Markdown.
- [x] File không có heading rõ.
- [ ] File quá ngắn.
- [ ] File quá lớn.
