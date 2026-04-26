# Checklist Phase 8 - Sẵn Sàng Release

## Repo

- [ ] `README.md` đúng trạng thái release.
- [ ] `README.vi.md` đúng trạng thái release.
- [ ] `CHANGELOG.md` có mục release.
- [ ] `SECURITY.md` cập nhật.
- [ ] `CONTRIBUTING.md` cập nhật.
- [ ] Không commit file bị ignore.
- [ ] Không commit secret.
- [ ] Không commit database.
- [ ] Không commit model file.

## CI

- [ ] Rust tests pass.
- [ ] Frontend typecheck pass.
- [ ] Frontend lint pass.
- [ ] Secret scan pass.
- [ ] Markdown docs check pass.
- [ ] Build web pass.
- [ ] Build desktop pass nếu release desktop.

## BYOK

- [ ] Session-only key mode hoạt động.
- [ ] Persistent key encryption nếu bật.
- [ ] Redaction tests pass.
- [ ] Prompt traces không chứa key.
- [ ] Error responses không chứa header.
- [ ] Public share không tiêu key ngầm.

## Product

- [ ] Import novel hoạt động.
- [ ] Tách chương hoạt động.
- [ ] Start analysis hoạt động.
- [ ] Progress event hoạt động.
- [ ] Reading view hoạt động.
- [ ] Review queue hoạt động.
- [ ] Export tối thiểu nếu scope release yêu cầu.

## Privacy

- [ ] Private project không public source text.
- [ ] Shared link có permission rõ.
- [ ] Object storage không public nhầm.
- [ ] Logs không chứa user upload content quá mức cần thiết.
- [ ] Có hướng dẫn xóa dữ liệu.

## Versioning

- [ ] Chọn version.
- [ ] Tạo tag.
- [ ] Tạo release notes.
- [ ] Kiểm tra repo visibility trước khi công bố.

