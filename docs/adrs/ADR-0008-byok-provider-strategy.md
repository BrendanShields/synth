# ADR-0008: Use a bring-your-own-key provider strategy for v1

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md)

## Context

Synth should be local-first and useful across changing model ecosystems. The v1 product should not depend on a hosted Synth account, a single model vendor, or a mature provider marketplace.

## Decision

Synth v1 will use bring-your-own-key provider configuration. The initial provider families are OpenAI-compatible endpoints and Ollama, mediated by a provider abstraction with default model and role override support.

## Consequences

- Users control credentials and provider selection.
- Synth can support planner, builder, adversary, summarizer, and requirements-critic roles without hard-coding a vendor.
- Provider credentials and metadata belong in app-local secure storage, not repo-versioned docs.
- Broad provider marketplace support is deferred until after the v1 provider abstraction is stable.
