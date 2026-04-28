# NovelGraph AI Platform

Nền tảng phân tích tiểu thuyết bằng AI với giao diện kiểu desktop, chạy được trên website và ứng dụng desktop local.

Phiên bản foundation hiện tại: `0.12.0`.

## Giấy Phép

Repo này được publish source-available theo PolyForm Noncommercial License 1.0.0. Bạn được dùng, đọc, chỉnh sửa và phân phối lại cho mục đích phi thương mại theo điều khoản giấy phép. Mọi mục đích thương mại đều bị cấm nếu chưa có giấy phép thương mại riêng từ chủ sở hữu bản quyền.

NovelGraph AI Platform là dự án rewrite mới, lấy cảm hứng từ AI Reader V2. Mục tiêu là biến truyện dài và tiểu thuyết thành các lớp dữ liệu có cấu trúc: sơ đồ quan hệ nhân vật, bản đồ thế giới, timeline, bách khoa nhân vật và địa điểm, chỉ mục cảnh, cùng chat RAG có dẫn chứng từ văn bản gốc.

Định hướng sản phẩm:

- Website hosted: người dùng đăng nhập, tải dữ liệu lên web và tự nhập API key LLM của riêng mình.
- Desktop local: chạy bằng Tauri, lưu dữ liệu offline và có thể dùng AI local.
- Một giao diện chung: workspace đầy đủ như ứng dụng desktop, không phải landing page.

## Mục Tiêu

- Import file TXT/Markdown và tách chương ổn định.
- Trích xuất fact từ từng chương, có evidence span gắn với văn bản gốc.
- Tạo hồ sơ cho nhân vật, địa điểm, tổ chức, vật phẩm và khái niệm.
- Tạo graph quan hệ, bản đồ thế giới, timeline, phe phái, chỉ mục cảnh và encyclopedia.
- Dịch truyện song song với phân tích AI, giữ glossary và alignment với văn bản gốc.
- Hỗ trợ project riêng tư trên website và project offline trên desktop.
- Cho phép người dùng tự dùng API key của OpenAI, Anthropic, DeepSeek, Gemini, Qwen hoặc provider tương thích.
- Giữ UI theo hướng công cụ làm việc: sidebar, tab, split pane, bảng dữ liệu và progress panel.

## Stack Đề Xuất

| Lớp | Định hướng |
|---|---|
| Frontend | SvelteKit 2 + Svelte 5 + TypeScript |
| Desktop | Tauri 2 |
| Backend | Rust + Axum + Tokio |
| Database | SQLite cho desktop, PostgreSQL cho website |
| Search/RAG | SQLite FTS/vector local, PostgreSQL full-text + pgvector hoặc Qdrant trên web |
| AI Web | BYOK proxy cho OpenAI-compatible providers và Anthropic |
| AI Desktop | llama.cpp `llama-server` với GGUF models |
| Storage Web | S3/R2/MinIO-compatible object storage |

## Nguyên Tắc Kiến Trúc

Dự án mới nên đi theo hướng evidence-first. LLM output không nên là nguồn sự thật chính. Thay vào đó, hệ thống cần lưu:

- Source chapter text.
- Evidence spans.
- Observations có confidence.
- Review decisions của người dùng.
- Graph/map/timeline/encyclopedia được tạo từ projections hoặc cache.

Pipeline mục tiêu:

```text
Import -> Split -> Prescan -> ExtractChapter[n] -> Normalize -> Aggregate
       -> IndexRAG -> BuildWorld -> BuildTimeline -> BuildVisualCache -> Review
```

## Trạng Thái Hiện Tại

Repo đang ở giai đoạn foundation. Hiện đã có Rust backend foundation với các crate `core`, `storage`, `jobs`, `ai`, `api`, endpoint Axum `/health`, migration/repository SQLite, API project, preview/confirm import truyện, lưu source segment, tạo analysis job pending, lưu trạng thái phân tích theo từng chương, tạo translation job, kiểm tra trạng thái job, endpoint cancel/pause job, lưu job event, endpoint local llama.cpp health/models/chat, endpoint draft extraction local cho một chương, endpoint aggregate workspace snapshot cho từng project, cơ chế xóa project theo hai chế độ archive hoặc purge dữ liệu, và local runtime manager cho llama.cpp để chọn GGUF có sẵn trên máy, tải preset nhỏ về thư mục `models/` trong repo, rồi start/stop `llama-server` ngay từ màn Settings. Repo hiện cũng đã có sẵn bundle runtime Windows của `llama.cpp` trong `tools/llama.cpp`, và backend sẽ tự ưu tiên dùng `llama-server.exe` ở đó nếu bạn chưa đặt `LLAMA_CPP_SERVER_BIN`. Phần frontend `apps/web` hiện đã nối dữ liệu thật cho bookshelf, overview, import preview/confirm, reading, analysis runner và Settings local LLM bằng typed API client phía server; đồng thời đã có nút xóa project trên bookshelf, chế độ sáng/tối/system cho toàn app, và modal chỉnh cỡ chữ, dãn dòng cho màn đọc. Màn review đã được chuyển thành placeholder rõ ràng cho tới khi observation persistence và review-item API hoàn thiện.

Hiện đã có:

- README và tài liệu GitHub cơ bản.
- Roadmap.
- Kế hoạch triển khai.
- Ghi chú bảo mật BYOK.
- ADR đầu tiên.
- `.codex` operating context cho AI coding agent.
- Root `pnpm` workspace và SvelteKit web workspace shell.
- Launcher Windows `scripts/dev-stack.ps1` và `scripts/dev-stack.bat` để chạy BE/FE cùng lúc và dọn process khi phiên CLI kết thúc.

## Tài Liệu Chính

- [README English](README.md)
- [Roadmap](ROADMAP.md)
- [Checklist triển khai](docs/checklists/README.md)
- [Implementation plan](docs/implementation-plan.md)
- [Product requirements](docs/product-requirements.md)
- [Data model](docs/data-model.md)
- [API contract](docs/api-contract.md)
- [Deployment model](docs/deployment.md)
- [Thiết kế dịch truyện song song](docs/translation/README.md)
- [BYOK security notes](docs/security-byok.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)

## Milestone Đầu Tiên

Không nên bắt đầu bằng graph/map/timeline phức tạp. Milestone đầu tiên nên là nền tảng:

- Schema workspace/project cho desktop và web.
- Auth boundary cho web.
- BYOK secret model và provider abstraction.
- Import + chapter splitting.
- Durable analysis job queue.
- Extraction contract đầu tiên có evidence spans.
- WebSocket/SSE progress events.

## Lưu Ý Bảo Mật

BYOK là vùng rủi ro cao. API key của người dùng phải được đối xử như mật khẩu:

- Không lưu API key trong browser local storage.
- Không log API key.
- Không để provider auth headers lọt vào prompt traces.
- Nếu lưu persistent key thì phải encrypt at rest.
- Nên có session-only key mode trước.
- Public/shared project không được âm thầm dùng key của owner.
