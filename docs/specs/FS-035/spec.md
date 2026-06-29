---
spec_id: FS-035
title: Approval-gated command execution
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0014-supervised-command-execution.md
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0010-workspace-jail.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-035: Approval-gated command execution

## 1. Problem statement

The `!` shell command has been blocked since FS-002 because command execution is the highest-risk capability. The product needs it — verification, builds, and tools are commands (PRD §15.3) — and the trust model is now in place to do it safely: the approval gate (ADR-0012), the workspace jail (ADR-0010), and the autonomy guardrails (FS-033). This story makes `!` real under ADR-0014: a command is requested, shown exactly, and runs only on explicit human approval, in the jailed workspace, with captured, bounded output and a timeout.

Command execution is excluded from high-autonomy auto-approval in every mode — it always prompts. This is the supervised tool boundary the rest of the product was built to protect.

## 2. Requirements

- R1. The Rust core must expose `request_run_command(command: String) -> Result<ApprovalRequest, String>` that validates the command (non-empty, within a length cap) and, if a workspace is open, records a pending approval (reusing the FS-018 store) capturing the exact command. It must not execute anything. The returned request's `action` is `run-command` and `command` is the exact command.
- R2. `auto_approves` (FS-033) must return false for `run-command` in every mode; command execution always requires an explicit approval.
- R3. `resolve_approval` must, for a pending command and only when `approved`, run the exact command via the system shell with the working directory set to the jailed workspace root, capturing stdout and stderr, then clear the pending approval. A denial clears it and runs nothing.
- R4. Execution must enforce a wall-clock timeout; on timeout it must return a readable result indicating the timeout and must not hang the app. Output (stdout+stderr) must be truncated to a fixed cap.
- R5. The command result (captured, capped output; exit indication; timeout indication) must be returned in the approval outcome so the renderer can show it. A non-zero exit is a result (shown), not a hard error; a spawn failure returns a readable `Err`.
- R6. The exact command captured at request time must be what executes on approval (no substitution), consistent with ADR-0012.
- R7. Execution must run only in the jailed workspace directory; with no workspace open, requesting returns a readable `Err`. The command is not rewritten or sanitized — the explicit approval is the control.
- R8. The command router must route a non-empty `!` command (`CommandKind::Shell`) to `disposition: handled` with a new `target: command`, carrying the command via the parsed argument, and preserving `requiresApproval: true`. An empty `!` stays a no-op/blocked. The router must no longer report a flat `blocked` for non-empty shell commands.
- R9. The renderer must, on a handled `command` route, call `request_run_command` and resolve it through the existing approval flow (always showing the approval surface for command execution), then display the captured output. Command/output state must be transient renderer state only.
- R10. Tests must only execute trivial, safe, terminating commands (e.g. `echo`) in a temporary directory; no test runs a destructive, networked, or unbounded command.
- R11. This story must not add new Tauri capability permissions (execution uses a core-side child process), must not change the FS-001 runtime status contract, and must not weaken the approval gate (command execution is never auto-approved).

## 3. Acceptance criteria

- AC1. `request_run_command("cargo test")` with a workspace open returns an `ApprovalRequest` with `action: run-command`, `command: cargo test`, `autoApprove: false`, and records a pending approval; nothing runs.
- AC2. `request_run_command("")` (or whitespace) returns `Err` and records no pending approval.
- AC3. `request_run_command` with no workspace open returns `Err` and records no pending approval.
- AC4. `auto_approves("run-command", "high_autonomy")` is false (and false for supervised).
- AC5. `resolve_approval(id, true)` for a pending command runs it in the workspace and returns an outcome containing the captured output; `echo hello` yields output containing `hello`. (Verified by a temp-dir test.)
- AC6. `resolve_approval(id, false)` runs no command.
- AC7. A command exceeding the timeout returns a readable timeout result and does not hang; output is truncated to the cap.
- AC8. Submitting `! echo hi` in the dock produces a handled `command` route, shows the approval surface (always, even in high-autonomy), and on approval displays the output.
- AC9. Rust unit coverage verifies command validation, that `run-command` never auto-approves, and execution of a safe command in a temp dir (output captured; non-zero exit captured; timeout path). Routing of `!` to the `command` target is covered.
- AC10. No test runs a destructive/networked/unbounded command; no code adds a Tauri capability or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: execution tests run only `echo`-class commands (and a fast failing command, and a short sleep for the timeout path) in a temporary directory. No network, no destructive commands.

Manual checks:

