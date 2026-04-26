# BYOK Security Notes

BYOK means "bring your own key". User API keys are sensitive production secrets. Treat them as passwords.

## Required Rules

- Never store provider API keys in frontend local storage.
- Never send API keys directly from browser to third-party providers.
- Proxy LLM calls through the backend.
- Never log full keys in request logs, traces, prompt runs, error reports, analytics, screenshots, or support exports.
- Show only masked keys after save.
- Support a session-only key mode that is not persisted.
- If keys are persisted, encrypt at rest.
- Scope keys to user and workspace.
- Public/shared projects must not silently spend the owner's key.

## Backend Responsibilities

- Normalize OpenAI-compatible, Anthropic, and local llama.cpp providers behind one interface.
- Apply rate limits and job quotas.
- Track token usage per user, provider, model, project, and job.
- Redact secrets from structured logs.
- Separate prompt traces from secret-bearing config.

## Data Model Suggestions

```text
llm_provider_configs
  id
  user_id
  workspace_id
  provider
  base_url
  model
  encrypted_api_key
  key_fingerprint
  created_at
  updated_at

llm_usage_events
  id
  user_id
  workspace_id
  project_id
  job_id
  provider
  model
  input_tokens
  output_tokens
  estimated_cost
  created_at
```

## First Implementation Choice

Start with session-only BYOK for the web MVP. Add encrypted persistent keys only after the proxy, redaction, and usage logs are tested.

