# Checklist Phase 6 - Frontend Workspace

## App Shell

- [x] SvelteKit app scaffold.
- [x] Layout desktop-style.
- [x] Sidebar project navigation.
- [x] Top toolbar.
- [x] Light, dark, and system color modes.
- [x] Main content region.
- [x] Split pane support.
- [x] Responsive desktop-first behavior.

## Routes

- [x] Projects/bookshelf.
- [x] Project delete modal.
- [x] Project overview.
- [x] Import novel.
- [x] Reading.
- [x] Analysis progress.
- [x] Review route placeholder.
- [x] Settings.
- [x] BYOK settings.
- [x] Local llama.cpp settings and model library.

## API Client

- [x] Typed client generated hoặc shared types.
- [x] Error handling.
- [ ] Request id display cho debug.
- [ ] Auth/session handling.
- [ ] Realtime event client.
- [ ] Retry policy cho safe requests.

## BYOK UI

- [x] Provider selector.
- [x] Base URL input.
- [x] Model input.
- [x] API key input.
- [x] Validate key action.
- [x] Masked key display.
- [x] Clear session key.
- [x] Warning về bảo mật.

## Import UI

- [x] File picker.
- [x] Drag and drop.
- [x] Preview table.
- [x] Chapter count.
- [ ] Split warnings.
- [x] Confirm import.

## Reading UI

- [x] Chapter list.
- [x] Chapter content.
- [x] Search trong chapter.
- [x] Cỡ chữ và dãn dòng.
- [x] Entity highlight placeholder.
- [x] Persist reading position.
- [ ] Nháy đúp raw text để sửa trực tiếp hoặc mở editor panel tại đúng chương.
- [ ] Blur hoặc Enter lưu raw text vào DB qua typed API.
- [ ] Escape hủy sửa và khôi phục dữ liệu từ DB/projection hiện tại.
- [ ] Sửa raw text phải hiển thị trạng thái stale cho analysis/translation liên quan.

## Inline Editing UX

- [ ] Component inline edit dùng chung cho text ngắn.
- [ ] Không làm nhảy layout khi chuyển view/edit.
- [ ] Optimistic update rồi reconcile bằng response từ API.
- [ ] Lỗi lưu hiển thị gần field và không mất dữ liệu đang nhập.
- [ ] Entity, alias, relationship, glossary, translation segment đều có đường sửa tại nơi hiển thị.
- [ ] Chuỗi UI lấy từ copy catalog/i18n registry, không hardcode trong component.

## Analysis UI

- [ ] Start analysis.
- [x] Cancel analysis.
- [x] Progress events.
- [x] Current stage.
- [ ] Failed chapter list.
- [ ] Retry failed chapters.
- [ ] Start/Resume chạy theo agentic pipeline, không cần người dùng xác nhận từng chương.
- [ ] Pause request phản hồi ngay trên UI và dừng ở ranh giới an toàn của job.
