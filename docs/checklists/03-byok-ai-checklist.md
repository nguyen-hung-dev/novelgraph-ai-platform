# Checklist Phase 3 - BYOK Và AI Provider

## Provider Abstraction

- [ ] Tạo trait provider chung.
  - [ ] `generate`.
  - [ ] `generate_stream`.
  - [ ] `validate_key`.
  - [ ] `estimate_cost`.
- [ ] Chuẩn hóa provider error.
- [ ] Chuẩn hóa usage tokens.
- [ ] Chuẩn hóa model metadata.

## Web BYOK

- [ ] Thiết kế session-only key flow.
  - [ ] Nhập key.
  - [ ] Validate key.
  - [ ] Lưu trong session an toàn.
  - [ ] Xóa key khỏi session.
- [ ] Thiết kế persistent key flow sau.
  - [ ] Encrypt at rest.
  - [ ] Masked display.
  - [ ] Key fingerprint.
  - [ ] Rotate key.
- [ ] Không lưu key trong browser local storage.
- [ ] Không trả key về frontend sau khi lưu.

## Provider Clients

- [ ] OpenAI-compatible client.
  - [ ] Base URL tùy chỉnh.
  - [ ] Chat completions.
  - [ ] Streaming.
  - [ ] JSON/schema mode nếu provider hỗ trợ.
- [ ] Anthropic client.
  - [ ] Messages API.
  - [ ] Streaming.
  - [ ] Token usage.
- [ ] llama.cpp client.
  - [ ] OpenAI-compatible local endpoint.
  - [ ] Health check.
  - [ ] Model metadata.

## Logging Và Bảo Mật

- [ ] Redact `Authorization`.
- [ ] Redact `x-api-key`.
- [ ] Redact request body chứa key.
- [ ] Redact provider error nếu có header.
- [ ] Thêm tests cho redaction.
- [ ] Không đưa key vào prompt trace.

## Usage Accounting

- [ ] Lưu `provider`.
- [ ] Lưu `model`.
- [ ] Lưu `input_tokens`.
- [ ] Lưu `output_tokens`.
- [ ] Lưu `estimated_cost`.
- [ ] Gắn usage với `user_id`.
- [ ] Gắn usage với `project_id`.
- [ ] Gắn usage với `job_id`.

