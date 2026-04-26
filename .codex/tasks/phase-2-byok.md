# Phase 2 - BYOK Provider Layer

Goal: safely support user-provided LLM API keys on the hosted web path.

## Scope

- Provider abstraction.
- OpenAI-compatible provider client.
- Anthropic provider client.
- llama.cpp local provider client.
- Session-only BYOK flow.
- Masked key display.
- Usage accounting.
- Redaction tests.

## Security Requirements

- API keys never go to third-party providers directly from browser code.
- API keys never appear in logs.
- Prompt traces never contain auth headers.
- Error messages must not expose secrets.
- Public/shared projects cannot spend another user's key implicitly.

## Non-Goals

- No persistent encrypted key storage until session-only flow and redaction are tested.
- No billing integration.

