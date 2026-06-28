---
spec_id: FS-012
title: Open a jailed workspace
status: Draft for review
type: feature-spec
created: 2026-06-28
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0001-rust-native-runtime.md
  - docs/adrs/ADR-0010-workspace-jail.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-012: Open a jailed workspace

## 1. Problem statement

Synth's V1 promise is to work on an existing repository, but the app has no notion of a workspace yet — it has never touched the filesystem. The first step toward the spec-to-PR loop is to let the user open a folder and have the trusted core establish it as the single, jailed workspace root (ADR-0010). This is the foundation every later filesystem and git capability will be confined to.

This story is deliberately the smallest safe step across the filesystem boundary: pick a directory, validate and canonicalize it in the Rust core, record it as the workspace root, and expose a path-confinement primitive that later specs reuse. It does not read repository contents, list files, parse anything, or perform git operations. Opening is read-only with respect to workspace contents.

## 2. Requirements

- R1. The Rust core must own a `workspace` module exposing an async Tauri command `open_workspace(path: String) -> Result<Workspace, String>` that validates `path` is an existing directory, canonicalizes it to an absolute path, and records it as the current workspace root.
- R2. `Workspace` must serialize in camelCase with at least: `root` (canonical absolute path) and `name` (final path component / folder name).
- R3. The core must expose `get_workspace() -> Option<Workspace>` returning the currently open workspace, or none when none is open.
- R4. The core must hold the workspace root in shared runtime state (Tauri managed state) so it persists for the process lifetime; it must not be written to disk or app-local storage in this story.
- R5. The core must provide a pure, unit-testable path-confinement function `is_within_root(root, candidate) -> bool` that returns true only when the candidate path, once normalized, is the root or strictly inside it, and false for traversal (`..`), sibling, or unrelated paths. This is the jail primitive ADR-0010 requires.
- R6. `open_workspace` must return a readable `Err` (no panic) when the path does not exist or is not a directory, and must not change the current workspace in that case.
- R7. The renderer must provide an accessible control to open a workspace using the OS folder picker, call `open_workspace`, and display the opened workspace's `name` and `root` quietly. Before any workspace is open, it must show a calm "no workspace" state.
- R8. Opening a workspace must not read, list, or parse any file inside it, must not perform git or network operations, and must not write to the workspace.
- R9. Any new Tauri capability added (folder picker dialog) must be the minimum required; no broad filesystem read/write capability may be granted to the renderer in this story. Workspace-content access remains a later spec.
- R10. This story must not change the FS-001 runtime status contract and must not remove or break any existing command (`parse_command`, `route_command`, `list_specs_index`, `get_static_spec_detail`, `get_provider_status`, `ask_model`, `ask_spec`, `ask_stream`).

## 3. Acceptance criteria

- AC1. `open_workspace` on a real directory returns a `Workspace` whose `root` is the canonical absolute path and whose `name` is the folder name, and `get_workspace` then returns it.
- AC2. `open_workspace` on a non-existent path or a file (not a directory) returns `Err` with a readable message and leaves any previously open workspace unchanged.
- AC3. `is_within_root` returns true for the root itself and a nested path, and false for a `..` traversal, a sibling directory, and an unrelated absolute path.
- AC4. The renderer's open control invokes the OS folder picker and, on selection, displays the workspace name and root; cancelling the picker leaves state unchanged.
- AC5. Before opening, the renderer shows a calm no-workspace state.
- AC6. Opening a workspace performs no file read/list/parse, no git, and no network call (verified by source review and the absence of such code paths).
- AC7. Rust unit coverage verifies `is_within_root` (inside, root, traversal, sibling, unrelated) and `Workspace` camelCase serialization; `open_workspace` directory validation is covered using a temp directory and a non-existent path.
- AC8. No code in this story reads workspace file contents, lists directories, performs git or network operations, grants broad filesystem capabilities, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `is_within_root` is pure and fully unit-tested. `open_workspace` validation may be tested against a temporary directory created by the test (directory metadata only, no content reads).

Manual checks:

- Run the app, open a folder via the picker, and confirm its name and path display.
- Cancel the picker and confirm nothing changes.
- Choose a path that is not a directory (if reachable) and confirm a calm error.
- Confirm `src-tauri/capabilities/*.json` adds only the folder-picker dialog permission and no broad filesystem read/write permission.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of an opened workspace shown in the shell.
- Short note confirming the only new capability is the folder-picker dialog, no workspace contents are read, and existing contracts are unchanged.

