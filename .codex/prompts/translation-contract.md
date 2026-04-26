# Translation Contract

This prompt contract is for future translation jobs.

## Inputs

- Source language.
- Target language.
- Source segment text.
- Approved glossary entries.
- Style guide.
- Relevant entity memory.
- Previous local context when allowed.

## Output Shape

```json
{
  "target_text": "...",
  "glossary_used": [
    {
      "source_term": "...",
      "target_term": "..."
    }
  ],
  "uncertain_terms": [
    {
      "source_term": "...",
      "reason": "..."
    }
  ],
  "warnings": []
}
```

## Requirements

- Preserve all meaning from the source segment.
- Do not add new plot facts.
- Keep approved glossary translations unchanged.
- Preserve markdown structure where possible.
- Report uncertain terms instead of guessing silently.
- Do not translate names that the style guide says to preserve.

