---
spec_id: FS-041
title: State export
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
---

# FS-041: State export

## 1. Problem statement

The PRD's Phase 8 includes backup/export (PRD §19). Synth now keeps operational state in several app-local stores — the event log (FS-032), the extension registry (FS-037), and workflow definitions (FS-038). There is no way to back that up or move it. This story adds a single export: gather the app-local state into one JSON bundle written to the app-data directory, so it can be backed up or copied. It is the credential-free part of Phase 8 (unlike signing/notarization/auto-update, which need a release identity and are out of scope).

Export is read-only over the sources and writes one bundle file to the existing app-data directory — no new capability, no network, no repo write.

## 2. Requirements

- R1. The Rust core must expose `export_state() -> Result<String, String>` that gathers the app-local state (events, extensions, workflows) into one bundle, writes it to `{app_data_dir}/synth-export.json`, and returns the written path.
- R2. Building the bundle from the in-memory state (events, extensions, workflows) must be a pure, unit-testable function producing a camelCase structure with at least `events`, `extensions`, and `workflows`.
- R3. Export must be read-only over the source stores (it must not modify the event log, registry, or workflow store) and must write only the single bundle file in the app-data directory (never the workspace/repo).
- R4. A missing source store must export as an empty collection for that section, not an error.
- R5. The renderer must provide an export control that calls `export_state` and shows the resulting path (and a calm error on failure). Export state must be transient renderer state only.
- R6. This story must not export secrets (e.g. provider API keys are never persisted and must not appear), must not perform any network operation, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `export_state` writes `{app_data_dir}/synth-export.json` containing `events`, `extensions`, and `workflows`, and returns that path.
- AC2. The bundle reflects the current app-local state; a missing source store yields an empty array for that section (no error).
- AC3. The source stores are unchanged after export (read-only over sources).
- AC4. The exported bundle contains no provider API key or other secret.
- AC5. The renderer exports and shows the path; a failure shows a calm error.
- AC6. Rust unit coverage verifies the pure bundle builder (camelCase keys; sections reflect inputs; empty inputs yield empty arrays).
- AC7. No code in this story modifies a source store, writes to the workspace/repo, performs a network operation, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the bundle builder is tested in memory; the file write reuses the established app-data write pattern. No network in tests.

Manual checks:

- Create a workflow and an extension, run a command, then export and confirm `synth-export.json` contains all three sections.
- Confirm the exported file contains no API key.
- Confirm the source stores are unchanged and `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- The exported bundle's top-level keys and a note confirming no secrets.
- Short note confirming read-only sources, app-data-only write, and unchanged capabilities.

## 5. Success criteria

- SC1. The app-local state can be exported to a single bundle for backup.
- SC2. Export is read-only over sources and writes only to app-data.
- SC3. No secrets, network, or new capability are involved.
- SC4. Missing stores degrade to empty sections.
- SC5. The slice stays story-sized — export only; import/restore and scheduling are deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Bundle completeness | events + extensions + workflows present | Rust tests / manual | @BrendanShields |
| Read-only sources | source stores unchanged after export | Manual / source | @BrendanShields |
| No secrets | no API key in bundle | Manual / source | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 network, 0 repo writes, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-040 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `export_state` gathering events + extensions + workflows into one app-data bundle.
- A pure bundle builder with unit tests.
- A renderer export control showing the path.

### Out of scope

- Import/restore from a bundle.
- Choosing the export location via a save dialog (would need a new capability) — fixed app-data path here.
- Scheduling, rotation, or encryption of backups.
- Exporting workspace/repo content (that lives in git) or knowledge notes (committed in the repo).
- Exporting provider secrets (never persisted).

## 8. Technical design

### Rust/Tauri core

Add a `backup` module:

```text
ExportBundle { events, extensions, workflows }                 // serde camelCase
build_bundle(events, extensions, workflows) -> ExportBundle     // pure
export_state(app) -> Result<String, String>                     // #[tauri::command]
```

`export_state` resolves the app-data directory, loads the three stores via their existing loaders (missing → empty), builds the bundle, writes `synth-export.json` (pretty JSON), and returns the path. The store path helpers are reused (made shared) rather than duplicating filenames.

### React renderer

Add an "Export state" control (e.g. near the event stream) that calls `export_state` and shows the returned path, with a calm error state.

### Styling

Reuse control/notice styles.

## 9. Impact notes

- Data model impact: introduces an `ExportBundle` IPC/file shape; no new persisted entities (reuses existing stores).
- Security/privacy impact: read-only over sources; writes one bundle to app-data; no secrets (API keys are never persisted), no network, no capability.
- Observability impact: export can be noted in the event log.
- Performance impact: a bounded read of the stores and one file write.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: leaking secrets. Mitigation: provider keys are serde-skipped and never persisted, so they are absent from the sources and the bundle; verified.
- Risk: corrupting sources. Mitigation: export only reads sources and writes a separate bundle file.
- Risk: scope creep into restore. Mitigation: this slice is export-only; import/restore is a later spec.
- Dependency: FS-032 events, FS-037 extensions, FS-038 workflows (app-local stores).

## 11. Open questions

None. This slice exports app-local state to a single bundle; import/restore, save-dialog location, and encryption are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-040 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-041-state-export`
- Expected implementation PR title: `feat(FS-041): State export`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-041/amendments/`.