- Submit `! echo hi`, approve, and confirm the output shows; deny another and confirm nothing runs.
- In high-autonomy mode, submit `! echo hi` and confirm it **still prompts** (never auto-approves).
- Submit an empty `!` and confirm nothing happens.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the approval surface for a command and the captured output after approval, and confirmation it still prompts in high-autonomy.
- Short note confirming jailed cwd, timeout + output cap, no auto-approval, and unchanged capabilities.

## 5. Success criteria

- SC1. A command can be run from the dock, gated by an explicit approval, in the jailed workspace.
- SC2. Command execution is never auto-approved, in any mode.
- SC3. Output is captured and bounded; a hung command cannot freeze the app (timeout).
- SC4. The executed command equals the captured command; nothing runs at request time.
- SC5. The slice stays story-sized and does not add allow-lists, streaming output, or remembered approvals.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 executions at request time; only on approval; never auto-approved | Rust tests / source | @BrendanShields |
| Jail + bounds | jailed cwd; output capped; timeout enforced | Rust tests / source | @BrendanShields |
| Result fidelity | executed command equals captured; output returned | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 auto-approvals, 0 new capabilities, no destructive test commands | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-034 intact; only `!` changes from blocked to handled | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_run_command` reusing the FS-018 store, with a `run-command` pending action excluded from auto-approval.
- A jailed, captured, capped, timeout-bounded command runner.
- Routing non-empty `!` to a `command` target.
- A renderer command flow reusing the approval surface and showing captured output.

### Out of scope

- Command allow-lists, per-command risk policy, or remembered approvals.
- Streaming/live command output (captured-on-completion only).
- Interactive commands (TTY), background processes, or process management.
- Running commands outside the workspace, or environment manipulation.
- Process-based extension brokering (ADR-0002; a later spec builds on this seam).

## 8. Technical design

### Rust/Tauri core

Add an `exec` module: `run_command(root, command, timeout, cap) -> Result<String, String>` that runs `sh -c <command>` (the system shell) with `current_dir` the workspace root, capturing stdout+stderr, enforcing a wall-clock timeout (via a worker thread + `recv_timeout` so the UI thread never blocks), and truncating output to `cap`. A non-zero exit returns Ok with the output (and an exit indicator); a spawn failure returns `Err`; a timeout returns a readable timeout string.

Extend the `approvals` `PendingAction` with `RunCommand(String)` and add `request_run_command` (validates the command, requires an open workspace, records the pending action; `auto_approves("run-command", _)` is false). `resolve_approval` gains a `RunCommand` arm that, on approval, calls `run_command` and returns the captured output in the outcome message.

### Command router

Add `RouteTarget::Command` (`command`). A non-empty `Shell` command returns `handled` + `command` (carrying the command via the parsed argument), preserving `requiresApproval: true`; empty `!` is not handled. Update the FS-003 shell-blocked expectation accordingly.

### React renderer

On a handled `command` route, call `request_run_command(argument)` and resolve through the existing approval overlay (which always shows for `run-command`, since `autoApprove` is false). On approval, display the captured output (in the approval notice or a command-output surface). Keep state transient.

### Styling

Reuse the approval surface and a calm preformatted output block.

## 9. Impact notes

- Data model impact: extends the pending-action set with a command variant; reuses `ApprovalRequest`/`ApprovalOutcome`; nothing persisted (the run can be noted in the event log).
- Security/privacy impact: the highest-risk capability, fully gated (ADR-0014): explicit approval always, never auto-approved, jailed cwd, captured+capped output, timeout, exact-command fidelity. No capability added (core-side child process).
- Observability impact: command request/approve/deny and the run are recorded/persisted (FS-011/FS-032).
- Performance impact: bounded by the timeout; output capped.
- Migration/backward compatibility impact: `!` changes from blocked to handled (gated); all other routes unchanged.

## 10. Risks and dependencies

- Risk: a dangerous command. Mitigation: explicit human approval showing the exact command, always (never auto-approved); jailed cwd; the human is the control (ADR-0014).
- Risk: a hung command freezing the app. Mitigation: a wall-clock timeout via a worker thread; the UI thread never blocks.
- Risk: runaway output. Mitigation: a fixed output cap.
- Risk: command substitution. Mitigation: the exact command is captured and executed unchanged.
- Dependency: FS-018 gate, FS-033 auto-approval classifier, FS-012 workspace jail.

## 11. Open questions

None. This slice runs a command gated, jailed, captured, and bounded; allow-lists, streaming, and extension brokering are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-034 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-035-command-execution`
- Expected implementation PR title: `feat(FS-035): Approval-gated command execution`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-035/amendments/`.
