---
spec_id: FS-018
title: Approval-gated branch creation
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0001-rust-native-runtime.md
  - docs/adrs/ADR-0011-git-via-cli-readonly-first.md
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0010-workspace-jail.md
---

# FS-018: Approval-gated branch creation

## 1. Problem statement

Synth can observe a repository (FS-016/FS-017) but has never mutated one. The V1 loop implements each spec on its own branch, so the first mutation Synth needs is creating a branch. Per the PRD (§4.6, §20) and ADR-0012, no mutation may execute without an explicit approval.

This story introduces the approval gate and uses it for exactly one low-risk mutation: creating a git branch. Requesting a branch validates the name and records a pending approval describing the exact git command; nothing runs. The user then approves or denies; only on approval does the core run `git branch <name>` in the jailed workspace. This establishes the request → approve → execute seam that every later mutation (commit, push, PR, file write) will reuse.

## 2. Requirements

- R1. The Rust core must own an approval module with a pending-approval store in managed state, holding the exact action to perform.
- R2. The core must expose `request_create_branch(name: String) -> Result<ApprovalRequest, String>` that validates the branch name and, if a workspace is open, records a pending approval and returns it. It must not run git or mutate anything.
- R3. `ApprovalRequest` must serialize in camelCase with at least: `id` (number), `action` (e.g. `create-branch`), `summary` (human-readable), and `command` (the exact git command that would run, e.g. `git branch fix/x`).
- R4. Branch-name validation must be a pure, unit-testable function rejecting empty names, names with whitespace, names beginning with `-`, names containing `..`, control characters, or characters outside a safe set (`A-Za-z0-9._/-`), and names with a leading or trailing `/`.
- R5. The core must expose `resolve_approval(id: number, approved: boolean) -> Result<ApprovalOutcome, String>` that looks up the pending approval, and only when `approved` is true executes the stored action (`git branch <name>` with `current_dir` the workspace root, fixed args, no shell, the name passed as a separate argument), then removes the pending approval. A denial removes the pending approval and runs nothing.
- R6. `ApprovalOutcome` must serialize in camelCase with at least: `id`, `approved` (boolean), and `message`.
- R7. Resolving an unknown or already-resolved `id` must return a readable `Err`. Requesting with no open workspace must return a readable `Err` and create no pending approval.
- R8. A mutation must never execute at request time, must execute at resolve time only when approved, and the executed command must be exactly the one captured in the request (no substitution).
- R9. Git branch creation must use the read-only-safe invocation discipline of ADR-0011 except that `branch` is a mutating subcommand: fixed argument array, no shell, the validated name as a separate argument, jailed `current_dir`. No push or network operation may occur.
- R10. The renderer must show a pending `ApprovalRequest` in a clear approval surface (overlay or panel) displaying the action summary and exact command, with explicit Approve and Deny controls, and must call `resolve_approval` with the user's decision. It must provide a control to request a branch (entering a name). Approval state shown in the renderer is transient.
- R11. This story must not add any other mutating action, must not push or perform network operations, must not write files other than via the gated `git branch`, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. `request_create_branch("feature/x")` with a workspace open returns an `ApprovalRequest` with `action: create-branch`, a `command` of `git branch feature/x`, and creates a pending approval; no branch exists yet.
- AC2. `request_create_branch` with an invalid name (empty, `has space`, `-rf`, `a..b`, `feature/`, or a control character) returns `Err` and creates no pending approval.
- AC3. `request_create_branch` with no workspace open returns `Err` and creates no pending approval.
- AC4. `resolve_approval(id, true)` runs `git branch <name>` in the workspace and returns `approved: true`; afterward the branch exists (observable via FS-016 status on a real repo).
- AC5. `resolve_approval(id, false)` runs no git command, returns `approved: false`, and removes the pending approval; no branch is created.
- AC6. `resolve_approval` with an unknown id returns `Err`; resolving the same id twice returns `Err` on the second call.
- AC7. The renderer shows the pending request with its exact command and Approve/Deny controls, and reflects the outcome; denying leaves the repository unchanged.
- AC8. Rust unit coverage verifies branch-name validation (valid and each invalid class), the request→pending→resolve lifecycle (approve executes, deny/​unknown do not), that no execution happens at request time, and camelCase serialization. Branch execution is covered against a temporary git repository created by the test.
- AC9. No code in this story pushes, performs network operations, adds another mutating action, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: branch execution is tested by `git init`-ing a temporary repository (with an initial commit), running the gated creation, and asserting the branch exists via git. No network operations.

Manual checks:

