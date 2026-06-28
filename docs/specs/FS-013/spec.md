---
spec_id: FS-013
title: Detect the workspace planning baseline
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
  - docs/adrs/ADR-0010-workspace-jail.md
  - docs/adrs/ADR-0004-planning-baseline-gate.md
---

# FS-013: Detect the workspace planning baseline

## 1. Problem statement

FS-012 lets the user open a jailed workspace, but Synth still knows nothing about what is inside it. The PRD's central gate (PRD §7, ADR-0004) is that project-level implementation is blocked until a planning baseline — a PRD and an ERD/HLSA — exists and is merged. The first useful thing Synth can learn about an opened repository is therefore whether that baseline is present.

This story performs the first jailed file read: confined to the workspace root (ADR-0010), the trusted core checks for the conventional planning documents and reports whether the baseline is complete. It reads only specific, known repo-relative paths for existence; it does not list directories, read arbitrary files, parse document contents, or enforce the gate yet. It makes Synth *aware* of the baseline so later specs can act on it.

## 2. Requirements

- R1. The Rust core must expose a Tauri command `inspect_planning_baseline() -> Result<PlanningBaseline, String>` that, using the currently open workspace, reports the presence of the conventional planning documents.
- R2. `PlanningBaseline` must serialize in camelCase with at least: `prdPresent` (boolean), `erdPresent` (boolean), and `complete` (boolean, true only when both are present).
- R3. The checked paths must be the conventional repo-relative locations `docs/PRD.md` and `docs/engineering/ERD.md`, resolved against the workspace root.
- R4. Every checked path must be confined to the workspace root using the ADR-0010 `is_within_root` primitive before any filesystem access; a path that would escape the root must be treated as not present, never read.
- R5. Detection must check only existence/file-ness of those specific paths. It must not list directories, read or parse file contents, follow into other files, perform git, or perform network operations.
- R6. If no workspace is open, `inspect_planning_baseline` must return a readable `Err` and perform no filesystem access.
- R7. The detection logic must be a parameterized, unit-testable function taking a root path (so it can be tested against a temporary directory), separate from the command that reads the open-workspace state.
- R8. The renderer must, when a workspace is open, request the baseline and display it quietly: whether the baseline is complete, and which of PRD / ERD is present or missing. Before a workspace is open, it must show nothing or a calm neutral state.
- R9. This story must not add new Tauri capability permissions (detection uses core-side `std::fs`, not a renderer filesystem capability), must not write to the workspace, and must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. With a workspace open whose `docs/PRD.md` and `docs/engineering/ERD.md` both exist, `inspect_planning_baseline` returns `prdPresent: true`, `erdPresent: true`, `complete: true`.
- AC2. With a workspace open that has neither file, it returns all false; with only one present, `complete` is false and the present flag is true for that one.
- AC3. With no workspace open, `inspect_planning_baseline` returns `Err` and reads nothing.
- AC4. The detection function only ever checks the two confined paths; a crafted root cannot cause it to read outside the root (covered by using `is_within_root`).
- AC5. The renderer shows a calm baseline summary when a workspace is open (e.g., "planning baseline complete" or which document is missing) and nothing intrusive before opening.
- AC6. Rust unit coverage verifies detection against a temp directory for the all-present, none-present, and one-present cases, and camelCase serialization.
- AC7. No code in this story lists directories, reads file contents, performs git or network operations, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: detection is tested against a temporary directory created by the test (creating empty marker files and checking the flags). No network I/O.

Manual checks:

- Open the Synth repo itself as a workspace and confirm the baseline shows complete (both PRD and ERD present).
- Open a folder without `docs/PRD.md` and confirm it shows the baseline as incomplete with the missing document indicated.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the baseline status for an opened workspace.
- Short note confirming only the two confined paths are checked, no contents are read, and capabilities are unchanged.

## 5. Success criteria

- SC1. Synth reports whether an opened repository has the conventional planning baseline.
- SC2. All detection is confined to the workspace root via the ADR-0010 primitive.
- SC3. Detection reads no file contents and lists no directories.
- SC4. No new capability is introduced.
- SC5. The slice stays story-sized and does not enforce the gate or read document bodies.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Detection correctness | all-present / none / one-present cases correct | Rust tests | @BrendanShields |
| Jail confinement | only confined paths checked; escape treated as absent | Rust tests / source | @BrendanShields |
| Read-only minimal | 0 directory listings, 0 content reads, 0 git/network | Source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Capability containment | 0 new Tauri capabilities | Capability diff | @BrendanShields |
| Contract stability | FS-001..FS-012 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `PlanningBaseline` camelCase shape and an `inspect_planning_baseline` command.
- A parameterized, unit-tested detection function confined by `is_within_root`.
- Existence checks for `docs/PRD.md` and `docs/engineering/ERD.md` only.
- A quiet renderer baseline status when a workspace is open.

### Out of scope

- Reading or parsing document contents, frontmatter, or titles.
- Listing directories, scanning for specs, or detecting ADRs/CODEOWNERS.
- Enforcing the planning-baseline gate or blocking any action (later spec).
- Git status or any write.
- Persisting baseline results.

## 8. Technical design

### Rust/Tauri core

Extend the `workspace` module:

```text
PlanningBaseline { prd_present, erd_present, complete }   // serde camelCase
detect_planning_baseline(root: &Path) -> PlanningBaseline  // parameterized, testable
inspect_planning_baseline(state) -> Result<PlanningBaseline, String>  // #[tauri::command]
```

`detect_planning_baseline` joins the two conventional relative paths to the root, verifies each is within the root via `is_within_root`, and checks `is_file()`. `complete` is the conjunction. `inspect_planning_baseline` reads the workspace root from managed state (Err if none) and delegates. No directory listing, no content reads.

### React renderer

When a workspace is open (or on open/change), call `inspect_planning_baseline` and store the result in transient state. Display a quiet one-line baseline status near the workspace surface: complete, or which document is missing. Reuse the FS-011 session log to optionally note the detection.

### Styling

Reuse muted/mono styles; add only a single quiet status line. No badges or noisy chrome.

## 9. Impact notes

- Data model impact: introduces a `PlanningBaseline` IPC shape; no persisted entities.
- Security/privacy impact: first jailed file read; limited to existence checks of two confined, conventional paths via `is_within_root`. No content reads, no capability added.
- Observability impact: detection can appear in the FS-011 session log; no event store yet.
- Performance impact: negligible; two path stats per inspection.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: drifting into reading document contents. Mitigation: existence checks only; contents are out of scope.
- Risk: path escaping the jail. Mitigation: `is_within_root` precedes every stat; escape is treated as absent.
- Risk: implying the gate is enforced. Mitigation: this story only reports presence; enforcement is a later spec.
- Dependency: FS-012 merged (workspace root and `is_within_root`).

## 11. Open questions

None. This slice detects the conventional planning baseline of an open workspace and reports it, confined to the jail.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-012 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-013-planning-baseline`
- Expected implementation PR title: `feat(FS-013): Detect the workspace planning baseline`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-013/amendments/`.
