---
spec_id: FS-015
title: List workspace specs
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
  - docs/adrs/ADR-0006-story-sized-immutable-feature-specs.md
---

# FS-015: List workspace specs

## 1. Problem statement

Synth can now open a repository, detect its planning baseline (FS-013), and read its planning documents (FS-014) — but the specs index it shows (FS-004/FS-005) is still the static, self-referential catalog of Synth's own specs. To work on a real repository, Synth must surface *that repository's* feature specs.

This story adds the first confined directory read: list the immediate `docs/specs/<spec-id>/` entries in the opened workspace that contain a `spec.md`, returning their ids and paths. It is the bridge from the static catalog to the real workspace. It is a single-level listing of one conventional location, jailed to the workspace root (ADR-0010); it does not recurse, read spec contents, parse markdown, or scan arbitrary directories.

## 2. Requirements

- R1. The Rust core must expose a Tauri command `list_workspace_specs() -> Result<Vec<WorkspaceSpec>, String>` that lists the feature-spec directories under the open workspace's `docs/specs/`.
- R2. `WorkspaceSpec` must serialize in camelCase with at least: `specId` (the directory name) and `path` (repo-relative path to that spec's `spec.md`).
- R3. The listing must read only the single directory `<root>/docs/specs`, one level deep. For each immediate child that is a directory, whose name matches a spec-id pattern (`FS-` followed by digits, case-insensitive), and which contains a `spec.md` file, it must emit one `WorkspaceSpec`.
- R4. The `docs/specs` path and each candidate `spec.md` path must be confined to the workspace root via the ADR-0010 `is_within_root` primitive before access.
- R5. Results must be returned in ascending `specId` order.
- R6. If no workspace is open, the command must return a readable `Err` and read nothing. If `docs/specs` does not exist, it must return an empty list (not an error).
- R7. The directory must not be read recursively; no file contents (including `spec.md`) may be read or parsed in this story; no other directory may be scanned; no git or network operation may be performed; nothing may be written.
- R8. The pure classification — whether a directory entry name is a valid spec id, and building the relative `spec.md` path — must be unit-testable without filesystem access.
- R9. The renderer must, when a workspace is open, show the workspace specs (ids and paths) as a quiet list, distinct from the static catalog, with a calm empty state when none are found and a readable error state on failure. Listing state must be transient renderer state only.
- R10. This story must not add new Tauri capability permissions (it uses core-side `std::fs`), must not write to the workspace, and must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. With a workspace whose `docs/specs/FS-001/spec.md` and `docs/specs/FS-002/spec.md` exist, `list_workspace_specs` returns two `WorkspaceSpec`s with `specId` `FS-001`, `FS-002` and the correct repo-relative `spec.md` paths, in ascending order.
- AC2. A `docs/specs` child that is a file, that does not match the spec-id pattern, or that lacks a `spec.md` is excluded.
- AC3. With no workspace open, the command returns `Err` and reads nothing.
- AC4. With a workspace that has no `docs/specs` directory, the command returns an empty list (no error).
- AC5. The listing reads only one directory level and no file contents (verified by source review and tests).
- AC6. The renderer shows the workspace specs as a quiet list, with calm empty and error states.
- AC7. Rust unit coverage verifies the pure spec-id classification/path building, and the listing against a temporary workspace (valid entries, excluded entries, ordering, empty/no-dir cases).
- AC8. No code in this story recurses, reads file contents, scans other directories, performs git or network operations, adds a Tauri capability, writes, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: listing is tested against a temporary workspace created by the test (making spec directories with and without `spec.md`, plus a non-matching entry). No network I/O.

Manual checks:

- Open the Synth repo as a workspace and confirm its FS-0xx specs are listed.
- Open a folder without `docs/specs` and confirm an empty list, not an error.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the workspace specs list for an opened repo.
- Short note confirming a single-level listing of `docs/specs`, no content reads, and unchanged capabilities.

## 5. Success criteria

- SC1. Synth lists the opened repository's feature specs from `docs/specs`.
- SC2. The listing is confined, single-level, and reads no file contents.
- SC3. Missing `docs/specs` is an empty list, not an error.
- SC4. No new capability and no writes are introduced.
- SC5. The slice stays story-sized and does not parse spec contents or build a full spec reader.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Listing correctness | Valid specs listed, non-specs excluded, ordered | Rust tests | @BrendanShields |
| Confinement | `docs/specs` and `spec.md` paths confined; single level only | Rust tests / source | @BrendanShields |
| Read-only minimal | 0 recursion, 0 content reads, 0 other dirs, 0 git/network | Source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Capability containment | 0 new Tauri capabilities | Capability diff | @BrendanShields |
| Contract stability | FS-001..FS-014 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `WorkspaceSpec` camelCase shape and a `list_workspace_specs` command.
- A single-level, confined read of `docs/specs` with spec-id + `spec.md` filtering.
- Pure spec-id classification and relative-path building with unit tests.
- A quiet renderer list of workspace specs with empty/error states.

### Out of scope

- Reading or parsing `spec.md` contents, frontmatter, titles, or status.
- Recursing into spec directories or scanning amendments.
- Selecting a workspace spec as the active artifact or grounding asks in it (later spec).
- Detecting ADRs, releases, knowledge docs, or CODEOWNERS.
- Git status, writes, or persistence.

## 8. Technical design

### Rust/Tauri core

Extend the `workspace` module:

```text
WorkspaceSpec { spec_id, path }   // serde camelCase
spec_id_from_dir_name(name: &str) -> Option<String>   // pure: validate FS-\d+ pattern
list_workspace_specs(state) -> Result<Vec<WorkspaceSpec>, String>   // #[tauri::command]
```

`spec_id_from_dir_name` returns the canonical id (e.g., uppercased) for names matching the pattern, else `None`. `list_workspace_specs` reads the workspace root (Err if none), confines and reads `<root>/docs/specs` one level via `std::fs::read_dir` (empty list if the directory is absent), and for each child directory with a valid spec-id name and an existing `spec.md` (confined), emits a `WorkspaceSpec` with the repo-relative `docs/specs/<id>/spec.md` path. Sort ascending by `spec_id`. No recursion, no content reads.

### React renderer

When a workspace is open, call `list_workspace_specs` and render the ids and paths as a quiet list (distinct from the static specs index), with calm empty and error states. Keep state transient.

### Styling

Reuse the existing quiet list/mono styles; add nothing beyond a simple list. No selection affordance in this story.

## 9. Impact notes

- Data model impact: introduces a `WorkspaceSpec` IPC shape; no persisted entities.
- Security/privacy impact: first directory listing; single conventional location, one level, jailed by `is_within_root`; no file contents read; no capability added; no writes.
- Observability impact: listing can be noted in the FS-011 session log; no event store yet.
- Performance impact: one directory read per listing; negligible.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: scope creep into reading spec contents. Mitigation: this story lists only; contents/selection are later specs.
- Risk: scanning arbitrary directories. Mitigation: only `<root>/docs/specs`, one level, confined.
- Risk: missing directory treated as failure. Mitigation: absent `docs/specs` returns an empty list.
- Dependency: FS-014 merged (workspace root, `is_within_root`).

## 11. Open questions

None. This slice lists the opened repository's feature specs from `docs/specs`, confined and read-only.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-014 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-015-list-workspace-specs`
- Expected implementation PR title: `feat(FS-015): List workspace specs`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-015/amendments/`.
