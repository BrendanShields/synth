---
spec_id: FS-022
title: Approval-gated pull request creation
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

# FS-022: Approval-gated pull request creation

## 1. Problem statement

The git-write chain is complete and gated: create branch → switch → commit → push (FS-018..FS-021). The final mechanical step of the V1 spec-to-PR loop is opening a pull request. This story adds the last gated mutation — creating a PR via the `gh` CLI — closing the loop so a spec's branch can become a PR through the same approval seam.

PR creation is the most outward-facing action: it publishes to the user's real provider. It is therefore gated by ADR-0012, treated as a remote operation per ADR-0013 (inherits the user's `gh` auth; no credential storage), and — because it cannot be exercised against a real provider hermetically — its execution is verified manually, never auto-run in tests. Requesting a PR validates the title and records a pending approval with the exact command; nothing runs. On approval the core runs `gh pr create` in the jailed workspace; on denial nothing runs.

## 2. Requirements

- R1. The Rust core must expose `request_create_pr(title: String, body: String) -> Result<ApprovalRequest, String>` that validates the title and, if a workspace is open, records a pending approval (reusing the FS-018 store) capturing the exact PR-creation command. It must not run `gh`, create a PR, or perform any network operation.
- R2. The returned `ApprovalRequest` must reuse the FS-018 shape with `action` `create-pr` and a `command` describing the exact effect (e.g. `gh pr create --title "<title>" --body "<body>"`, with the body truncated for display only).
- R3. PR-title validation must be a pure, unit-testable function rejecting empty/whitespace-only titles and titles exceeding a fixed maximum length. The body must be rejected only if it exceeds a fixed maximum length; title and body text are otherwise unrestricted (passed to `gh` as separate arguments, never via a shell).
- R4. `resolve_approval` (the FS-018 command) must, for a pending PR and only when `approved`, run `gh pr create --title <title> --body <body>` in the jailed workspace root with fixed arguments and no shell (title and body as separate arguments), then clear the pending approval. A denial clears it and runs nothing.
- R5. PR creation must surface `gh` failures (not installed, not authenticated, no remote/branch, PR already exists) as a readable `Err` without panicking, and must not leave a partial pending approval. On success the outcome message should include the created PR URL when `gh` returns one.
- R6. The action captured at request time must be exactly what executes on approval (no substitution), consistent with ADR-0012.
- R7. PR creation must not force, must not modify existing PRs, must not store credentials (the user's `gh` auth is used), and must not perform any other mutation.
- R8. Automated tests must not run `gh pr create` against any real provider. Only the pure validation and the captured-command construction and gate lifecycle are unit-tested; execution is verified manually.
- R9. The renderer must provide a control to request a PR (title and body), show the pending request in the FS-018 approval surface with the exact command (clearly an outward-facing network action), and resolve it via the existing Approve/Deny flow. After an approved PR creation it should surface the resulting message/URL.
- R10. This story must not add new Tauri capability permissions, must not change the FS-001 runtime status contract, and must not alter the FS-018 approval-gate semantics or the FS-018..FS-021 actions.

## 3. Acceptance criteria

- AC1. `request_create_pr("Add feature", "Body text")` with a workspace open returns an `ApprovalRequest` with `action: create-pr` and a `command` describing `gh pr create --title "Add feature" --body "..."`, and records a pending approval; no PR is created.
- AC2. `request_create_pr` with an empty/whitespace title returns `Err` and records no pending approval; a body exceeding the maximum length also returns `Err`.
- AC3. `request_create_pr` with no workspace open returns `Err` and records no pending approval.
- AC4. `resolve_approval(id, false)` for a pending PR runs no command and creates no PR.
- AC5. `resolve_approval(id, true)` runs `gh pr create --title <title> --body <body>` (verified manually); a `gh` failure returns a readable `Err` with the pending approval cleared.
- AC6. The captured command equals what executes on approval (title/body passed as separate arguments).
- AC7. The renderer requests a PR, shows the exact command as an outward-facing action, and on approval surfaces the outcome/URL; denying creates nothing.
- AC8. Rust unit coverage verifies title/body validation, the captured-command construction, and the request→pending→resolve(deny) lifecycle. No test runs `gh pr create`. FS-018..FS-021 tests remain intact.
- AC9. No code in this story force-creates/modifies PRs, stores credentials, runs `gh` in tests, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: tests cover pure validation, captured-command construction, and the gate lifecycle (including denial). They must not invoke `gh pr create`. PR-creation execution is verified manually on a throwaway/test repository.

Manual checks:

- On a test repo with a pushed branch and `gh` authenticated, request a PR and confirm the approval surface shows `gh pr create --title "…" --body "…"`.
- Approve and confirm a PR is created (URL surfaced); deny another and confirm none is created.
- Request with an empty title and confirm a calm validation error with no pending approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- A note describing the manual PR-creation verification (on a test repo), since execution is not automated.
- Short note confirming nothing executes at request time, execution is gated and manual-only in tests, no credential storage, and capabilities unchanged.

## 5. Success criteria

- SC1. Opening a PR requires explicit approval; nothing runs at request time.
- SC2. On approval the core runs exactly the captured `gh pr create`; on denial nothing runs.
- SC3. The full mechanical spec-to-PR chain (branch → switch → commit → push → PR) is complete and uniformly gated.
- SC4. No credential storage, force, or new capability is introduced; tests never hit a real provider.
- SC5. The slice stays story-sized and does not add PR editing, review, or provider configuration.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 executions/network at request time; execution only on approval | Rust tests / source | @BrendanShields |
| Validation | empty title / oversize title or body rejected | Rust tests | @BrendanShields |
| Command fidelity | executed command equals captured command | Rust tests / source | @BrendanShields |
| Hermetic tests | 0 `gh pr create` calls in automated tests | Source review | @BrendanShields |
| Scope containment | 0 credential-store/force/PR-edit, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-021 commands/contracts intact; gate semantics unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_create_pr` reusing the FS-018 approval store, with a `create-pr` pending action.
- Pure title/body validation and captured-command construction with unit tests.
- Gated `gh pr create --title <title> --body <body>` execution in `resolve_approval` (manual verification).
- A renderer PR-request control (title + body) reusing the FS-018 approval surface, surfacing the resulting URL.

### Out of scope

- Editing, closing, merging, or reviewing PRs.
- Provider configuration, auth setup, or credential storage/management.
- Generating the PR body from the spec/commits automatically (later spec).
- Selecting base/head explicitly (relies on `gh` defaults for the current branch in this slice).
- Policy classification or an audit/event store.

## 8. Technical design

### Rust/Tauri core

Extend the `approvals` `PendingAction` with a `CreatePr { title, body }` variant and add `request_create_pr`. Add a `git::create_pr(root, title, body) -> Result<String, String>` helper running `gh pr create --title <title> --body <body>` with `current_dir` the workspace root, fixed args, no shell, returning the trimmed stdout (PR URL) on success. `request_create_pr` validates the title (pure `is_valid_pr_title`) and body length, requires an open workspace, and records the pending PR with a display command (body truncated). `resolve_approval` gains a `CreatePr` arm that, on approval, calls `create_pr` and returns the outcome including the URL; denial and unknown-id behaviour are unchanged.

### React renderer

Add a PR-request control (title input + body textarea) that calls `request_create_pr` and shows the returned request in the existing approval overlay, presented as an outward-facing action; Approve/Deny use the existing flow. On approval, surface the outcome message/URL. Keep state transient.

### Styling

Reuse the FS-018 approval surface and workspace controls; add a title input and a small body textarea consistent with the existing controls.

## 9. Impact notes

- Data model impact: extends the in-memory pending-action set with a PR variant; reuses `ApprovalRequest`/`ApprovalOutcome`; nothing persisted.
- Security/privacy impact: most outward-facing action, fully gated (ADR-0012) and treated as a remote op (ADR-0013): inherits the user's `gh` auth, no credential storage, no force, no PR edits. Nothing executes without approval; jailed, fixed-arg, no-shell. Tests never call `gh pr create`. No capability added.
- Observability impact: PR request/approve/deny can be noted in the FS-011 session log.
- Performance impact: network-bound on approval only; otherwise negligible.
- Migration/backward compatibility impact: additive; the gate and all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: creating a PR at request time. Mitigation: request only validates and records; tests assert no execution and lifecycle via denial.
- Risk: tests creating real PRs. Mitigation: tests never invoke `gh pr create`; execution is manual-only.
- Risk: shell injection via title/body. Mitigation: passed to `gh` as separate arguments with no shell; validation bounds length.
- Risk: scope creep into PR editing/merge. Mitigation: creation only.
- Dependency: FS-021 merged (approval gate with four actions, push); the `gh` CLI authenticated for manual verification.

## 11. Open questions

None. This slice gates PR creation via `gh` behind explicit approval, with execution verified manually rather than against a real provider in tests.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-021 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-022-approval-gated-pr`
- Expected implementation PR title: `feat(FS-022): Approval-gated pull request creation`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-022/amendments/`.
