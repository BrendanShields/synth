# ADR-0014: Command execution is always gated, jailed, captured, and bounded

**Status:** Accepted
**Created:** 2026-06-29
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/adrs/ADR-0010-workspace-jail.md`](ADR-0010-workspace-jail.md), [`docs/adrs/ADR-0012-approval-gate-for-mutations.md`](ADR-0012-approval-gate-for-mutations.md)

## Context

The `!` shell command kind has been deliberately blocked since FS-002 because running commands is the highest-risk capability (PRD §20, §15.3). The product needs it — verification, builds, and tools are commands — but it must be the most carefully gated action in the system. The model proposes a command; only an explicit human decision may run it.

## Decision

Synth executes commands as a **supervised tool boundary** under strict, non-negotiable constraints:

- **Always gated, never auto-approved.** A command is requested (recorded as a pending approval showing the exact command) and runs only on an explicit human approval (ADR-0012). Command execution is **excluded from high-autonomy auto-approval (FS-033) in every mode** — it always prompts.
- **Jailed working directory.** Commands run with their working directory set to the opened, canonicalized workspace root (ADR-0010). Synth does not run commands without an open workspace.
- **Captured and bounded output.** stdout/stderr are captured and truncated to a fixed cap; the command runs under a wall-clock timeout so a hung or runaway command cannot freeze the app.
- **Exact command preserved.** The command that executes is exactly the one captured at request time (no substitution).
- **The user's own shell/environment.** Commands run via the system shell in the user's environment (matching what they would run themselves); Synth does not sanitize or rewrite the command — the human approval is the control.
- **Tests never run an unbounded or destructive command.** Automated tests exercise only trivial, safe, terminating commands (e.g. `echo`) in a temporary directory.

## Consequences

- Synth can run verification and tooling, but every command is an explicit, logged, bounded, jailed, human-approved action.
- High autonomy never reduces the prompt for command execution — the riskiest action keeps the strongest control.
- A timeout and output cap bound the blast radius of a bad command (hang, runaway output).
- This is the seam later policy (allow-lists, per-command risk, remembered approvals) and the process-based extension broker (ADR-0002) build on.
