# Evidence Grounding Rules

Evidence is the boundary between useful AI analysis and unsupported hallucination.

## Rules

- A quote must exist in the source chapter text.
- Evidence should be short enough for UI display.
- Do not cite summaries as source evidence.
- Prior memory can guide disambiguation but cannot be cited as current-chapter evidence.
- If a fact is inferred from multiple spans, include all relevant spans.
- If evidence cannot be found, mark the fact as inferred and send it to review.

## Validation Ideas

- Check quote substring exists in chapter text.
- Check spans are within chapter bounds.
- Check span text matches quote after whitespace normalization.
- Check current extraction does not cite future chapters.

