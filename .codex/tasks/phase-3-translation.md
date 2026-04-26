# Phase 3 - Parallel Translation

Goal: support story translation in parallel with AI analysis without weakening source-grounded analysis.

## Scope

- Source segmentation shared by analysis and translation.
- Translation jobs.
- Translation segment persistence.
- Glossary and style guide model.
- Translation review items.
- Side-by-side reading UI.
- Agentic translation jobs that can run without per-segment human approval.
- Inline correction flow for translation segments and glossary terms.

## Rules

- Source text remains authoritative.
- Translation output is a versioned projection.
- Translated text must not replace source evidence.
- Glossary changes should be tracked and may trigger retranslation.
- Translation provider usage must be tracked separately from analysis usage.
- Review items should not block the whole translation pipeline.
- User-edited translation segments must not be overwritten unless force rerun is explicit.
- Prompt templates and UI copy must come from registries, not feature-code literals.

## First Slice

- Add source segment model.
- Add translation job model.
- Add translation segment model.
- Add glossary entry model.
- Add API contract draft.
- Add side-by-side reading UI placeholder.
- Add stale marker behavior for glossary, alias, source text, and translation edits.
