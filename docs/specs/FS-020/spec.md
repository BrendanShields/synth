---
spec_id: FS-020
title: Approval-gated branch switch
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

# FS-020: Approval-gated branch switch

## 1. Problem statement

FS-018 and FS-019 gate branch creation and commit. The spec-to-PR loop implements each spec on its own branch, which means moving onto a branch to work on it. This story adds the third gated mutation — switching to an existing branch — completing the *local* branch workflow (create → switch → commit), all behind the ADR-0012 approval gate, before any remote/network operation is introduced.

Switching reuses the gate exactly: requesting a switch validates the branch name and records a pending approval with the exact command; nothing runs. On approval the core runs `git switch <name>` in the jailed workspace; on denial nothing runs. No branch is created and no network operation occurs.

## 2. Requirements

- R1. The Rust core must expose `request_switch_branch(name: String) -> Result<ApprovalRequest, String>` that validates the branch name (reusing FS-018 `is_valid_branch_name`) and, if a workspace is open, records a pending approval (reusing the FS-018 store) and returns it. It must not run git or mutate anything.
- R2. The returned `ApprovalRequest` must reuse the FS-018 shape with `action` `switch-branch` and `command` `git switch <name>`.
- R3. `resolve_approval` (the FS-018 command) must, for a pending switch and only when `approved`, run `git switch <name>` in the jailed workspace root with fixed arguments and no shell (the name as a separate argument), then clear the pending approval. A denial clears it and runs nothing.
- R4. Switch execution must surface git failures (e.g. the branch does not exist, or the switch is blocked by uncommitted changes) as a readable `Err` without panicking, and must not leave a partial pending approval.
- R5. The action captured at request time must be exactly what executes on approval (no substitution), consistent with ADR-0012.
- R6. Switching must not create a branch, must not commit, must not push/fetch or perform any network operation, and must not write files other than the working-tree update inherent to `git switch`.
- R7. The renderer must provide a control to request a switch (entering an existing branch name), show the pending request in the FS-018 approval surface with the exact command, and resolve it via the existing Approve/Deny flow. After an approved switch, it must refresh git status so the new current branch is reflected.
- R8. This story must not add new Tauri capability permissions, must not change the FS-001 runtime status contract, and must not alter the FS-018 approval-gate semantics or the FS-018/FS-019 actions.

## 3. Acceptance criteria

- AC1. `request_switch_branch("feature/x")` with a workspace open returns an `ApprovalRequest` with `action: switch-branch` and `command: git switch feature/x`, and records a pending approval; the current branch is unchanged.
- AC2. `request_switch_branch` with an invalid name returns `Err` and records no pending approval.
- AC3. `request_switch_branch` with no workspace open returns `Err` and records no pending approval.
- AC4. `resolve_approval(id, true)` for a pending switch runs `git switch <name>`; afterward the current branch is `<name>` (observable via FS-016).
- AC5. `resolve_approval(id, false)` runs no git command and leaves the current branch unchanged.
- AC6. Switching to a non-existent branch returns a readable `Err` on approval, with the pending approval cleared.
- AC7. The renderer requests a switch, shows the exact command, and on approval reflects the new current branch; denying changes nothing.
- AC8. Rust unit coverage verifies that a switch pending action captures the exact command, and the switch execution against a temporary git repository (current branch changes on success; non-existent branch errors). The FS-018/FS-019 tests remain intact.
- AC9. No code in this story creates a branch, commits, pushes, performs network operations, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: switch execution is tested by `git init`-ing a temporary repository with a commit, creating a branch, running the gated switch, and asserting the current branch changed; a non-existent branch is asserted to error. No network operations.

Manual checks:

- Open the Synth repo, create a branch (FS-018), then request a switch to it and confirm the approval surface shows `git switch <name>`.
- Approve and confirm (via FS-016) the current branch changed; deny another and confirm no change.
- Request a switch to a non-existent branch and confirm a calm error on approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the approval surface for a switch and confirmation of an approved switch (branch changed) and a denied no-op.
- Short note confirming nothing executes at request time, only `git switch` executes on approval, no creation/commit/push/network, and capabilities are unchanged.

## 5. Success criteria

- SC1. Switching branches requires explicit approval; nothing runs at request time.
- SC2. On approval the core runs exactly the captured `git switch <name>`; on denial nothing runs.
- SC3. The local branch workflow (create → switch → commit) is complete and fully gated.
- SC4. No creation, commit, push, network, or new capability is introduced.
- SC5. The slice stays story-sized and does not add push or PR creation.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 executions at request time; execution only on approval | Rust tests / source | @BrendanShields |
| Command fidelity | executed command equals captured command | Rust tests / source | @BrendanShields |
| Switch correctness | approved switch changes current branch; bad branch errors | Rust tests (temp repo) | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 creation/commit/push/network, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-019 commands/contracts intact; gate semantics unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_switch_branch` reusing the FS-018 approval store, with a `switch-branch` pending action.
- Gated `git switch <name>` execution in `resolve_approval`, tested against a temp repo.
- A renderer switch-request control reusing the FS-018 approval surface.

### Out of scope

- Creating a branch while switching (`-c`/`-b`); only switching to an existing branch.
- Commit, push, fetch, or any remote/network operation.
- Stashing, discarding, or force-switching past uncommitted changes.
- PR creation, policy classification, or an audit/event store.

## 8. Technical design

### Rust/Tauri core

Extend the `approvals` `PendingAction` with a `SwitchBranch(name)` variant and add `request_switch_branch`. Add a `git::switch_branch(root, name)` helper running `git switch <name>` with `current_dir` the workspace root, fixed args, no shell. `request_switch_branch` validates the name (reusing `is_valid_branch_name`), requires an open workspace, and records the pending switch. `resolve_approval` gains a `SwitchBranch` arm that, on approval, calls `switch_branch` and returns the outcome; denial and unknown-id behaviour are unchanged.

### React renderer

Add a switch-request control (branch-name input) that calls `request_switch_branch` and shows the returned request in the existing approval overlay; Approve/Deny use the existing flow. After an approved switch, refresh git status. Keep state transient.

### Styling

Reuse the FS-018/FS-019 workspace controls and approval surface; add only a switch input consistent with the others.

## 9. Impact notes

- Data model impact: extends the in-memory pending-action set with a switch variant; reuses `ApprovalRequest`/`ApprovalOutcome`; nothing persisted.
- Security/privacy impact: third gated mutation; nothing executes without approval; fixed-arg, no-shell, jailed git; name validated and passed as a separate argument. No creation/commit/push/network, no capability added.
- Observability impact: switch request/approve/deny can be noted in the FS-011 session log.
- Performance impact: negligible; git switch only on approval.
- Migration/backward compatibility impact: additive; the gate and all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: switching at request time. Mitigation: request only validates and records; tests assert the branch is unchanged after request.
- Risk: data loss from switching over uncommitted changes. Mitigation: plain `git switch` refuses to overwrite conflicting local changes and reports an error; force/stash is out of scope.
- Risk: scope creep into create-and-switch or push. Mitigation: switch to an existing branch only.
- Dependency: FS-019 merged (approval gate with two actions, git helpers, workspace root).

## 11. Open questions

None. This slice gates switching to an existing branch behind explicit approval, completing the local branch workflow.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-019 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-020-approval-gated-switch`
- Expected implementation PR title: `feat(FS-020): Approval-gated branch switch`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-020/amendments/`.
