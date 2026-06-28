---
spec_id: FS-030
title: Approval-gated amendment
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0006-story-sized-immutable-feature-specs.md
  - docs/adrs/ADR-0007-amendments-always-pause-work.md
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0010-workspace-jail.md
---

# FS-030: Approval-gated amendment

## 1. Problem statement

Approved specs are immutable; when implementation reveals a spec is wrong, incomplete, or blocked, the deviation is captured as an amendment rather than an in-place edit (PRD §11, ADR-0006, ADR-0007). FS-025 lets Synth save a spec; this story adds the parallel ability to save an **amendment** for an existing spec, into `docs/specs/<spec-id>/amendments/<amendment-id>.md`, gated by approval (ADR-0012) and confined to the jail (ADR-0010).

Like spec save, requesting an amendment validates the ids and content and records a pending approval with the exact target path; nothing is written. On approval the core writes the amendment file; on denial nothing is written. This makes the immutable-spec + amendment workflow real in-app, without editing the approved spec.

## 2. Requirements

- R1. The Rust core must expose `request_save_amendment(specId: String, amendmentId: String, content: String) -> Result<ApprovalRequest, String>` that validates the spec id, amendment id, and content and, if a workspace is open, records a pending approval (reusing the FS-018 store) capturing the exact target path. It must not write anything.
- R2. The returned `ApprovalRequest` must reuse the FS-018 shape with `action` `save-amendment` and `command` describing the write (e.g. `write docs/specs/FS-005/amendments/AMD-001.md`).
- R3. Spec-id validation reuses the `FS-<digits>` rule. Amendment-id validation must be a pure, unit-testable function accepting the canonical `AMD-<digits>` form (case-insensitive, normalized to uppercase) and rejecting anything else. Content must be rejected if empty/whitespace-only or exceeding a fixed maximum length.
- R4. The target path must be `docs/specs/<spec-id>/amendments/<amendment-id>.md` relative to the workspace root, confined with `is_within_root` before any write; a path that escapes the root must error and never be written.
- R5. `resolve_approval` (the FS-018 command) must, for a pending amendment and only when `approved`, create the amendments directory if needed and write the content to the target path within the jail, then clear the pending approval. A denial clears it and writes nothing.
- R6. The write must create only the spec's `amendments/` directory and the single amendment file; it must not write outside `docs/specs/<spec-id>/amendments/`, must not modify the approved spec or unrelated files, must not perform git or network operations, and must not run a shell.
- R7. Write failures must return a readable `Err` without panicking, and must not leave a partial pending approval.
- R8. The action captured at request time must be exactly what executes on approval (no substitution), consistent with ADR-0012.
- R9. The renderer must let the user save an amendment (spec id, amendment id, content), show the pending request in the FS-018 approval surface with the exact target path, and resolve it via the existing Approve/Deny flow.
- R10. This story must not add new Tauri capability permissions, must not edit the approved spec, and must not change the FS-001 runtime status contract or the FS-018 approval-gate semantics.

## 3. Acceptance criteria

- AC1. `request_save_amendment("FS-005", "AMD-001", "<content>")` with a workspace open returns an `ApprovalRequest` with `action: save-amendment` and a `command` referencing `docs/specs/FS-005/amendments/AMD-001.md`, and records a pending approval; no file is written.
- AC2. `request_save_amendment` with an invalid spec id, invalid amendment id (`amd`, `AMD-`, `../x`), or empty content returns `Err` and records no pending approval.
- AC3. `request_save_amendment` with no workspace open returns `Err` and records no pending approval.
- AC4. `resolve_approval(id, true)` writes the content to `docs/specs/<spec-id>/amendments/<amendment-id>.md`; the file exists afterward and the approved spec file is unchanged.
- AC5. `resolve_approval(id, false)` writes nothing.
- AC6. A path that would escape the workspace is rejected and never written.
- AC7. The renderer saves an amendment, shows the exact target path, and on approval the amendment file exists; denying writes nothing.
- AC8. Rust unit coverage verifies amendment-id and content validation, the confined write into a temporary workspace (and escape refusal), and the request→pending→resolve lifecycle. FS-018..FS-029 tests remain intact.
- AC9. No code in this story writes outside `docs/specs/<spec-id>/amendments/`, edits the spec, performs git/network operations, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the confined write is tested against a temporary workspace (writing an amendment file and asserting its contents and that an escaping path is refused). No network operations.

Manual checks:

- Save an amendment for an existing spec and confirm the approval surface shows `write docs/specs/<id>/amendments/<amd>.md`.
- Approve and confirm the amendment file exists and the spec file is unchanged; deny another and confirm nothing is written.
- Try invalid ids and confirm calm validation errors with no pending approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the approval surface for an amendment and confirmation of an approved write (file present, spec unchanged) and a denied no-op.
- Short note confirming the write is confined to `amendments/`, the spec is never edited, and capabilities are unchanged.

## 5. Success criteria

- SC1. An amendment can be saved for a spec into `amendments/<id>.md`, gated by approval.
- SC2. The approved spec is never edited; amendments are additive (ADR-0006, ADR-0007).
- SC3. The write is confined to the jail and to the spec's `amendments/` directory.
- SC4. No spec edit, git/network, or new capability is introduced.
- SC5. The slice stays story-sized and does not auto-pause work or enforce the amendment lifecycle beyond saving.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Gate integrity | 0 writes at request time; write only on approval | Rust tests / source | @BrendanShields |
| Validation | invalid spec/amendment id or empty content rejected | Rust tests | @BrendanShields |
| Confinement | write inside amendments/ only; spec untouched; escape refused | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 spec edits, 0 git/network, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-029 commands/contracts intact; gate semantics unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_save_amendment` reusing the FS-018 approval store, with a `save-amendment` pending action.
- Pure amendment-id and content validation with unit tests.
- A confined `std::fs` write to `docs/specs/<id>/amendments/<amd>.md` in `resolve_approval`, tested against a temp workspace.
- A renderer save control (spec id + amendment id + content) reusing the FS-018 approval surface.

### Out of scope

- Editing the approved spec or any file other than the amendment.
- Auto-pausing in-flight work or enforcing the full amendment lifecycle/telemetry (ADR-0007 pause behaviour beyond saving).
- Generating the amendment content with the model.
- Approving/marking the amendment, or committing it (the existing git chain can commit it separately).
- Overwrite protection or versioning of amendments.

## 8. Technical design

### Rust/Tauri core

Add a confined write helper (in `workspace`): `write_amendment_file(root, spec_id, amendment_id, content) -> Result<String, String>` that builds `docs/specs/<spec-id>/amendments/<amendment-id>.md`, verifies `is_within_root`, creates the amendments directory, and writes the content, returning the relative path. Add a pure `amendment_id_from_name(name) -> Option<String>` (`AMD-<digits>` → canonical uppercase). Extend the `approvals` `PendingAction` with a `SaveAmendment { spec_id, amendment_id, content }` variant and add `request_save_amendment` (validates both ids and content length, requires an open workspace, records the pending save). `resolve_approval` gains a `SaveAmendment` arm that, on approval, calls the write helper and returns the outcome; denial and unknown-id behaviour are unchanged.

### React renderer

Add a save-amendment control (spec id, amendment id, content). On save, call `request_save_amendment` and show the returned request in the existing approval overlay with the exact target path; Approve/Deny use the existing flow. Keep state transient.

### Styling

Reuse the FS-018 approval surface and workspace control styles; add inputs consistent with the existing ones.

## 9. Impact notes

- Data model impact: extends the in-memory pending-action set with an amendment variant; reuses `ApprovalRequest`/`ApprovalOutcome`; the amendment file is repo-versioned content the user commits via the existing git chain.
- Security/privacy impact: a confined workspace write to `amendments/` only, gated by approval, ids/content validated, core-side `std::fs`, no shell. No spec edit, no other files, no git/network, no capability added.
- Observability impact: amendment request/approve/deny can be noted in the FS-011 session log.
- Performance impact: negligible; one small file write on approval.
- Migration/backward compatibility impact: additive; the gate and all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: editing the approved spec. Mitigation: the write targets only `amendments/<id>.md`; the spec path is never written.
- Risk: path traversal. Mitigation: both ids validated, path built by the core, `is_within_root` enforced before write.
- Risk: scope creep into the full pause/telemetry lifecycle. Mitigation: this slice only saves the amendment file; the pause/approval lifecycle is future.
- Dependency: FS-025 merged (gated write pattern), FS-018 gate, workspace helpers.

## 11. Open questions

None. This slice saves an amendment file for a spec, confined and gated; the full amendment lifecycle remains separate.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-029 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-030-amendment`
- Expected implementation PR title: `feat(FS-030): Approval-gated amendment`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-030/amendments/`.
