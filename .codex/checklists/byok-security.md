# BYOK Security Checklist

- [ ] API keys are never stored in frontend local storage.
- [ ] API keys are sent only to backend over HTTPS.
- [ ] Backend redacts keys from logs.
- [ ] Provider auth headers are not stored in traces.
- [ ] UI displays only masked keys.
- [ ] Session-only key mode exists.
- [ ] Persistent keys are encrypted at rest before launch.
- [ ] Usage is tracked per user/project/provider/model.
- [ ] Public share mode cannot spend owner's key without explicit permission.
- [ ] Error responses do not include provider request headers.
- [ ] CI has a basic secret scan.