## 5. Success criteria

- SC1. The user can open a folder and Synth records it as the single jailed workspace root in the trusted core.
- SC2. A reusable path-confinement primitive exists and is unit-tested (ADR-0010).
- SC3. Opening is read-only with respect to workspace contents; no file/git/network access is introduced.
- SC4. Capability expansion is limited to the folder picker.
- SC5. The slice stays story-sized and does not become a file browser, repository reader, or git integration.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Open correctness | Real dir → canonical root + name; bad path → readable error | Rust tests / manual | @BrendanShields |
| Jail primitive | `is_within_root` correct for inside/root/traversal/sibling/unrelated | Rust tests | @BrendanShields |
| Capability containment | Only the folder-picker dialog permission added; no broad fs read/write | Capability diff | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Read-only open | 0 workspace content reads, 0 git, 0 network on open | Source review | @BrendanShields |
| Contract stability | FS-001..FS-011 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A Rust `workspace` module with `open_workspace`, `get_workspace`, managed-state root, and the `is_within_root` jail primitive.
- A typed, camelCase `Workspace` shape.
- The minimum capability (folder-picker dialog) and a renderer control to open and display a workspace.
- Unit tests for the jail primitive, serialization, and directory validation.

### Out of scope

- Reading, listing, watching, or parsing any workspace file or directory contents.
- Detecting or reading repo docs (PRD/specs) from the opened workspace.
- Git status, branch, commit, diff, or any git operation.
- Persisting the workspace across sessions or a recent-workspaces list.
- Policy/approval prompts (Phase 2 policy engine is a later spec).
- Any write to the workspace.

## 8. Technical design

### Rust/Tauri core

Add a `workspace` module. Add the `tauri-plugin-dialog` plugin for the folder picker and initialize it; grant only the folder-picker permission in the default capability.

```text
Workspace { root: String, name: String }   // serde camelCase
open_workspace(path: String) -> Result<Workspace, String>   // async #[tauri::command]
get_workspace() -> Option<Workspace>                          // #[tauri::command]
is_within_root(root: &Path, candidate: &Path) -> bool         // pure
```

`open_workspace` uses `std::fs` to confirm the path exists and is a directory, canonicalizes it, derives `name` from the final component, and stores the `Workspace` in Tauri managed state behind a mutex. `get_workspace` reads that state. `is_within_root` normalizes both paths and checks the root is an ancestor-or-equal of the candidate; it rejects `..` escapes. Register both commands; keep all prior commands.

The renderer uses the dialog plugin's folder picker (directory mode) to obtain a path, then calls `open_workspace`.

### React renderer

Add a quiet workspace surface (for example near the shell header or status): an "Open workspace" control that invokes the folder picker and `open_workspace`, then shows `name` and `root`. Fetch `get_workspace` once on load. Keep copy minimal and calm.

### Styling

Reuse muted/mono styles; add only what is needed for one quiet workspace line and an unobtrusive open control.

## 9. Impact notes

- Data model impact: introduces a `Workspace` IPC shape and process-lifetime managed state; nothing persisted to disk.
- Security/privacy impact: first filesystem boundary crossing; limited to validating a user-chosen directory and storing its path. The jail primitive is established now so later fs access is confined by construction (ADR-0010). Only the folder-picker dialog capability is added.
- Observability impact: opening a workspace can be recorded by the FS-011 session log; no event store yet.
- Performance impact: negligible; one directory stat and canonicalization per open.
- Migration/backward compatibility impact: additive; all prior commands and contracts unchanged.

## 10. Risks and dependencies

- Risk: granting filesystem access too broadly. Mitigation: add only the folder-picker dialog permission; workspace-content reads are a separate, later spec and must use `is_within_root`.
- Risk: path traversal escaping the jail. Mitigation: canonicalization plus the unit-tested `is_within_root` ancestor check, enforced in the core.
- Risk: scope creep into a file browser. Mitigation: this story stores only the root and reads no contents.
- Dependency: FS-011 merged; `tauri-plugin-dialog` for the picker.

## 11. Open questions

None. This slice opens and records a jailed workspace root and provides the confinement primitive; content access is deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-011 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-012-open-workspace`
- Expected implementation PR title: `feat(FS-012): Open a jailed workspace`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-012/amendments/`.
