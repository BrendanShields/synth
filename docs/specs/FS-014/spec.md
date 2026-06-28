---
spec_id: FS-014
title: Read a workspace planning document
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
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-014: Read a workspace planning document

## 1. Problem statement

FS-013 tells Synth whether the opened repository has a planning baseline, but the user still cannot see those documents in the shell. Synth is a documents-first product (PRD §4, §19): the natural next step is to read and display the opened repo's planning documents in the reader surface.

This story performs the first jailed file-content read. To keep the filesystem surface tight, the renderer never sends a path: it requests a known document *kind* (`prd` or `erd`), and the trusted core maps that to the conventional confined path, reads it within the jail (ADR-0010) with a size cap, and returns the text. No arbitrary paths, no directory listing, no writes. The renderer shows the document text quietly in a reader view.

## 2. Requirements

- R1. The Rust core must expose a Tauri command `read_workspace_doc(kind: String) -> Result<WorkspaceDoc, String>` where `kind` is one of a fixed allow-list (`prd`, `erd`). The renderer must not pass a filesystem path.
- R2. The core must map each allowed `kind` to its conventional repo-relative path (`prd` → `docs/PRD.md`, `erd` → `docs/engineering/ERD.md`) and resolve it against the open workspace root.
- R3. `WorkspaceDoc` must serialize in camelCase with at least: `kind`, `path` (repo-relative), and `text` (the file contents).
- R4. Before reading, the resolved path must be confined to the workspace root with the ADR-0010 `is_within_root` primitive; a path that escapes the root must error and never be read.
- R5. The core must read at most a fixed maximum number of bytes (a size cap) and return text; a file larger than the cap must be truncated to the cap, never read unbounded.
- R6. An unknown `kind`, no open workspace, or a missing/unreadable file must return a readable `Err` with no panic and no partial/invalid result.
- R7. Mapping `kind` to a confined relative path and enforcing the allow-list must be pure, unit-testable logic separate from the filesystem read.
- R8. The command must read only the single mapped file. It must not list directories, read other files, follow links outside the root, perform git, or perform network operations, and it must not write to the workspace.
- R9. The renderer must let the user view an available planning document (for example via a control shown when the baseline reports it present) and render its text in a calm reader surface, with a readable error state on failure. The reader state must be transient renderer state only.
- R10. This story must not add new Tauri capability permissions (the read uses core-side `std::fs`), must not write to the workspace, and must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. With a workspace open whose `docs/PRD.md` exists, `read_workspace_doc("prd")` returns a `WorkspaceDoc` with `kind: "prd"`, `path: "docs/PRD.md"`, and the file's text.
- AC2. `read_workspace_doc("erd")` reads `docs/engineering/ERD.md` similarly.
- AC3. `read_workspace_doc("secrets")` (or any value not in the allow-list) returns `Err` and reads nothing.
- AC4. With no workspace open, the command returns `Err` and reads nothing.
- AC5. A file larger than the size cap is returned truncated to the cap, not read unbounded.
- AC6. The pure mapping function returns the confined relative path for allowed kinds and rejects unknown kinds; a path that would escape the root is rejected.
- AC7. The renderer renders the returned document text in a calm reader surface and shows a readable error if the read fails.
- AC8. Rust unit coverage verifies kind→path mapping and allow-list rejection, confined reading against a temporary workspace (including truncation), and camelCase serialization.
- AC9. No code in this story lists directories, reads files outside the allow-list, performs git or network operations, adds a Tauri capability, writes to the workspace, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: reading is tested against a temporary workspace created by the test (writing a small file and a larger-than-cap file). No network I/O.

Manual checks:

- Open the Synth repo as a workspace and view the PRD; confirm its text renders.
- View the ERD and confirm its text renders.
- Confirm an unreadable/missing document shows a calm error.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of a rendered workspace document.
- Short note confirming only allow-listed, confined documents are read, the size cap is enforced, and capabilities are unchanged.

## 5. Success criteria

- SC1. The user can read the opened repository's planning documents in the shell.
- SC2. Reads are confined to the jail and limited to an allow-list of known documents.
- SC3. Reads are size-capped and degrade calmly on error.
- SC4. No new capability and no writes are introduced.
- SC5. The slice stays story-sized and does not become a file browser, arbitrary file reader, or markdown editor.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Read correctness | prd/erd return the right confined text | Rust tests / manual | @BrendanShields |
| Allow-list + jail | Unknown kind and escape rejected; only mapped file read | Rust tests / source | @BrendanShields |
| Size cap | Over-cap file truncated, never read unbounded | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Capability containment | 0 new Tauri capabilities | Capability diff | @BrendanShields |
| Contract stability | FS-001..FS-013 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `WorkspaceDoc` camelCase shape and a `read_workspace_doc(kind)` command with a fixed allow-list.
- Pure kind→confined-path mapping with unit tests.
- A confined, size-capped read of the single mapped file.
- A calm renderer reader surface for an available planning document.

### Out of scope

- Reading arbitrary paths or any file outside the `prd`/`erd` allow-list.
- Directory listing, spec scanning, or ADR/CODEOWNERS reading.
- Markdown rendering/formatting beyond plain text (no markdown engine).
- Editing, writing, or saving documents.
- Git operations or persistence of read content.

## 8. Technical design

### Rust/Tauri core

Extend the `workspace` module:

```text
WorkspaceDoc { kind, path, text }   // serde camelCase
workspace_doc_path(kind: &str) -> Option<&'static str>   // pure: allow-list mapping
read_workspace_doc(state, kind: String) -> Result<WorkspaceDoc, String>   // #[tauri::command]
```

`workspace_doc_path` returns the conventional relative path for `prd`/`erd` and `None` otherwise. `read_workspace_doc` reads the workspace root from managed state (Err if none), maps the kind (Err if unknown), joins and confirms `is_within_root`, then reads the file with a fixed byte cap (e.g., open and read up to N bytes, lossy-decoding to text). All failure paths return readable `Err`.

### React renderer

When the baseline reports a document present, show a quiet control to view it; on activation call `read_workspace_doc(kind)` and render `text` in a calm reader surface (preformatted, no markdown engine) with an error state. Keep it transient.

### Styling

Reuse prose/mono styles; add a simple scrollable, low-contrast reader block. No syntax highlighting, tabs, or chrome.

## 9. Impact notes

- Data model impact: introduces a `WorkspaceDoc` IPC shape; no persisted entities.
- Security/privacy impact: first file-content read; constrained to an allow-list of two confined paths, jailed by `is_within_root` and size-capped. No arbitrary paths (renderer sends a kind, not a path); no capability added; no writes.
- Observability impact: a read can be noted in the FS-011 session log; no event store yet.
- Performance impact: one capped read per view; negligible.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: arbitrary-file-read via a path argument. Mitigation: the renderer passes a `kind` from a fixed allow-list, never a path; the core maps it.
- Risk: reading a huge file. Mitigation: a fixed byte cap with truncation.
- Risk: path escaping the jail. Mitigation: `is_within_root` before the read.
- Risk: scope creep into a file browser/editor. Mitigation: allow-list of two read-only documents, plain text only.
- Dependency: FS-013 merged (workspace root, baseline, `is_within_root`).

## 11. Open questions

None. This slice reads two allow-listed, confined, size-capped planning documents for display.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-013 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-014-read-workspace-doc`
- Expected implementation PR title: `feat(FS-014): Read a workspace planning document`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-014/amendments/`.
