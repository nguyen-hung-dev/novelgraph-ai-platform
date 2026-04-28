# Kiến Trúc Module Và Quy Ước Phân Cấp

Tài liệu này là chuẩn tổ chức thư mục cho NovelGraph AI Platform. Mục tiêu chính là giúp codebase dễ đọc, dễ giao việc cho AI coding agent, và dễ nâng cấp tính năng mà không tiếp tục dồn logic vào các file quá lớn.

## Mục Tiêu

- Mỗi module có trách nhiệm rõ ràng, tên thư mục phản ánh domain hoặc tầng kỹ thuật.
- File route, handler, repository, component, prompt, parser và presenter phải tách riêng khi logic bắt đầu có nhiều nhánh.
- AI có thể mở một cụm file nhỏ để hiểu tính năng thay vì phải đọc toàn bộ `lib.rs`, `sqlite.rs` hoặc một route Svelte lớn.
- Luồng phụ thuộc đi một chiều: app/router gọi service, service gọi domain/storage/AI, domain không gọi ngược app hoặc storage.
- Mỗi lần thêm tính năng mới phải ưu tiên đặt vào module hiện có hoặc tạo module mới đúng tầng, không mở rộng file tổng hợp quá ngưỡng.

## Ngân Sách Dòng

Áp dụng cho file viết tay trong `apps/`, `crates/`, `docs/`, `.codex/` và script vận hành:

| Mức | Giới hạn | Hành động bắt buộc |
|---|---:|---|
| Healthy | Dưới 800 dòng | Có thể tiếp tục chỉnh nếu module vẫn rõ trách nhiệm. |
| Soft limit | Từ 800 dòng | Không thêm feature mới nếu không có kế hoạch tách module trong cùng PR hoặc checklist gần nhất. |
| Hard limit | Từ 1200 dòng | Không thêm logic mới. Phải split trước hoặc chỉ được sửa bug nhỏ có phạm vi hẹp kèm ghi chú debt. |

Ngoại lệ có kiểm soát:

- File generated, lockfile, migration đơn lẻ, fixture snapshot, hoặc changelog tích lũy.
- File legacy đang trong quá trình chia nhỏ, nhưng không được nhận thêm domain mới.
- Prompt contract dài được phép vượt soft limit nếu đã có mục lục, version rõ ràng và không trộn code.

Khi một file chạm soft limit, tạo task trong checklist chuyển đổi trước khi tiếp tục mở rộng. Khi chạm hard limit, việc refactor module là blocker cho mọi feature mới cùng khu vực.

## Quy Tắc Phụ Thuộc

```text
apps/web routes
  -> apps/web feature modules
  -> apps/web API clients
  -> crates/api routes
  -> crates/api services
  -> crates/core domain contracts
  -> crates/storage repositories
  -> database

crates/api services
  -> crates/ai providers
  -> crates/jobs orchestration
  -> crates/storage repositories
  -> crates/core validation

crates/core
  -> pure domain models, validation, schema constants
  -> no HTTP, SQL, filesystem, process, or UI dependency
```

Không tạo phụ thuộc vòng giữa domain. Nếu hai module cần chia sẻ kiểu dữ liệu, chuyển kiểu đó xuống `crates/core` hoặc tạo contract nhỏ hơn.

## Cấu Trúc Mục Tiêu

```text
apps/
  web/
    src/
      routes/
        settings/
        projects/
        projects/[projectId]/
      lib/
        api/
          clients/
          contracts/
        components/
          primitives/
          workspace/
        features/
          analysis/
          byok/
          import/
          local-runtime/
          projects/
          reading/
          settings/
          translation/
        realtime/
        state/
        workspace/
  desktop/
    src-tauri/

crates/
  api/
    src/
      app.rs
      routes/
        analysis.rs
        byok.rs
        health.rs
        local_runtime.rs
        projects.rs
        realtime.rs
      services/
        analysis/
        byok/
        import/
        local_runtime/
        projects/
      realtime/
      errors.rs
  core/
    src/
      domain/
        analysis.rs
        byok.rs
        jobs.rs
        novel.rs
        project.rs
        story.rs
        translation.rs
      prompts/
      schemas/
      validation/
  storage/
    src/
      db/
      mappers/
      repositories/
        analysis.rs
        byok.rs
        jobs.rs
        novel.rs
        project.rs
        story.rs
        translation.rs
      sqlite/
      postgres/
    migrations/
      sqlite/
      postgres/
  ai/
    src/
      providers/
        gemini.rs
        llama_cpp.rs
        openai_compatible.rs
      structured_output/
      token_budget/
  jobs/
    src/
      queue.rs
      runners/
      events.rs
```

Cấu trúc này là hướng đích. Không cần chuyển toàn bộ trong một lần, nhưng mọi code mới nên đi theo layout này.

## Backend

### `crates/api`

`crates/api` chỉ nên chứa HTTP boundary, orchestration service cấp ứng dụng, realtime boundary và lỗi API.

- `src/lib.rs` hoặc `src/main.rs` chỉ khởi tạo app, state, router tổng và dependency injection.
- `routes/*` giữ Axum handler mỏng: parse input, gọi service, trả response.
- `services/*` chứa use case cấp ứng dụng, ví dụ save BYOK config, run one analysis step, import novel.
- Logic extraction dài, prompt repair, provider calls và repository SQL không nằm trực tiếp trong route handler.
- Mỗi route file nên tập trung một domain. Nếu một domain vượt soft limit, tách `routes/domain/*.rs` hoặc chuyển logic xuống service.

