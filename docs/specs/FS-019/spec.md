---
spec_id: FS-019
title: Approval-gated commit
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

# FS-019: Approval-gated commit

## 1. Problem statement

FS-018 established the approval gate and used it for branch creation. The spec-to-PR loop also needs to commit work. This story adds a second gated mutation — committing the workspace's changes — to prove the gate generalizes beyond a single action and to advance the git-write surface toward logical commits (PRD §18.3).

Committing reuses the ADR-0012 seam exactly: requesting a commit validates the message and records a pending approval with the exact commands; nothing runs. On approval the core stages all changes and commits with the message, in the jailed workspace. On denial nothing runs. No push or network occurs.

## 2. Requirements

- R1. The Rust core must expose `request_commit(message: String) -> Result<ApprovalRequest, String>` that validates the message and, if a workspace is open, records a pending approval (reusing the FS-018 approval store) and returns it. It must not run git or mutate anything.
- R2. The returned `ApprovalRequest` must reuse the FS-018 shape and set `action` to `commit` and `command` to a human-readable description of the exact effect (e.g. `git add -A && git commit -m "<message>"`).
- R3. Commit-message validation must be a pure, unit-testable function rejecting empty/whitespace-only messages and messages exceeding a fixed maximum length. Message text is otherwise unrestricted (it is passed to git as a separate argument, never via a shell).
- R4. `resolve_approval` (the FS-018 command) must, for a pending commit and only when `approved`, stage all changes and commit with the message in the jailed workspace root using fixed-argument git invocations with no shell (`git add -A`, then `git commit -m <message>`), then clear the pending approval. A denial clears it and runs nothing.
- R5. The commit execution must surface git failures (e.g. nothing to commit, no identity configured) as a readable `Err` without panicking, and must not leave a partial pending approval.
- R6. The exact action captured at request time must be what executes on approval (no substitution), consistent with ADR-0012.
- R7. Committing must not push, fetch, or perform any network operation, must not create branches or switch branches, and must not write files other than through the git commit itself.
- R8. The renderer must provide a control to request a commit (entering a message), show the pending request in the FS-018 approval surface with the exact effect, and resolve it via the existing Approve/Deny flow. After an approved commit, it must refresh git status so the now-clean tree is reflected.
- R9. This story must not add new Tauri capability permissions, must not change the FS-001 runtime status contract, and must not alter the FS-018 approval-gate semantics (request never executes; resolve executes only on approval).

## 3. Acceptance criteria

- AC1. `request_commit("docs: update")` with a workspace open returns an `ApprovalRequest` with `action: commit` and a `command` describing `git add -A && git commit -m "docs: update"`, and records a pending approval; nothing is committed yet.
- AC2. `request_commit` with an empty/whitespace message returns `Err` and records no pending approval.
- AC3. `request_commit` with no workspace open returns `Err` and records no pending approval.
- AC4. `resolve_approval(id, true)` for a pending commit stages all changes and commits with the message; afterward the working tree is clean (observable via FS-016) and a new commit exists (observable via FS-017).
- AC5. `resolve_approval(id, false)` runs no git command and leaves the working tree unchanged.
- AC6. When there is nothing to commit, approval returns a readable `Err`, and the pending approval is cleared.
- AC7. The renderer requests a commit, shows the exact effect in the approval surface, and on approval reflects the clean tree.
- AC8. Rust unit coverage verifies message validation, that a commit pending action captures the exact message, and the stage-and-commit execution against a temporary git repository (clean tree afterward, new commit present). The FS-018 branch tests remain intact.
- AC9. No code in this story pushes, performs network operations, creates/switches branches, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: commit execution is tested by `git init`-ing a temporary repository with a configured identity, writing a file, running the gated commit, and asserting a clean tree and a new commit. No network operations.

Manual checks:

