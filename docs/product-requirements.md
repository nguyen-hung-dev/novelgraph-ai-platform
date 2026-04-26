# Product Requirements

## Product Summary

NovelGraph AI Platform is a desktop-style workspace for analyzing long-form fiction with AI. It should work as a hosted website and as a local desktop app.

## Target Users

- Novel readers who want structured summaries and maps.
- Writers managing complex worlds and character relationships.
- Researchers and reviewers exploring narrative structure.
- Fan communities building reference material.
- Users who prefer to bring their own LLM API key.

## Core Jobs

- Import a novel.
- Split it into chapters.
- Run AI analysis with progress and retry controls.
- Read chapters with entity highlighting.
- Explore character relationships.
- Explore world/location structure.
- Explore timeline and scenes.
- Ask grounded questions about the novel.
- Translate chapters while preserving glossary, style, and source alignment.
- Review uncertain AI outputs.
- Review uncertain translation terms and segments.
- Export project data.

## MVP Scope

The MVP should prove the foundation:

- Web workspace shell.
- Project/bookshelf.
- TXT/Markdown import.
- Chapter splitting.
- BYOK settings.
- Durable analysis job model.
- First evidence-first extraction schema.
- Reading view.
- Basic review queue.
- Translation architecture and glossary model.

## Non-MVP

- Full production billing.
- Public marketplace.
- Full map renderer parity.
- Full multi-language translation quality suite.
- Full document export suite.
- Mobile-first layout.
- Enterprise organization controls.

## Product Constraints

- The interface should feel like a tool, not a landing page.
- The same visual language should work on web and desktop.
- User secrets and private source text must be protected.
- LLM outputs must be traceable to source evidence where possible.
