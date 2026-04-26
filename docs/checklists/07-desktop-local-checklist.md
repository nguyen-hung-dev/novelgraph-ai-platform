# Checklist Phase 7 - Desktop Và Local Mode

## Tauri Shell

- [ ] Tạo `apps/desktop`.
- [ ] Kết nối shared web UI.
- [ ] Cấu hình app window.
- [ ] Cấu hình app data directory.
- [ ] Cấu hình file dialog.
- [ ] Cấu hình updater sau này.

## Local Backend

- [ ] Chạy Rust backend/core trong desktop mode.
- [ ] SQLite local database.
- [ ] Local upload storage.
- [ ] Local export directory.
- [ ] Health check nội bộ.
- [ ] Realtime bridge.

## llama.cpp Sidecar

- [ ] Chọn binary strategy.
- [ ] Detect platform.
- [x] Start sidecar.
- [x] Stop sidecar.
- [x] Health check.
- [x] Model path config.
- [ ] Context window config.
- [ ] GPU backend config.

## Offline Behavior

- [ ] App mở được khi không có internet.
- [ ] Project local không phụ thuộc hosted API.
- [ ] BYOK web settings không bắt buộc trong desktop local.
- [ ] Export/import hoạt động offline.

## Packaging

- [ ] Windows build.
- [ ] macOS build.
- [ ] Linux build nếu có.
- [ ] Không bundle model quá lớn mặc định.
- [ ] Tài liệu hướng dẫn cài model local.

## Local Security

- [ ] Không lưu cloud API key không mã hóa.
- [ ] Không log key.
- [ ] Local database path rõ ràng.
- [ ] Export không tự động chứa secret config.
