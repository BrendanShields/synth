# ADR-0011: Integrate git through the system CLI, read-only first, confined to the workspace

**Status:** Accepted
**Created:** 2026-06-29
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/adrs/ADR-0010-workspace-jail.md`](ADR-0010-workspace-jail.md)

## Context

Synth's V1 promise centres on git: branches, commits, and PRs are first-class product artifacts (PRD §18). Before any of that, Synth needs to *observe* the repository's git state. Two implementation paths exist: bind a library (libgit2) or shell out to the system `git` CLI. The CLI matches what users already have configured (credentials, hooks, signing, includes), avoids a heavy native dependency, and produces output Synth can parse. It also means Synth runs an external process — a capability the product otherwise gates.

## Decision

Synth integrates git by invoking the **system `git` CLI** as a child process, under strict constraints:

- **Read-only first.** The initial git surface is observation only (`status`, `rev-parse`, `branch --show-current`, `log`). Mutating operations (branch, commit, push, PR) are separate, later, explicitly-scoped specs.
- **Confined to the workspace.** Git is always run with its working directory set to the opened, canonicalized workspace root (ADR-0010). Synth does not run git outside an opened workspace.
- **Fixed argument sets, never user-interpolated.** Each git command uses a hard-coded, read-only argument list. No shell is invoked (`Command` with explicit args, not a shell string), and no user- or model-supplied text is passed as git arguments in the read-only phase.
- **Failures are data, not crashes.** A folder that is not a git repository, or a missing `git` binary, is reported as a typed state, not a panic.

## Consequences

- Synth inherits the user's real git configuration and credentials without re-implementing git.
- The git surface grows from observation to mutation deliberately, each step its own reviewed spec, so write/push operations never arrive by accident.
- Running an external process is introduced narrowly (one known binary, fixed read-only args, jailed cwd, no shell), keeping it analyzable until the Phase 2 policy/approval engine can govern command execution generally.
- Parsing git output is isolated in pure, testable functions so behaviour does not depend on a live repository.