- Open the Synth repo, request a branch, and confirm the approval surface shows the exact `git branch <name>` command.
- Approve and confirm (via the FS-016 status) the new branch exists; deny another request and confirm no branch is created.
- Attempt an invalid name and confirm a calm validation error with no pending approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the approval surface showing the exact command, and confirmation of an approved branch and a denied no-op.
- Short note confirming nothing executes at request time, only `git branch` executes on approval, no push/network, and capabilities are unchanged.

## 5. Success criteria

- SC1. Creating a branch requires an explicit approval; nothing runs at request time.
- SC2. On approval the core runs exactly the captured `git branch <name>`; on denial nothing runs.
- SC3. Branch names are validated by pure rules before any request exists.
- SC4. No push, network, other mutation, or new capability is introduced.
- SC5. The request→approve→execute seam is reusable by later mutations and the slice stays story-sized.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 executions at request time; execution only on approval | Rust tests / source | @BrendanShields |
| Input validation | every invalid name class rejected before request | Rust tests | @BrendanShields |
| Command fidelity | executed command equals captured command | Rust tests / source | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 push/network, 0 other mutations, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-017 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An approval module with a pending-approval store and `ApprovalRequest`/`ApprovalOutcome` shapes.
- `request_create_branch` and `resolve_approval` commands.
- Pure branch-name validation with unit tests.
- Gated `git branch <name>` execution (jailed, fixed args, no shell), tested against a temp repo.
- A renderer approval surface (summary + exact command + Approve/Deny) and a request-branch control.

### Out of scope

- Any other mutation: commit, checkout/switch, merge, rebase, tag, push, or PR creation.
- Remote or network operations of any kind.
- File writes outside the gated `git branch`.
- Policy classification, risk levels, remembered approvals, or an audit/event store (future, on this seam).
- Switching to the created branch or changing the working tree.

## 8. Technical design

### Rust/Tauri core

Add an `approvals` module and a gated git helper.

```text
ApprovalRequest { id, action, summary, command }   // serde camelCase
ApprovalOutcome { id, approved, message }           // serde camelCase
is_valid_branch_name(name: &str) -> bool            // pure
request_create_branch(approvals, workspace, name) -> Result<ApprovalRequest, String>
resolve_approval(approvals, workspace, id, approved) -> Result<ApprovalOutcome, String>
```

`ApprovalState` holds `Mutex<(next_id, HashMap<id, PendingAction>)>` where `PendingAction` captures the action (e.g. `CreateBranch(name)`) and the exact command string. `request_create_branch` validates the name (Err on invalid), requires an open workspace (Err otherwise), allocates an id, stores the pending action, and returns the request — running nothing. `resolve_approval` removes the pending action by id (Err if absent); if `approved`, it runs the captured action via a `git::create_branch(root, name)` helper (`git branch <name>`, jailed, fixed args, no shell) and returns the outcome; if denied, it returns an `approved: false` outcome and runs nothing. `git::create_branch` reuses ADR-0011 invocation discipline.

### React renderer

Add a control to request a branch (name input). On request, store the returned `ApprovalRequest` and show an approval surface (overlay/panel) with the action summary and exact command and Approve/Deny buttons. Approve/Deny call `resolve_approval`; render the outcome and clear the pending request. Refresh git status after an approved creation so the new branch is reflected. Keep all state transient. The FS-011 session log may note request/approve/deny.

### Styling

Reuse quiet surfaces; the approval surface may be a calm centered panel with the command in mono. No alarming colors beyond a restrained emphasis; Approve/Deny clearly distinct and accessible.

## 9. Impact notes

- Data model impact: introduces `ApprovalRequest`/`ApprovalOutcome` IPC shapes and an in-memory pending-approval store; nothing persisted.
- Security/privacy impact: first mutation, fully gated (ADR-0012). Nothing executes without explicit approval; the executed command equals the captured one; branch name validated; jailed, fixed-arg, no-shell git; no push/network. No capability added.
- Observability impact: request/approve/deny can be noted in the FS-011 session log; durable audit log is future.
- Performance impact: negligible; one git invocation only on approval.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: a mutation slipping in at request time. Mitigation: request only validates and stores; tests assert no branch exists after request.
- Risk: argument injection via the branch name. Mitigation: pure validation rejecting `-`-leading/whitespace/`..`/unsafe chars, plus passing the name as a separate argument with no shell.
- Risk: command substitution between request and resolve. Mitigation: the exact command/action is captured at request and executed unchanged.
- Risk: scope creep into commit/push. Mitigation: only `git branch`; everything else is out of scope.
- Dependency: FS-017 merged (git module, workspace root, ADR-0011 discipline).

## 11. Open questions

None. This slice gates exactly one low-risk mutation (branch creation) behind explicit approval, establishing the reusable gate.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-017 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-018-approval-gated-branch`
- Expected implementation PR title: `feat(FS-018): Approval-gated branch creation`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-018/amendments/`.
