---
spec_id: FS-050
title: Extension run observability
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0002-process-based-extensibility.md
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0014-supervised-command-execution.md
---

# FS-050: Extension run observability

## 1. Problem statement

The PRD's Phase 5 calls for extension observability (PRD §16): extension calls must be logged with identity, and extension errors/interruption outcomes must be visible. Synth can register extensions (FS-037), surface declared permission scope at the run gate (FS-046), and run an extension command through the supervised command executor, but extension activity is still folded into generic approval/session events. A user cannot inspect a durable, extension-specific trail showing which extension was requested, denied, succeeded, or failed.

This story adds a narrow extension run log. It records extension identity, declared scope, command, status, and outcome detail in app-local storage, and surfaces recent runs beside the extension registry. It does not add new execution powers; it only observes the existing gated run path.

## 2. Requirements

- R1. The Rust core must define an `ExtensionRunRecord` shape that serializes camelCase with at least `id`, `extensionId`, `name`, `kind`, `scope`, `command`, `status`, and `detail`.
- R2. Extension run records must be persisted app-locally in an append-only JSONL store, separate from the registry, and tolerate a missing or malformed file by returning the valid recent records that can be parsed.
- R3. `request_run_extension(id)` must append a `requested` record containing the extension identity, declared scope, and exact command before returning the approval request.
- R4. Resolving an extension run approval must append exactly one terminal record: `denied` when the user denies, `succeeded` when the command completes successfully, or `failed` when command execution returns an error. The terminal record must include the same extension identity/scope/command as the request record, plus readable outcome detail.
- R5. Generic run-command approvals that are not extension runs must keep their current behavior and must not create extension run records.
- R6. The Rust core must expose `list_extension_runs(limit) -> Vec<ExtensionRunRecord>` returning recent extension run records newest-first, bounded by the requested limit.
- R7. The renderer must show recent extension runs in the Extensions surface, including extension name, kind/scope, status, command, and detail, with a calm empty state.
- R8. This story must not add new Tauri capability permissions, must not change the execution semantics of extension commands, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. Requesting a run for a registered extension records a `requested` `ExtensionRunRecord` with the extension's id/name/kind/scope/command and returns the same gated approval request as FS-046.
- AC2. Denying that approval records a `denied` terminal record for the same extension and executes nothing.
- AC3. Approving a successful extension command records a `succeeded` terminal record with the command output detail.
- AC4. Approving a failing extension command records a `failed` terminal record with a readable error detail before returning the error to the renderer.
- AC5. `list_extension_runs(10)` returns at most ten valid records, newest-first, and skips malformed JSONL lines without panicking.
- AC6. Generic `request_run_command` / `RunCommand` approvals do not produce extension run records.
- AC7. The renderer lists recent extension runs and refreshes the list after requesting or resolving an extension run.
- AC8. Rust unit coverage verifies JSONL parse/load ordering, malformed-line tolerance, record construction, and the distinction between extension runs and generic commands. Frontend tests cover formatting/render helper behavior if a helper is added.
- AC9. No new Tauri capabilities are added and existing approval, command execution, and FS-001 runtime contracts remain stable.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Rust unit tests must cover the pure/file-backed record helpers using temporary files. Tests must not execute real extension commands through the shell; success/failure terminal-record behavior can be covered through pure helper functions and source review of the `resolve_approval` branch.

Manual checks:

- Register a harmless extension (for example `echo synth-extension-ok`) with scope `shell`.
- Request a run and confirm the Extensions surface shows a `requested` record.
- Deny the approval and confirm a `denied` terminal record appears, with no command output.
- Request again, approve, and confirm a `succeeded` record includes command output.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Manual run-log note showing requested → denied and requested → succeeded flows.
- Short note confirming generic command approvals remain non-extension records and capabilities are unchanged.

## 5. Success criteria

