---
spec_id: FS-045
title: State import
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

# FS-045: State import

## 1. Problem statement

FS-041 exports the app-local state to a bundle; the matching restore is missing (PRD §19, backup/export). This story imports the authored portions of a bundle — the extension registry (FS-037) and workflow definitions (FS-038) — so a user can move their setup between machines or recover it. The durable event log is deliberately not overwritten: it is an audit history, not portable configuration.

Import is bounded and explicit: it reads the export bundle from the app-data directory, validates it, and replaces the extensions and workflows stores. It does not touch the repo, run anything, or clobber the event log.

## 2. Requirements

- R1. The Rust core must expose `import_state() -> Result<ImportSummary, String>` that reads `{app_data_dir}/synth-export.json`, validates it as a bundle, and restores the extensions and workflows stores from it, returning counts.
- R2. Parsing/validating a bundle from text must be a pure, unit-testable function returning a typed bundle or a readable error (malformed input is an error, not a panic).
- R3. Import must restore only the extensions and workflows stores; it must not modify or overwrite the event log (`events.jsonl`) or the session tree.
- R4. A missing bundle file must return a readable `Err` (nothing changed). Restoring writes whole-file JSON to the two stores, reusing their existing save paths.
- R5. `ImportSummary` must serialize in camelCase with at least `extensions` and `workflows` (counts restored).
- R6. The renderer must provide an import control that calls `import_state`, shows the summary, refreshes the extensions and workflows lists, and shows a calm error on failure. Import state must be transient renderer state.
- R7. This story must not import or alter the event log, must not write to the workspace/repo, must not perform any network operation, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. Given a valid `synth-export.json`, `import_state` restores its extensions and workflows and returns counts matching the bundle.
- AC2. After import, the event log (`events.jsonl`) and session tree are unchanged.
- AC3. A missing bundle file returns `Err` and changes nothing.
- AC4. A malformed bundle returns `Err` (no panic) and changes nothing.
- AC5. `ImportSummary` serializes in camelCase (`extensions`, `workflows`).
- AC6. The renderer imports, shows the summary, and the extensions/workflows lists reflect the restored data.
- AC7. Rust unit coverage verifies bundle parsing (valid + malformed) and that a parsed bundle yields the expected collections; the event log is out of the restore path.
- AC8. No code in this story alters the event log, writes to the workspace/repo, performs a network operation, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: bundle parsing is tested purely; restore reuses the existing store save paths against a temporary directory pattern. No network in tests.

Manual checks:

- Export state (FS-041), modify/remove an extension, then import and confirm the extension is restored.
- Confirm the event log is unchanged after import.
- Import with no bundle present and confirm a calm error.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- The import summary counts and a note confirming the event log is untouched.
- Short note confirming app-data-only read/write and unchanged capabilities.

## 5. Success criteria

- SC1. Authored extensions and workflows can be restored from an export bundle.
- SC2. The event log/audit history is never clobbered by import.
- SC3. Import is bounded, explicit, and app-data-only.
- SC4. No repo write, network, or new capability is introduced.
- SC5. The slice stays story-sized — restore of authored stores; merge strategies and event-history import are deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Restore correctness | extensions + workflows restored to counts | Rust tests / manual | @BrendanShields |
| Log safety | event log unchanged after import | Manual / source | @BrendanShields |
| Robustness | malformed/missing bundle → Err, no change | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 repo writes, 0 network, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-044 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `import_state` restoring extensions + workflows from the app-data bundle.
- A pure bundle parser/validator with unit tests.
- An `ImportSummary` camelCase shape.
- A renderer import control that refreshes the restored lists.

### Out of scope

- Importing/merging the event log or session tree.
- Merge/conflict strategies (import replaces the two stores wholesale).
- Choosing the bundle location via a dialog (fixed app-data path).
- Importing provider config/secrets (never exported).
- Versioned/migrating bundle schemas.

## 8. Technical design

### Rust/Tauri core

In `backup`, add:

```text
ImportSummary { extensions, workflows }                 // serde camelCase
parse_bundle(text) -> Result<ExportBundle, String>       // pure
import_state(app) -> Result<ImportSummary, String>        // #[tauri::command]
```

`import_state` reads `{app_data_dir}/synth-export.json` (Err if missing), `parse_bundle`s it, and writes the extensions and workflows via their existing `save_registry`/`save_store` to their resolved paths. The event log and session tree are not touched.

### React renderer

Add an "Import state" control near "Export state" that calls `import_state`, shows the summary, and calls `refreshExtensions` + `refreshWorkflows`.

### Styling

Reuse control/notice styles.

## 9. Impact notes

- Data model impact: introduces an `ImportSummary` shape; reuses the existing stores.
- Security/privacy impact: app-data-only read/write; no repo write, no network, no capability; no secrets (none are exported).
- Observability impact: import can be noted in the event log (an append, not a restore of it).
- Performance impact: a bounded read + two whole-file writes.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: clobbering the audit log. Mitigation: import never writes the event log; only extensions + workflows.
- Risk: malformed bundle. Mitigation: pure parser returns a readable error and changes nothing.
- Risk: silent overwrite of current stores. Mitigation: import is an explicit user action and reports a summary; merge strategies are a later spec.
- Dependency: FS-041 export, FS-037 extensions, FS-038 workflows.

## 11. Open questions

None. This slice restores authored extensions and workflows from a bundle; merge strategies, event-log import, and schema versioning are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-044 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-045-state-import`
- Expected implementation PR title: `feat(FS-045): State import`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-045/amendments/`.
