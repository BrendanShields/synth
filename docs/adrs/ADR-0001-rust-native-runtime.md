# ADR-0001: Use a Rust-native trusted runtime

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md)

## Context

Synth is a local-first desktop harness that performs privileged operations against real repositories, local files, shell commands, credentials, provider calls, and git state. The product requires a trusted kernel that can enforce policy independently from the model and the renderer.

## Decision

Synth will use a Rust-native Tauri core as the trusted product runtime. The React renderer remains a thin visual surface. Models, tools, extensions, and UI actions request work through the Rust core rather than performing privileged operations directly.

## Consequences

- Runtime policy, filesystem access, command execution, git automation, provider streaming, approvals, and audit logging are owned by Rust/Tauri.
- The renderer can be redesigned without moving trust enforcement into UI code.
- Feature specs that touch privileged behavior must define Rust-side interfaces, policy checks, and verification evidence.
- Implementation complexity is higher than a renderer-only app, but the trust boundary is explicit and durable.
