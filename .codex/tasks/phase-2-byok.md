# Phase 2 - AI Provider Layer

Goal: support local llama.cpp first, then safely support user-provided LLM API keys on the hosted web path.

## Scope

- Provider abstraction.
- llama.cpp local provider client.
- OpenAI-compatible provider client.
- Anthropic provider client.
- llama.cpp local provider client.
- Session-only BYOK flow.
- Masked key display.
- Usage accounting.
- Redaction tests.

## Local-First Priority

- Local llama.cpp health check.
- Local llama.cpp model list.
- Local llama.cpp chat completions.
- Local draft chapter extraction for prompt evaluation.
- No API key required for local llama.cpp.
- Do not couple local LLM calls to browser local storage.

## Security Requirements

- API keys never go to third-party providers directly from browser code.
- API keys never appear in logs.
- Prompt traces never contain auth headers.
- Error messages must not expose secrets.
- Public/shared projects cannot spend another user's key implicitly.

## Non-Goals

- No persistent encrypted key storage until session-only flow and redaction are tested.
- No billing integration.
