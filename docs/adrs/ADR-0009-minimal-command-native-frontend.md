# ADR-0009: Build a minimal command-native frontend

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md)

## Context

Synth is a planning and execution harness, not a full IDE replacement or dashboard cockpit. The product should keep attention on one artifact, the current workflow state, and the next command or approval.

## Decision

Synth will use a minimal command-native interface: one focused central artifact, a small contextual status, a bottom command/input dock, and overlays only when context demands them.

## Consequences

- Reader, session, diff, render, approval, and amendment views are modes of a focused shell rather than permanent panels.
- The renderer remains thin and subscribes to typed runtime events from the Rust core.
- UI feature specs should prefer keyboard-native flows and low-noise artifact review.
- Full visual workflow composition remains a roadmap feature, not a v1 shell requirement.
