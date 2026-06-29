---
spec_id: FS-033
title: High-autonomy auto-approval
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0007-amendments-always-pause-work.md
  - docs/adrs/ADR-0001-rust-native-runtime.md
---

# FS-033: High-autonomy auto-approval

## 1. Problem statement

FS-026 made autonomy mode visible but explicitly did not change approval behaviour. The PRD's high-autonomy mode reduces routine interruption after plan approval (PRD §13.1) — but never bypasses trust boundaries (§13.2): out-of-workspace access, destructive actions, credential access, network access, PR creation, and amendment approval always require an explicit decision. This story implements that reduction safely: in high-autonomy mode, **only low-risk, local, non-destructive** mutations auto-approve; everything else stays gated exactly as in supervised mode.

The trusted core decides whether an action auto-approves, based on the mode and the action's risk class. The action still flows through the FS-018 gate (request → resolve) and is still logged; high autonomy only removes the prompt for the safe subset.

## 2. Requirements

- R1. The Rust core must compute, for each approval request, whether it auto-approves, via a pure, unit-testable function `auto_approves(action, mode) -> bool`.
- R2. `auto_approves` must return true **only** when the mode is `high_autonomy` **and** the action is in the low-risk local set: `create-branch`, `switch-branch`, `commit`, `save-spec`. It must return false for `push`, `create-pr` (network), and `save-amendment` (amendments always pause, ADR-0007), and always false in `supervised` mode.
- R3. Each `request_*` command must read the current autonomy mode and set an `autoApprove` boolean on the returned `ApprovalRequest` from `auto_approves(action, mode)`. The request still records the pending action exactly as today (no execution at request time).
- R4. `ApprovalRequest` must serialize in camelCase including the existing fields plus `autoApprove`. Adding the field must not break existing request behaviour or the resolve flow.
- R5. The renderer must, on receiving a request with `autoApprove: true`, immediately resolve it as approved (`resolve_approval(id, true)`) without showing the approval prompt; on `autoApprove: false` it must show the approval surface and require an explicit decision as today.
- R6. Auto-approval must not change the execution path: the action executes only via `resolve_approval` (on approval), the captured command is unchanged, and the event is recorded/persisted as for any approval.
- R7. In supervised mode (default), every action must still require an explicit approval — no behaviour change from today.
- R8. This story must not auto-approve any networked, destructive, out-of-workspace, credential, PR, or amendment action in any mode; must not add new Tauri capability permissions; and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `auto_approves("create-branch", "high_autonomy")` is true; `auto_approves("commit", "high_autonomy")` is true; `auto_approves("save-spec", "high_autonomy")` is true; `auto_approves("switch-branch", "high_autonomy")` is true.
- AC2. `auto_approves("push", "high_autonomy")`, `auto_approves("create-pr", "high_autonomy")`, and `auto_approves("save-amendment", "high_autonomy")` are all false.
- AC3. `auto_approves(<any action>, "supervised")` is false.
- AC4. In high-autonomy mode, requesting a branch returns `autoApprove: true` and the renderer resolves it without a prompt; the branch is created and the event recorded.
- AC5. In high-autonomy mode, requesting a push or a PR returns `autoApprove: false` and the renderer shows the approval surface requiring an explicit decision.
- AC6. In supervised mode, every request returns `autoApprove: false` and shows the approval surface.
- AC7. The executed command equals the captured command in all cases; nothing executes at request time.
- AC8. Rust unit coverage verifies `auto_approves` for each action × mode and that `ApprovalRequest` serializes `autoApprove` in camelCase. Existing approval/gate tests remain intact.
- AC9. No code in this story auto-approves a network/destructive/amendment/PR action, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks:

- In supervised mode, confirm every mutation still prompts.
- Switch to high-autonomy; confirm create-branch / switch / commit / save-spec proceed without a prompt (and appear in the event log), while push / open-PR / save-amendment still prompt.
- Confirm the executed command matches and nothing runs at request time.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- A note/screenshot showing a low-risk action auto-approved in high-autonomy and a networked action still prompting.
- Short note confirming network/amendment/PR/destructive actions never auto-approve and capabilities are unchanged.

## 5. Success criteria

- SC1. High-autonomy mode reduces prompts for low-risk local mutations only.
- SC2. Network, destructive, PR, and amendment actions always require explicit approval in every mode (§13.2, ADR-0007).
- SC3. Supervised mode behaviour is unchanged.
- SC4. The execution path and audit/event recording are unchanged; only the prompt is skipped.
- SC5. The slice stays story-sized and does not add per-action risk configuration or remembered approvals.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Auto-approval scope | only low-risk local actions in high-autonomy | Rust tests | @BrendanShields |
| Guardrails | push/PR/amendment/destructive never auto-approve | Rust tests | @BrendanShields |
| Supervised unchanged | all actions prompt in supervised | Rust tests / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 networked auto-approvals, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-032 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A pure `auto_approves(action, mode)` classifier.
- An `autoApprove` flag on `ApprovalRequest`, set by each `request_*` from the current mode.
- Renderer auto-resolution of `autoApprove` requests without a prompt.
- Rust unit tests for the classifier and serialization.

### Out of scope

- Per-action or per-path risk configuration / policy rules.
- Remembered approvals ("always allow X").
- Auto-approving any networked, destructive, out-of-workspace, credential, PR, or amendment action.
- Changing the execution or audit path.

## 8. Technical design

### Rust/Tauri core

Add `auto_approves(action: &str, mode: &str) -> bool` to the `approvals` module:

```text
mode == "high_autonomy" && action in { "create-branch", "switch-branch", "commit", "save-spec" }
```

Add `auto_approve: bool` to `ApprovalRequest` (camelCase `autoApprove`). Each `request_*` command gains an `AutonomyState` parameter, reads the mode, and sets `auto_approve = auto_approves(<its action>, &mode)` on the returned request. The recording of the pending action is unchanged. `resolve_approval` is unchanged. Because `request_*` already returns `ApprovalRequest`, only the constructor/return is extended.

### React renderer

After each `request_*` call, if the returned request `autoApprove` is true, immediately call `resolve_approval(id, true)` and surface the outcome (no overlay); otherwise show the approval overlay as today. Centralize this in the existing request-handling path so all request types share it.

### Styling

No new styling.

## 9. Impact notes

- Data model impact: adds `autoApprove` to `ApprovalRequest`; no persisted entities.
- Security/privacy impact: high autonomy skips the prompt only for low-risk local mutations; network/destructive/PR/amendment actions remain explicitly gated in every mode (§13.2, ADR-0007). The core decides; the renderer relays. No capability added.
- Observability impact: auto-approved actions are recorded/persisted like any approval (FS-011/FS-032).
- Performance impact: negligible.
- Migration/backward compatibility impact: additive; supervised mode unchanged; the new field is additive.

## 10. Risks and dependencies

- Risk: auto-approving something dangerous. Mitigation: a strict allow-list of four local, non-destructive actions; network/destructive/PR/amendment excluded and tested.
- Risk: bypassing the gate. Mitigation: execution still happens only via `resolve_approval`; auto-approval calls it with explicit `approved: true` and is logged.
- Risk: supervised regressions. Mitigation: `auto_approves` returns false for supervised; tests assert it.
- Dependency: FS-018 gate, FS-026 autonomy mode, the request_* commands.

## 11. Open questions

None. This slice reduces prompts for a strict low-risk local subset in high-autonomy, preserving every §13.2 boundary.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-032 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-033-auto-approval`
- Expected implementation PR title: `feat(FS-033): High-autonomy auto-approval`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-033/amendments/`.
