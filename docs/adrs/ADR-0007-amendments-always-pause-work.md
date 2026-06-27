# ADR-0007: Always pause for amendment approval

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/templates/amendment.md`](../templates/amendment.md)

## Context

Implementation can reveal missing requirements, ambiguous acceptance criteria, hidden dependencies, security constraints, or user preference changes. Continuing without explicit approval turns those discoveries into silent scope drift.

## Decision

When implementation detects a material deviation from an approved spec, Synth pauses and creates an amendment. Amendment approval is always blocking in both supervised and high-autonomy modes.

## Consequences

- High-autonomy mode cannot bypass spec-change approval.
- Amendments record the changed clause, reason, impact, approval decision, and deviation telemetry.
- Task/check plans can update only after the amendment is approved.
- Amendment telemetry becomes an input to better future planning and adversarial review.
