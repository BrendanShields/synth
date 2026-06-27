# ADR-0002: Use process-based extensibility

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md)

## Context

Synth's roadmap includes MCP servers, skills, subagents, custom tools, local model servers, and deterministic workflows. These capabilities should be composable without giving extension code unrestricted authority over the user's workspace.

## Decision

Synth extensions will run out-of-process and communicate with the Rust core through typed protocols. Extension actions must declare capabilities and route privileged requests through Synth policy and audit logging.

## Consequences

- Extensions provide capability, not authority.
- Workspace, shell, network, credential, and git access remain mediated by the trusted runtime.
- Extension identity can be attached to tool calls, approvals, failures, and audit events.
- Protocol and supervision work is required before extension ecosystems can mature.
