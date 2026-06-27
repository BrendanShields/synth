# ADR-0006: Use story-sized immutable feature specs

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/templates/feature-spec.md`](../templates/feature-spec.md)

## Context

Large, vague implementation requests encourage silent scope drift. Synth needs an implementation contract small enough to review, test, and trace to a single PR.

## Decision

Feature specs are the unit of implementation. Each feature spec must be story-sized and include problem statement, requirements, acceptance criteria, tests/verification, success criteria, and metrics. Approved specs are immutable.

## Consequences

- Larger initiatives must be decomposed into releases/epics and then story-sized specs.
- Each spec gets its own branch, verification evidence, logical commits, review, and PR.
- Missing mandatory sections block implementation.
- Material in-flight changes require amendments instead of editing the approved spec in place.