- Open the Synth repo, make a change, request a commit, and confirm the approval surface shows the exact effect.
- Approve and confirm (via FS-016/FS-017) the tree is clean and the commit appears; deny another and confirm the tree is unchanged.
- Request with an empty message and confirm a calm validation error with no pending approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the approval surface for a commit and confirmation of an approved commit (clean tree) and a denied no-op.
- Short note confirming nothing executes at request time, only stage+commit executes on approval, no push/network, and capabilities are unchanged.

## 5. Success criteria

- SC1. Committing requires an explicit approval; nothing runs at request time.
- SC2. On approval the core stages and commits exactly as captured; on denial nothing runs.
- SC3. The approval gate is shown to generalize across two distinct mutations (branch, commit).
- SC4. No push, network, branch mutation, or new capability is introduced.
- SC5. The slice stays story-sized and does not add push or PR creation.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 executions at request time; execution only on approval | Rust tests / source | @BrendanShields |
| Message validation | empty/oversize rejected before request | Rust tests | @BrendanShields |
| Commit correctness | approved commit yields clean tree + new commit | Rust tests (temp repo) | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 push/network, 0 branch mutation, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-018 commands/contracts intact; gate semantics unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_commit` reusing the FS-018 approval store, with a `commit` pending action.
- Pure commit-message validation with unit tests.
- Gated stage-and-commit execution (`git add -A`, `git commit -m`) in `resolve_approval`, tested against a temp repo.
- A renderer commit-request control reusing the FS-018 approval surface.

### Out of scope

- Push, fetch, or any remote/network operation.
- Branch creation/switching (FS-018 only) or selective staging of specific paths.
- Commit amending, signing configuration, or hooks management.
- PR creation, policy classification, or an audit/event store.

## 8. Technical design

### Rust/Tauri core

Extend the `approvals` module's pending action with a `Commit(message)` variant and add `request_commit`. Add a `git::commit_all(root, message)` helper that runs `git add -A` then `git commit -m <message>` with `current_dir` the workspace root, fixed args, no shell. `request_commit` validates the message (pure `is_valid_commit_message`), requires an open workspace, and records the pending commit, returning an `ApprovalRequest` with `action: commit`. `resolve_approval` gains a match arm for `Commit` that, on approval, calls `commit_all` and returns the outcome; denial and unknown-id behaviour are unchanged.

### React renderer

Add a commit-message control (input) that calls `request_commit` and shows the returned request in the existing approval overlay; Approve/Deny use the existing `resolve_approval` flow. After an approved commit, refresh git status. Keep state transient.

### Styling

Reuse the FS-018 approval surface and workspace control styles; add only a message input consistent with the branch control.

## 9. Impact notes

- Data model impact: extends the in-memory pending-action set with a commit variant; no new IPC shape (reuses `ApprovalRequest`/`ApprovalOutcome`); nothing persisted.
- Security/privacy impact: second gated mutation; nothing executes without approval; fixed-arg, no-shell, jailed git; message passed as a separate argument. No push/network, no capability added.
- Observability impact: commit request/approve/deny can be noted in the FS-011 session log; durable audit log is future.
- Performance impact: negligible; git add+commit only on approval.
- Migration/backward compatibility impact: additive; FS-018 gate and all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: committing at request time. Mitigation: request only validates and records; tests assert no commit after request.
- Risk: partial state on failure (staged but not committed). Mitigation: surface git errors; `git add -A` then `git commit` is the standard sequence, and a failed commit leaves a recoverable staged state reported to the user; selective unstaging is out of scope.
- Risk: scope creep into push/PR. Mitigation: only stage+commit; push and PR are later specs.
- Dependency: FS-018 merged (approval gate, `git::create_branch` pattern, workspace root).

## 11. Open questions

None. This slice gates committing all changes behind explicit approval, reusing the FS-018 gate.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-018 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-019-approval-gated-commit`
- Expected implementation PR title: `feat(FS-019): Approval-gated commit`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-019/amendments/`.
