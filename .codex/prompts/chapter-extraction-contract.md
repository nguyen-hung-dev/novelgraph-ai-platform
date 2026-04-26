# Chapter Extraction Contract

This is the intended contract for future extraction prompts and schemas.

## Principles

- Extract only from the current chapter and allowed prior context.
- Do not use future chapters as evidence.
- Every fact should include evidence spans from source text.
- Uncertain facts must include confidence and review reason.
- Output must be valid structured JSON.

## Required Output Groups

- characters
- locations
- organizations
- items
- concepts
- relationships
- events
- spatial_relations
- review_items

## Evidence Span Shape

```json
{
  "chapter_num": 1,
  "start_char": 128,
  "end_char": 196,
  "quote": "short source quote",
  "reason": "why this supports the fact"
}
```

## Observation Shape

```json
{
  "subject": "Han Li",
  "predicate": "appears_in",
  "object": {
    "chapter_num": 1,
    "location": "Yellow Maple Valley"
  },
  "confidence": 0.92,
  "evidence": []
}
```

## Review Rule

Create a review item when:

- confidence is low;
- entity identity is ambiguous;
- relation direction is uncertain;
- location hierarchy is inferred but not explicit;
- output depends on prior memory rather than current chapter evidence.

