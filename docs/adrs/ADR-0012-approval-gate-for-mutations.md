# ADR-0012: Gate every mutating action behind an explicit approval

**Status:** Accepted
**Created:** 2026-06-29
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/adrs/ADR-0010-workspace-jail.md`](ADR-0010-workspace-jail.md), [`docs/adrs/ADR-0011-git-via-cli-readonly-first.md`](ADR-0011-git-via-cli-readonly-first.md)

## Context

Everything Synth has done so far is read-only. The V1 loop requires mutation — creating branches, committing, opening PRs — and the PRD is explicit that the model proposes and the trusted runtime enforces (PRD §4.6, §20): file writes, command execution, and out-of-workspace operations must route through policy and approval boundaries, and high autonomy reduces routine prompts but never bypasses trust enforcement.

Before any mutating capability exists, Synth needs one consistent way to gate mutations so that each new mutating capability inherits the gate rather than inventing its own.

## Decision

Synth introduces an **approval gate** in the trusted core. A mutating action is split into two steps:

1. **Request.** The renderer (on behalf of the user or model) asks the core to perform a mutating action. The core validates the request, records a pending, typed `ApprovalRequest` describing exactly what would run, and returns it. **Nothing privileged executes at this step.**
2. **Resolve.** The user explicitly approves or denies the pending request. The core executes the action **only** on approval, then clears the pending request. A denial discards it with no effect.

Constraints:

- Mutations never execute as a side effect of the request step; execution happens only in the resolve step after explicit approval.
- The exact command/effect is captured in the request and is what executes on approval — no substitution between request and resolve.
- Inputs that feed a mutation (e.g. a branch name) are validated by pure, testable rules before a request is created, so malformed or dangerous inputs never reach execution.
- Approval state lives in the trusted core, not the renderer; the renderer can only display pending requests and relay the user's decision.
- This is the seam the future policy engine, audit log, and high-autonomy "remembered decisions" plug into; for now every mutating action requires an explicit decision.

## Consequences

- Every mutating capability (branch, commit, push, file write) goes through one gate instead of bespoke prompts.
- A mutation can never happen without an explicit, logged user decision, satisfying the PRD's trust-enforcement requirement.
- The request/resolve split gives a natural place to later attach policy classification, risk levels, remembered approvals, and audit events without changing call sites.
- The renderer stays unprivileged: it proposes and relays, the core decides and executes.
