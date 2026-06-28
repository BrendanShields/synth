# ADR-0010: Confine all workspace file access to an explicitly opened, jailed root

**Status:** Accepted
**Created:** 2026-06-28
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/adrs/ADR-0001-rust-native-runtime.md`](ADR-0001-rust-native-runtime.md)

## Context

Synth's V1 promise is to operate on an existing repository. Until now the app has been workspace-free: no filesystem access beyond the localhost provider calls. Opening a real repository crosses a trust boundary the rest of the product is organized around (PRD §20, ERD §4): the model proposes, the trusted runtime enforces, and out-of-workspace access must be gated.

Before any file reads, writes, or git operations exist, the product needs a single, well-defined notion of *where work is allowed to happen* so that every later filesystem capability can be confined to it rather than retrofitted.

## Decision

Synth introduces a **workspace jail**: a single workspace is opened explicitly by the user, and the trusted Rust core records its canonicalized absolute path as the workspace root. Every filesystem operation Synth performs on workspace content must resolve to a path inside that root; paths that escape it (via `..`, symlinks, or absolute paths) are rejected by the core.

- The workspace root is established only by an explicit user open action, never inferred or persisted across sessions without an equally explicit step.
- Path confinement is enforced in the trusted core by canonicalizing candidate paths and verifying the root is a prefix — not in the renderer.
- The first slice establishes the root and the confinement primitive; reads, writes, and git operations are added by later specs and must route through the confinement check.
- Opening a workspace is read-only with respect to its contents until a later spec adds scoped read access.

## Consequences

- There is one authoritative answer to "what may Synth touch on disk", enforced centrally.
- Later filesystem and git specs inherit the jail instead of each re-deciding scope.
- Out-of-workspace access remains impossible by construction until a policy/approval layer (Phase 2) deliberately allows specific exceptions.
- The renderer never decides filesystem scope; it only displays the opened workspace and requests actions the core validates.
