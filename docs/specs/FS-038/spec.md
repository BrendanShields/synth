---
spec_id: FS-038
title: Workflow definitions
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
  - docs/adrs/ADR-0014-supervised-command-execution.md
---

# FS-038: Workflow definitions

## 1. Problem statement

The PRD's Phase 6 introduces deterministic workflows: named, ordered sequences of steps the harness runs in a known order (PRD §17). The foundation is a definition store: name a workflow and give it ordered steps, so the steps can be run one at a time through the existing command gate. This reuses the app-local storage pattern (FS-032/FS-037, ADR-0003) and the gated command runner (FS-035), and is the seam later orchestration (auto-advance, branching, checkpoints) builds on.

This story stores and runs workflow steps; it does not auto-advance, branch, or orchestrate. Each step run is an explicit, gated command — the human approves every step.

## 2. Requirements

- R1. The Rust core must expose `save_workflow(name: String, steps: Vec<String>) -> Result<Workflow, String>` that validates the workflow and stores it in the app-local registry, returning it.
- R2. `Workflow` must serialize in camelCase with at least: `id`, `name`, and `steps` (ordered list of command strings).
- R3. Validation must be pure and unit-testable: the name non-empty and within a length cap, at least one step, every step non-empty and within a length cap, and a bounded maximum number of steps. Validating a workflow must be a separate pure function from storage.
- R4. The core must expose `list_workflows() -> Vec<Workflow>` and `remove_workflow(id: u64) -> Result<(), String>`.
- R5. The store must be in the app-data directory (resolved via the Tauri path API), never the workspace/repo; loading/saving to a path must be unit-testable; a missing/malformed store loads as empty.
- R6. Running a step must go through the existing command gate (FS-035): the renderer runs a chosen step via `request_run_command(step)`, requiring explicit approval and running in the jailed workspace. This story adds no new execution path, no auto-advance, and no new authority.
- R7. The renderer must list workflows (name + ordered steps), provide a form to define one (name + steps), allow removing one, and run an individual step via the command gate. Display is transient renderer state derived from the core.
- R8. This story must not auto-run or auto-advance steps, must not write to the workspace/repo, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `save_workflow("verify", ["bun run test", "cargo test"])` returns a `Workflow` with an id and the ordered steps and persists it; `list_workflows` then includes it.
- AC2. `save_workflow` with an empty name, no steps, or a step that is empty/over the cap returns `Err` and stores nothing.
- AC3. `remove_workflow(id)` removes it; `list_workflows` no longer includes it; an unknown id returns `Err`.
- AC4. With no store file, `list_workflows` returns an empty list (no error).
- AC5. Running a step from the renderer issues `request_run_command(step)`, which requires explicit approval (never auto-approved) and runs in the jailed workspace; steps do not auto-advance.
- AC6. The store file is in the app-data directory, not the workspace/repo.
- AC7. Rust unit coverage verifies workflow validation (good and bad), the load/save/remove round-trip against a temp file (with malformed-tolerance), and camelCase serialization preserving step order.
- AC8. No code in this story auto-advances steps, writes to the workspace/repo, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: load/save/remove are tested against a temporary store file; validation is pure. No network or workspace I/O in tests.

Manual checks:

- Define a workflow with two steps, confirm it lists in order; remove it, confirm it’s gone.
- Run a step and confirm it goes through the approval surface (gated), then shows output; confirm the next step does not run automatically.
- Define a workflow with an empty step and confirm a calm validation error.
- Confirm the store file is under the app-data directory and `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of a workflow with steps and a gated step run.
- Short note confirming app-local storage, gated per-step run (no auto-advance, no new authority), and unchanged capabilities.

## 5. Success criteria

- SC1. Workflows (name + ordered steps) can be defined, listed, and removed in app-local storage.
- SC2. Steps run one at a time through the gated command boundary — no auto-advance, no new authority.
- SC3. The store is private operational state, never in the repo.
- SC4. No auto-run/auto-advance, workspace write, or new capability is introduced.
- SC5. The slice stays story-sized and is the foundation for later orchestration.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Workflow CRUD | save/list/remove correct; malformed tolerated | Rust tests | @BrendanShields |
| Validation | name/steps rules enforced; order preserved | Rust tests | @BrendanShields |
| Gated run | per-step run goes through approval; no auto-advance | Source review / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 auto-advance, 0 workspace writes, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-037 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `save_workflow` / `list_workflows` / `remove_workflow` over an app-local store.
- A `Workflow` camelCase shape; pure validation and load/save helpers.
- A renderer workflow list, define form, remove, and per-step gated run.

### Out of scope

- Auto-advancing, branching, conditionals, loops, or parallel steps.
- Step types beyond commands (e.g. spec actions, approvals as steps).
- Orchestration, scheduling, retries, or run history.
- Editing workflows in place (remove + re-save suffices here).

## 8. Technical design

### Rust/Tauri core

Add a `workflows` module mirroring `extensions`:

```text
Workflow { id, name, steps }                          // serde camelCase, (de)serializable
validate_workflow(name, steps) -> Result<(), String>   // pure
load_store(path) -> Vec<Workflow>                      // [] if missing/malformed
save_store(path, &[Workflow]) -> Result<(), String>
save_workflow(app, name, steps) -> Result<Workflow, String>   // #[tauri::command]
list_workflows(app) -> Vec<Workflow>                    // #[tauri::command]
remove_workflow(app, id) -> Result<(), String>          // #[tauri::command]
```

The store is a JSON array at `{app_data_dir}/workflows.json`. `save_workflow` validates, loads, assigns the next id, pushes, and saves. Running is not added here — the renderer reuses `request_run_command` per step.

### React renderer

Add a workflows surface: a list (name + ordered steps, each step with a gated Run) and a define form (name + steps, e.g. newline-separated). Remove per workflow. Refresh after save/remove.

### Styling

Reuse list/control styles.

## 9. Impact notes

- Data model impact: introduces a `Workflow` IPC shape and an app-local JSON store; private operational state (ADR-0003).
- Security/privacy impact: definitions are app-local (never the repo); running a step is a gated command with no new authority (ADR-0014). No capability added.
- Observability impact: define/remove/step-run can be noted in the event log.
- Performance impact: negligible; small store file.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: a dangerous step command. Mitigation: every step run goes through the command gate (explicit approval, jailed, bounded); no auto-advance.
- Risk: store corruption. Mitigation: malformed store loads empty; whole-file writes.
- Risk: scope creep into orchestration. Mitigation: this slice only stores/lists/runs-per-step; orchestration is a later spec.
- Dependency: FS-032/FS-037 (app-local storage pattern), FS-035 (gated command run).

## 11. Open questions

None. This slice defines and runs workflow steps under the gate; orchestration and richer step types are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-037 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-038-workflow-definitions`
- Expected implementation PR title: `feat(FS-038): Workflow definitions`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-038/amendments/`.
