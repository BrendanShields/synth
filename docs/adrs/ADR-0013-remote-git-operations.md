# ADR-0013: Remote git operations are network actions, gated and restricted to configured remotes

**Status:** Accepted
**Created:** 2026-06-29
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/adrs/ADR-0011-git-via-cli-readonly-first.md`](ADR-0011-git-via-cli-readonly-first.md), [`docs/adrs/ADR-0012-approval-gate-for-mutations.md`](ADR-0012-approval-gate-for-mutations.md)

## Context

The V1 loop produces a pull request per spec, which requires pushing branches to a remote. Push (and later fetch) are different in kind from local mutations: they reach the network, touch the user's real remotes and credentials, and are hard to reverse. The PRD requires that network access route through approval and policy (PRD §13.2, §20), and ADR-0012 already established the approval gate for mutations.

## Decision

Remote git operations are treated as **network mutations**: gated by the ADR-0012 approval gate, and further restricted so they cannot reach an unintended destination.

- **Configured remotes only.** Remote operations target a remote already configured in the repository (e.g. `origin`); Synth does not accept or push to arbitrary URLs. The remote name is validated to a safe identifier and passed as a separate argument with no shell.
- **Gated like any mutation.** A push is requested (validated, recorded as a pending approval with the exact command) and executes only on explicit approval, per ADR-0012.
- **Inherit the user's git credentials.** Remote auth uses the user's existing git/credential configuration (consistent with ADR-0011's CLI choice); Synth does not store or manage credentials in this phase.
- **No force, no destructive remote ops by default.** The initial remote surface is a normal push of the current branch; force-push, deleting remote refs, and rewriting published history are out of scope and would be separate, explicitly-scoped decisions.
- **Testing never touches a real remote.** Automated tests exercise push against throwaway *local bare* repositories only; they must not contact any network remote.

## Consequences

- Synth can publish branches toward a PR while every push remains an explicit, approved, logged action against a known remote.
- Credentials stay in the user's environment; Synth inherits rather than manages them until a later credential/policy phase.
- The "configured remotes only, no arbitrary URLs, no force" constraints bound the blast radius of the first network capability until the Phase 2 policy engine can govern it generally.
- Tests stay hermetic and offline by using local bare remotes, so verifying push never mutates a real remote.
