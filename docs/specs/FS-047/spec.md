---
spec_id: FS-047
title: Knowledge drift detection
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
---

# FS-047: Knowledge drift detection

## 1. Problem statement

The PRD's Phase 7 calls for code/doc drift detection (PRD §18): captured knowledge goes stale when the code it describes moves or is deleted. Knowledge notes (FS-039) routinely reference repository files (e.g. `src/runtime.ts`, `docs/PRD.md`); when those paths no longer exist, the note has drifted. This story detects that drift deterministically: extract referenced repo paths from each note and report the ones that no longer exist in the workspace.

This is read-only and local: it reads notes and checks path existence within the jail. It changes nothing and runs no model.

## 2. Requirements

- R1. Extracting referenced repository paths from note content must be a pure, unit-testable function `extract_referenced_paths(content) -> Vec<String>` that returns workspace-relative-looking paths (containing `/` and a file extension), excluding URLs and absolute paths, de-duplicated.
- R2. The core must expose `detect_knowledge_drift() -> Result<Vec<DriftFinding>, String>` that, for each knowledge note, extracts referenced paths and reports those that do not exist within the jailed workspace. `DriftFinding` must serialize camelCase with at least `slug`, `title`, and `missingPath`.
- R3. Existence checks must be confined to the workspace root (ADR-0010); a path that would escape the root is treated as not-applicable (skipped), never followed.
- R4. With no workspace open, `detect_knowledge_drift` returns a readable `Err`. A note with no referenced paths, or whose references all exist, yields no findings.
- R5. The detection must be read-only (no writes, no git/network/model) and bounded (note count/size capped via the existing reader).
- R6. The renderer must provide a control to check drift and show the findings (note title + missing path), with a calm empty state when there is no drift. Drift display must be transient renderer state.
- R7. This story must not modify notes, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `extract_referenced_paths` returns `src/app.tsx` from text mentioning it, excludes `https://example.com/x`, excludes `/etc/passwd` (absolute), and de-duplicates repeats.
- AC2. `detect_knowledge_drift` reports a `DriftFinding` for a note referencing a non-existent path and reports nothing for a note whose referenced path exists.
- AC3. A path that escapes the workspace root is skipped (not reported, not followed).
- AC4. No workspace open returns `Err`.
- AC5. Detection is read-only; notes are unchanged after a check.
- AC6. The renderer shows findings (title + missing path) and a calm empty state.
- AC7. Rust unit coverage verifies `extract_referenced_paths` (inclusion, URL/absolute exclusion, de-dup), drift detection over a temp workspace (missing vs existing), escape-skipping, and camelCase serialization.
- AC8. No code in this story modifies notes, performs git/network/model I/O, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: extraction is pure; detection is tested against a temporary workspace with a knowledge note and a present/absent referenced file. No network or model.

Manual checks:

- Capture a note referencing an existing file and a non-existent one; run drift and confirm only the missing one is reported.
- Confirm no findings when all references exist.
- Confirm notes are unchanged and `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of a drift finding and the empty state.
- Short note confirming read-only detection, jail confinement, and unchanged capabilities.

## 5. Success criteria

- SC1. Knowledge notes referencing moved/deleted files are flagged as drift.
- SC2. Detection is deterministic, read-only, and jail-confined.
- SC3. False positives are limited by conservative path extraction (URLs/absolute excluded).
- SC4. No note modification or new capability is introduced.
- SC5. The slice stays story-sized — path-existence drift; semantic/content drift is deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Extraction precision | repo paths in; URLs/absolute out; de-duped | Rust tests | @BrendanShields |
| Drift correctness | missing reported, existing not | Rust tests | @BrendanShields |
| Confinement | escaping paths skipped | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 note writes, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-046 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Pure `extract_referenced_paths` and `detect_knowledge_drift` over knowledge notes.
- A `DriftFinding` camelCase shape.
- A renderer drift-check control and findings/empty surface.

### Out of scope

- Semantic/content drift (whether the prose still matches the code), model-judged staleness.
- Auto-fixing notes, updating paths, or citation rewriting.
- Drift over specs/ADRs/other docs (only `docs/knowledge/`).
- Git-history-aware move detection (renames).

## 8. Technical design

### Rust/Tauri core

In `knowledge`, add:

```text
DriftFinding { slug, title, missingPath }                  // serde camelCase
extract_referenced_paths(content) -> Vec<String>            // pure
detect_knowledge_drift(workspace) -> Result<Vec<DriftFinding>, String>   // command
```

`extract_referenced_paths` tokenizes on whitespace and markdown delimiters, strips backticks/parens/punctuation, and keeps tokens that look like relative repo paths (contain `/`, have a `.<ext>` suffix, are not URLs or absolute). `detect_knowledge_drift` reads the notes (FS-044 reader), and for each extracted path checks `is_within_root` and existence under the workspace root, reporting missing ones.

### React renderer

Add a "Check drift" control (in the knowledge surface) that calls `detect_knowledge_drift` and lists findings (title → missing path), with an empty state.

### Styling

Reuse list/notice styles.

## 9. Impact notes

- Data model impact: introduces a `DriftFinding` IPC shape; no persisted entities.
- Security/privacy impact: read-only, jail-confined existence checks; no writes, model, network, or capability.
- Observability impact: surfaces stale knowledge for maintenance.
- Performance impact: a bounded scan over notes and their referenced paths.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: false positives from non-path tokens. Mitigation: conservative extraction (require `/` + extension, exclude URLs/absolute); tested.
- Risk: following escaping paths. Mitigation: `is_within_root` gate before existence checks.
- Risk: missing real drift (semantic). Mitigation: scoped to path-existence drift; semantic drift is deferred.
- Dependency: FS-039 capture, FS-044 knowledge reader, FS-012 jail.

## 11. Open questions

None. This slice detects path-existence drift in knowledge notes; semantic drift and auto-fix are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-046 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-047-knowledge-drift`
- Expected implementation PR title: `feat(FS-047): Knowledge drift detection`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-047/amendments/`.