- SC1. Extension runs have a durable, extension-specific audit trail.
- SC2. Each run record carries extension identity, kind, declared scope, command, status, and readable detail.
- SC3. Denials and failures are visible, not just successful outputs.
- SC4. Generic command execution remains unchanged and is not mislabeled as an extension run.
- SC5. The slice stays story-sized: it observes extension runs but does not introduce MCP brokering, sandbox enforcement, workflow timelines, or richer analytics.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Record completeness | 100% of extension records include id/name/kind/scope/command/status/detail | Rust tests / source review | @BrendanShields |
| Outcome coverage | requested, denied, succeeded, and failed statuses represented in helper coverage/source review | Rust tests / source review | @BrendanShields |
| Ordering and resilience | newest-first bounded listing; malformed lines skipped | Rust tests | @BrendanShields |
| Scope containment | 0 new capabilities; no new execution path | Capability/source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Contract stability | FS-001..FS-049 commands/contracts intact except additive list command | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An app-local extension-run JSONL store.
- `ExtensionRunRecord` plus parse/load/append helpers.
- Recording `requested`, `denied`, `succeeded`, and `failed` statuses for extension-originated command runs.
- A `list_extension_runs(limit)` command.
- A small renderer list of recent extension runs in the Extensions surface.

### Out of scope

- MCP process brokering, protocol transport, or marketplace integration.
- Enforcing extension scopes beyond the existing approval gate.
- Streaming live process output into the extension run log.
- Workflow run timelines or visual workflow execution.
- Editing, deleting, exporting, or filtering run records beyond a simple recent bounded list.
- Recording generic command approvals as extension runs.

## 8. Technical design

### Rust/Tauri core

Extend `extensions` (or a small sibling module if that keeps the boundary cleaner) with:

```text
ExtensionRunRecord { id, extension_id, name, kind, scope, command, status, detail } // serde camelCase
extension_runs_path(app) -> Result<PathBuf, String>          // {app_data_dir}/extension-runs.jsonl
serialize_run_record(record) / parse_run_record(line)        // pure JSONL helpers
append_run_record(path, extension, status, detail) -> Result<ExtensionRunRecord, String>
load_run_records(path, limit) -> Vec<ExtensionRunRecord>     // newest-first, malformed-tolerant
list_extension_runs(app, limit) -> Vec<ExtensionRunRecord>   // #[tauri::command]
```

Change the approval pending action for extension-originated runs from a plain `RunCommand(command)` to a distinct extension run variant carrying the extension identity and command. `request_run_extension` still returns a gated approval request with action `run-command` and the same user-facing command/summary, but it also appends the `requested` record. `resolve_approval` handles the extension variant by appending `denied`, `succeeded`, or `failed` records while preserving the existing execution behavior and error propagation.

Generic command approvals remain `RunCommand(command)` and do not call the extension-run recorder.

### React renderer

Add an `extensionRuns` state array and `refreshExtensionRuns()` using `list_extension_runs({ limit: 20 })`. Call it on initial load, after `requestRunExtension`, and after resolving an approval. Render recent runs below the registered extensions list with name, kind/scope, status, command, and detail. Reuse the existing extension/event list styles.

### Styling

Reuse `doc-extensions`/`doc-events` styles; add only minimal status affordance if needed.

## 9. Impact notes

- Data model impact: introduces an app-local `extension-runs.jsonl` store and `ExtensionRunRecord` IPC shape; additive only.
- Security/privacy impact: records command strings and outputs already visible to the user; no secrets should be placed in extension commands, but the log is app-local and private. No new execution authority, network path, or capability.
- Observability impact: completes the first extension-specific audit surface required by PRD §16.
- Performance impact: bounded append/read of small JSONL records; renderer requests a small recent tail.
- Migration/backward compatibility impact: missing store loads empty; malformed lines are skipped; existing extension registry and generic command approvals remain compatible.

## 10. Risks and dependencies

- Risk: leaking sensitive command output in an app-local log. Mitigation: keep the log private/app-local, display only recent entries, and avoid exporting it in this slice.
- Risk: double-logging extension runs as both generic events and extension records. Mitigation: session events remain high-level UI activity; extension records are the durable extension-specific audit trail.
- Risk: breaking generic command approvals while adding extension identity. Mitigation: keep generic `RunCommand` separate and test/source-review that it does not emit extension records.
- Dependency: FS-037 extension registry, FS-046 scoped run approvals, FS-032 app-local event persistence pattern, FS-035 command execution.

## 11. Open questions

None. This slice records and displays recent extension run activity only; richer extension analytics and MCP brokering are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-049 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-050-extension-observability`
- Expected implementation PR title: `feat(FS-050): Extension run observability`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-050/amendments/`.
