---
spec_id: FS-025
title: Approval-gated spec save
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0010-workspace-jail.md
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0006-story-sized-immutable-feature-specs.md
---

# FS-025: Approval-gated spec save

## 1. Problem statement

FS-024 drafts a feature spec with the model, but the draft is transient — Synth cannot yet persist it into the repository. The spec-to-PR loop stores specs at `docs/specs/<spec-id>/spec.md` (ADR-0006). This story adds the ability to save a spec into the open workspace, as the first **workspace file write**, confined to the jail (ADR-0010) and gated by approval (ADR-0012).

Saving reuses the approval gate: requesting a save validates the spec id and content and records a pending approval with the exact target path; nothing is written. On approval the core writes the content to `docs/specs/<spec-id>/spec.md` inside the workspace; on denial nothing is written. The write is confined by `is_within_root`, refuses to escape the workspace, and creates only the spec's own directory.

## 2. Requirements

- R1. The Rust core must expose `request_save_spec(specId: String, content: String) -> Result<ApprovalRequest, String>` that validates the spec id and content and, if a workspace is open, records a pending approval (reusing the FS-018 store) capturing the exact target path. It must not write anything.
- R2. The returned `ApprovalRequest` must reuse the FS-018 shape with `action` `save-spec` and `command` describing the write (e.g. `write docs/specs/FS-025/spec.md`).
- R3. Spec-id validation must be a pure, unit-testable function accepting the canonical `FS-<digits>` form (case-insensitive, normalized to uppercase) and rejecting anything else. Content must be rejected if empty/whitespace-only or exceeding a fixed maximum length.
- R4. The target path must be `docs/specs/<spec-id>/spec.md` relative to the workspace root, and must be confined with `is_within_root` before any write; a path that escapes the root must error and never be written.
- R5. `resolve_approval` (the FS-018 command) must, for a pending save and only when `approved`, create the spec directory if needed and write the content to the target path within the jail, then clear the pending approval. A denial clears it and writes nothing.
- R6. The write must create only the spec's directory and the single `spec.md` file; it must not write outside `docs/specs/<spec-id>/`, must not delete or modify unrelated files, must not perform git or network operations, and must not run a shell.
- R7. Write failures (e.g. permission denied) must return a readable `Err` without panicking, and must not leave a partial pending approval.
- R8. The action captured at request time must be exactly what executes on approval (no substitution), consistent with ADR-0012.
- R9. The renderer must let the user save a spec (a spec-id input and the content, e.g. from the FS-024 draft), show the pending request in the FS-018 approval surface with the exact target path, and resolve it via the existing Approve/Deny flow. After an approved save, it should refresh the workspace specs list (FS-015). Save state shown in the renderer is transient.
- R10. This story must not add new Tauri capability permissions (the write uses core-side `std::fs`), must not overwrite-protect or version specs beyond a plain write, and must not change the FS-001 runtime status contract or the FS-018 approval-gate semantics.

## 3. Acceptance criteria

- AC1. `request_save_spec("FS-025", "<content>")` with a workspace open returns an `ApprovalRequest` with `action: save-spec` and a `command` referencing `docs/specs/FS-025/spec.md`, and records a pending approval; no file is written.
- AC2. `request_save_spec` with an invalid spec id (`notaspec`, `FS-`, `../x`) or empty content returns `Err` and records no pending approval.
- AC3. `request_save_spec` with no workspace open returns `Err` and records no pending approval.
- AC4. `resolve_approval(id, true)` for a pending save writes the content to `docs/specs/<spec-id>/spec.md` within the workspace; the file exists with the content afterward (observable via FS-014/FS-015).
- AC5. `resolve_approval(id, false)` writes nothing.
- AC6. A path that would escape the workspace is rejected and never written.
- AC7. The renderer saves a spec, shows the exact target path, and on approval reflects the new spec in the workspace specs list; denying writes nothing.
- AC8. Rust unit coverage verifies spec-id and content validation, that the confined write writes the file inside a temporary workspace (and refuses an escaping path), and the request→pending→resolve lifecycle (approve writes, deny does not). FS-018..FS-023 tests remain intact.
- AC9. No code in this story writes outside `docs/specs/<spec-id>/`, deletes files, performs git/network operations, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the confined write is tested against a temporary workspace (writing a spec file and asserting its contents; asserting an escaping path is refused). No network operations.

Manual checks:

- Draft a spec (FS-024), enter a spec id, save it, and confirm the approval surface shows `write docs/specs/<id>/spec.md`.
- Approve and confirm the file exists and appears in the workspace specs list; deny another and confirm nothing is written.
- Try an invalid spec id and confirm a calm validation error with no pending approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the approval surface for a save and confirmation of an approved write (file present) and a denied no-op.
- Short note confirming the write is confined to `docs/specs/<id>/`, nothing executes at request time, and capabilities are unchanged.

## 5. Success criteria

- SC1. A spec can be saved into the workspace at `docs/specs/<id>/spec.md`, gated by approval.
- SC2. The write is confined to the jail and to the spec's own directory.
- SC3. On approval the core writes exactly the captured target; on denial nothing is written.
- SC4. No write outside the spec directory, no git/network, and no new capability are introduced.
- SC5. The slice stays story-sized and does not approve, branch, commit, or enforce immutability.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 writes at request time; write only on approval | Rust tests / source | @BrendanShields |
| Validation | invalid spec id / empty content rejected | Rust tests | @BrendanShields |
| Confinement | write inside docs/specs/<id>/ only; escape refused | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 writes outside the spec dir, 0 git/network, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-024 commands/contracts intact; gate semantics unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_save_spec` reusing the FS-018 approval store, with a `save-spec` pending action.
- Pure spec-id and content validation with unit tests.
- A confined `std::fs` write to `docs/specs/<id>/spec.md` in `resolve_approval`, tested against a temp workspace.
- A renderer save control (spec id + content) reusing the FS-018 approval surface, refreshing the specs list after an approved save.

### Out of scope

- Approving/marking the spec immutable, or amendment handling.
- Overwrite protection, diffing against an existing spec, or versioning.
- Creating a branch/commit/PR from the saved spec (the existing git chain can do that separately).
- Writing any file other than the spec, or any path outside `docs/specs/<id>/`.
- Auto-allocating the spec id (the id is provided in this slice).

## 8. Technical design

### Rust/Tauri core

Add a confined write helper (in `workspace`): `write_spec_file(root, spec_id, content) -> Result<String, String>` that builds `docs/specs/<spec-id>/spec.md`, verifies `is_within_root`, creates the spec directory, and writes the content, returning the relative path. Extend the `approvals` `PendingAction` with a `SaveSpec { spec_id, content }` variant and add `request_save_spec` (validates spec id via a pure `is_valid_spec_id` and content length, requires an open workspace, records the pending save with the target-path command). `resolve_approval` gains a `SaveSpec` arm that, on approval, calls the write helper and returns the outcome; denial and unknown-id behaviour are unchanged.

### React renderer

Add a save control: a spec-id input and the content (prefilled from the FS-024 draft when present). On save, call `request_save_spec` and show the returned request in the existing approval overlay with the exact target path; Approve/Deny use the existing flow. After an approved save, refresh the workspace specs list. Keep state transient.

### Styling

Reuse the FS-018 approval surface and workspace controls; add a spec-id input consistent with the others.

## 9. Impact notes

- Data model impact: extends the in-memory pending-action set with a save variant; reuses `ApprovalRequest`/`ApprovalOutcome`; the saved file is repo-versioned content the user commits via the existing git chain.
- Security/privacy impact: first workspace file write — confined to `docs/specs/<id>/` by `is_within_root`, gated by approval, validated id/content, core-side `std::fs`, no shell. No write elsewhere, no deletion, no git/network, no capability added.
- Observability impact: save request/approve/deny can be noted in the FS-011 session log.
- Performance impact: negligible; one small file write on approval.
- Migration/backward compatibility impact: additive; the gate and all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: writing at request time. Mitigation: request only validates and records; tests assert no file after request.
- Risk: path traversal. Mitigation: id validated to `FS-<digits>`, path built by the core, `is_within_root` enforced before write.
- Risk: clobbering an existing spec. Mitigation: a plain write replaces the spec file; overwrite protection and immutability are out of scope and a later spec; the user reviews before approving.
- Risk: scope creep into approve/commit. Mitigation: save only; the git chain and approval/immutability are separate.
- Dependency: FS-024 merged (draft) and FS-018 gate, FS-010/FS-015 workspace helpers.

## 11. Open questions

None. This slice saves a spec file into the workspace, confined and gated; approval, immutability, and committing remain separate.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-024 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-025-save-spec`
- Expected implementation PR title: `feat(FS-025): Approval-gated spec save`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-025/amendments/`.
