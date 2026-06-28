---
spec_id: FS-021
title: Approval-gated push
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0011-git-via-cli-readonly-first.md
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0013-remote-git-operations.md
  - docs/adrs/ADR-0010-workspace-jail.md
---

# FS-021: Approval-gated push

## 1. Problem statement

The local branch workflow (create → switch → commit) is complete and gated (FS-018/FS-019/FS-020). The V1 loop produces a PR per spec, which requires publishing the branch to a remote. This story adds the first network mutation — pushing the current branch to a configured remote — gated by the same approval seam (ADR-0012) and restricted to configured remotes with no arbitrary URLs and no force (ADR-0013).

Requesting a push validates the remote name and records a pending approval with the exact command; nothing runs. On approval the core runs `git push -u <remote> HEAD` in the jailed workspace; on denial nothing runs. Tests exercise push against a local bare remote only and never contact a network remote.

## 2. Requirements

- R1. The Rust core must expose `request_push(remote: String) -> Result<ApprovalRequest, String>` that validates the remote name (defaulting an empty input to `origin`) and, if a workspace is open, records a pending approval (reusing the FS-018 store) and returns it. It must not run git or perform any network operation.
- R2. The returned `ApprovalRequest` must reuse the FS-018 shape with `action` `push` and `command` `git push -u <remote> HEAD`.
- R3. Remote-name validation must be a pure, unit-testable function accepting a safe identifier (`A-Za-z0-9._-`, non-empty, not beginning with `-`) and rejecting anything else (whitespace, slashes, URLs, `..`, control characters). An empty/whitespace input must be normalized to `origin` before validation.
- R4. `resolve_approval` (the FS-018 command) must, for a pending push and only when `approved`, run `git push -u <remote> HEAD` in the jailed workspace root with fixed arguments and no shell (the remote as a separate argument), then clear the pending approval. A denial clears it and runs nothing.
- R5. Push execution must surface git failures (no such remote, rejected, auth failure, no upstream) as a readable `Err` without panicking, and must not leave a partial pending approval.
- R6. The action captured at request time must be exactly what executes on approval (no substitution), consistent with ADR-0012.
- R7. Push must target a configured remote by name only — never an arbitrary URL — must not force-push or delete remote refs, and must not perform any other mutation. No credentials are stored; the user's existing git credential configuration is used.
- R8. The renderer must provide a control to request a push (optional remote name, default `origin`), show the pending request in the FS-018 approval surface with the exact command (clearly a network action), and resolve it via the existing Approve/Deny flow. After an approved push, it must refresh git status.
- R9. Automated tests must exercise push against a local bare repository only and must not contact any network remote.
- R10. This story must not add new Tauri capability permissions, must not change the FS-001 runtime status contract, and must not alter the FS-018 approval-gate semantics or the FS-018/FS-019/FS-020 actions.

## 3. Acceptance criteria

- AC1. `request_push("origin")` (or `request_push("")` → `origin`) with a workspace open returns an `ApprovalRequest` with `action: push` and `command: git push -u origin HEAD`, and records a pending approval; nothing is pushed.
- AC2. `request_push` with an invalid remote name (whitespace, a URL, `-x`, `a/b`) returns `Err` and records no pending approval.
- AC3. `request_push` with no workspace open returns `Err` and records no pending approval.
- AC4. `resolve_approval(id, true)` for a pending push runs `git push -u <remote> HEAD`; against a local bare remote the branch appears on the remote afterward.
- AC5. `resolve_approval(id, false)` runs no git command and pushes nothing.
- AC6. Pushing with no such remote (or other git failure) returns a readable `Err` on approval, with the pending approval cleared.
- AC7. The renderer requests a push, shows the exact command as a network action, and on approval reflects success; denying pushes nothing.
- AC8. Rust unit coverage verifies remote-name validation (including default-to-origin), that a push pending action captures the exact command, and push execution against a local bare remote (branch present on the remote afterward; bad remote errors). No test contacts a network remote. FS-018/FS-019/FS-020 tests remain intact.
- AC9. No code in this story force-pushes, deletes remote refs, accepts arbitrary URLs, stores credentials, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: push execution is tested by creating a temporary working repo and a temporary **bare** repo, adding the bare repo as a remote, committing, running the gated push, and asserting the branch exists in the bare remote. No network remote is contacted.

