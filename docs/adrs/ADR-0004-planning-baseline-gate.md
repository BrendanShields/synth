# ADR-0004: Gate project-level implementation on a merged planning baseline

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md)

## Context

Agentic coding failures often start before code changes: vague goals, missing acceptance criteria, hidden assumptions, and architecture decisions made implicitly during implementation. Synth's product premise is that planning artifacts are first-class deliverables.

## Decision

For project-level work, Synth will require a reviewed and merged planning baseline before implementation begins. The baseline includes at minimum the PRD and ERD/HLSA, with material decisions recorded as ADRs.

## Consequences

- Project-level implementation is blocked until the planning PR is merged.
- Planning documents become the source of truth for feature specs and later implementation PRs.
- Users may still ask questions or request explanation without creating a spec.
- The harness must classify requests so it can apply the correct gate.