### `crates/core`

`crates/core` là tầng domain thuần.

- Chứa model, input/output contract, enum, validation thuần, schema version và parser thuần.
- Không import `axum`, `sqlx`, `reqwest`, `tokio::process`, `tauri`, hoặc Svelte-generated artifacts.
- Khi `domain.rs` tăng lớn, tách thành `domain/mod.rs` và từng domain file.
- Prompt schema và output schema nên có version rõ ràng để AI biết thay đổi nào là breaking.

### `crates/storage`

`crates/storage` là tầng repository và migration.

- Không gom toàn bộ SQLite query vào một file lớn.
- Mỗi repository sở hữu một nhóm bảng và mapper tương ứng.
- Query dùng chung đặt trong helper nhỏ, không tạo service domain trong storage.
- SQLite và PostgreSQL phải giữ contract tương đương; khác biệt dialect được cô lập trong adapter hoặc migration.
- Mapper row-to-domain nên nằm gần repository nhưng tách khỏi function nghiệp vụ nếu file bắt đầu dài.

### `crates/ai`

`crates/ai` là tầng provider và structured output.

- Mỗi provider có module riêng.
- Provider không biết HTTP route hoặc DB user ownership.
- Redaction, retry, JSON repair, token budget và model capability nằm trong module chuyên trách.
- BYOK key thật chỉ đi qua backend service/provider, không lộ ra frontend hoặc log.

### `crates/jobs`

`crates/jobs` sở hữu queue, runner, cancellation, retry và event emission.

- Request HTTP không chạy long-running analysis trực tiếp.
- Job event là contract typed để frontend invalidate đúng vùng dữ liệu.
- Runner gọi service hoặc provider theo step nhỏ, có checkpoint rõ ràng.

## Frontend

### `apps/web/src/routes`

Route SvelteKit chỉ nên giữ layout cấp route, `load`, action mapping và composition.

- Không đặt toàn bộ Reading, Settings hoặc Analysis workflow trong một `+page.svelte` lớn.
- Component domain đặt trong `src/lib/features/<feature>/`.
- Server action gọi `src/lib/server` hoặc API client typed; không viết lại HTTP shape trong route.
- Route đang vượt soft limit phải được tách component trước khi thêm workflow mới.

### `apps/web/src/lib/features`

Mỗi feature là một thư mục độc lập:

```text
features/byok/
  ByokSettingsPanel.svelte
  byokForm.ts
  byokModels.ts
  index.ts
```

Quy ước:

- Component hiển thị không gọi `fetch` trực tiếp nếu đã có API client.
- State UI phức tạp tách thành helper hoặc store trong cùng feature.
- Presenter biến API DTO thành view model đặt trong feature, không nhét vào route.
- Component dùng lại nhiều feature thì chuyển lên `components/workspace` hoặc `components/primitives`.

### `apps/web/src/lib/api`

- Chia API client theo domain: `projects`, `novels`, `analysis`, `byok`, `localRuntime`, `realtime`.
- Type contract sinh từ backend hoặc mirror typed contract, không tạo shape tùy tiện trong component.
- `server/api.ts` không được trở thành file tổng hợp khổng lồ; khi vượt soft limit, tách `server/clients/*.ts`.

## Tài Liệu Và Hướng Dẫn AI

- `AGENTS.md` giữ quy tắc ngắn gọn bắt buộc cho mọi agent.
- `.codex/implementation-rules.md` giữ quy ước kỹ thuật chi tiết hơn.
- `docs/module-architecture.md` là nguồn chính cho cấu trúc module.
- `docs/checklists/11-module-refactor-checklist.md` là kế hoạch chuyển đổi từng bước.
- Tài liệu lớn cũng áp dụng soft limit. Nếu một chủ đề dài, tách thành docs con và tạo index.

## Quy Trình Khi Thêm Tính Năng

1. Xác định domain sở hữu tính năng.
2. Kiểm tra file dự kiến chỉnh có vượt 800 hoặc 1200 dòng không.
3. Nếu dưới soft limit, chỉnh trong module hiện có.
4. Nếu vượt soft limit, tách helper/component/service trước hoặc trong cùng thay đổi.
5. Nếu vượt hard limit, tạo PR/refactor split trước khi thêm feature.
6. Cập nhật checklist hoặc docs nếu thay đổi boundary, schema, route hoặc storage ownership.

## Ưu Tiên Chuyển Đổi Hiện Tại

Các file sau đang là điểm nghẽn kiểm soát codebase:

- `crates/api/src/lib.rs`: tách router, service, extraction orchestration, realtime và helper.
- `crates/storage/src/sqlite.rs`: tách repository theo domain và mapper.
- `crates/core/src/extraction.rs`: tách schema, prompt contract, validation và repair helpers.
- `apps/web/src/routes/projects/[projectId]/reading/+page.svelte`: tách Reading workspace, chapter list, detail panel, highlights và action state.

Không thêm workflow lớn vào các file này trước khi có bước split tương ứng trong checklist chuyển đổi.