Manual checks:

- In a repo with an `origin` remote on a throwaway/test project, request a push and confirm the approval surface shows `git push -u origin HEAD` as a network action.
- Approve and confirm the branch is pushed; deny another and confirm nothing is pushed.
- Request with an invalid remote name and confirm a calm validation error with no pending approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Note/screenshot of the local-bare-remote push test result.
- Short note confirming configured-remote-only (no URLs), no force, no credential storage, nothing executes at request time, and capabilities unchanged.

## 5. Success criteria

- SC1. Publishing a branch requires explicit approval; nothing runs at request time.
- SC2. On approval the core runs exactly the captured `git push -u <remote> HEAD`; on denial nothing runs.
- SC3. Push targets configured remotes by name only — no arbitrary URLs, no force.
- SC4. Tests verify push hermetically against a local bare remote, never a network remote.
- SC5. The slice stays story-sized and does not add PR creation or fetch/pull.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 executions/network at request time; execution only on approval | Rust tests / source | @BrendanShields |
| Remote validation | invalid remotes/URLs rejected; empty → origin | Rust tests | @BrendanShields |
| Push correctness | approved push lands the branch on a local bare remote | Rust tests | @BrendanShields |
| Hermetic tests | 0 network remotes contacted in tests | Source review | @BrendanShields |
| Scope containment | 0 force/url/credential-store, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-020 commands/contracts intact; gate semantics unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_push` reusing the FS-018 approval store, with a `push` pending action and a default-to-origin remote.
- Pure remote-name validation with unit tests.
- Gated `git push -u <remote> HEAD` execution in `resolve_approval`, tested against a local bare remote.
- A renderer push-request control reusing the FS-018 approval surface.

### Out of scope

- PR creation (the next spec) and any GitHub/provider API call.
- Force-push, deleting remote refs, or rewriting published history.
- Fetch, pull, or any inbound sync.
- Arbitrary remote URLs, adding/removing remotes, or credential storage/management.
- Policy classification, risk levels, or an audit/event store.

## 8. Technical design

### Rust/Tauri core

Extend the `approvals` `PendingAction` with a `Push(remote)` variant and add `request_push`. Add a `git::push(root, remote)` helper running `git push -u <remote> HEAD` with `current_dir` the workspace root, fixed args, no shell. `request_push` normalizes an empty remote to `origin`, validates it (pure `is_valid_remote_name`), requires an open workspace, and records the pending push. `resolve_approval` gains a `Push` arm that, on approval, calls `push` and returns the outcome; denial and unknown-id behaviour are unchanged.

### React renderer

Add a push-request control (optional remote input, default `origin`) that calls `request_push` and shows the returned request in the existing approval overlay, presented as a network action; Approve/Deny use the existing flow. After an approved push, refresh git status. Keep state transient.

### Styling

Reuse the FS-018 approval surface and workspace controls; the push control mirrors the others. The approval surface may indicate a network action with a small, restrained label.

## 9. Impact notes

- Data model impact: extends the in-memory pending-action set with a push variant; reuses `ApprovalRequest`/`ApprovalOutcome`; nothing persisted.
- Security/privacy impact: first network mutation, fully gated (ADR-0012, ADR-0013). Configured-remote-only, no URLs, no force, no credential storage; the user's git credentials are used by the CLI. Nothing executes without approval; jailed, fixed-arg, no-shell. No capability added.
- Observability impact: push request/approve/deny can be noted in the FS-011 session log.
- Performance impact: network-bound on approval only; otherwise negligible.
- Migration/backward compatibility impact: additive; the gate and all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: pushing at request time. Mitigation: request only validates and records; tests assert nothing is pushed after request.
- Risk: pushing to an unintended destination. Mitigation: configured-remote-by-name only, validated, no URLs; `HEAD` pushes the current branch.
- Risk: destructive remote change. Mitigation: no force, no ref deletion; out of scope.
- Risk: tests hitting a real network. Mitigation: tests use a local bare remote only.
- Dependency: FS-020 merged (approval gate with three actions, git helpers, workspace root); a configured remote for manual verification.

## 11. Open questions

None. This slice gates pushing the current branch to a configured remote behind explicit approval, tested hermetically.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-020 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-021-approval-gated-push`
- Expected implementation PR title: `feat(FS-021): Approval-gated push`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-021/amendments/`.
