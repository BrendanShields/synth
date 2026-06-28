---
spec_id: FS-027
title: Read-only diff review
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
  - docs/adrs/ADR-0011-git-via-cli-readonly-first.md
---

# FS-027: Read-only diff review

## 1. Problem statement

Synth can report git status and log (FS-016/FS-017), but the user cannot see *what changed*. The PRD names a diff review view as a core surface (PRD §19.2) and review is central to the spec-to-PR loop. The first, safe step is read-only: show the working-tree diff against the last commit.

This story adds a read-only `git diff` surface under the ADR-0011 discipline (system `git`, jailed cwd, fixed read-only args, no shell). It returns the diff text (bounded), parsed enough for the renderer to present it calmly with added/removed lines distinguished. It performs no mutation, staging, or apply.

## 2. Requirements

- R1. The Rust core must expose `git_diff() -> Result<GitDiff, String>` returning the open workspace's working-tree diff against `HEAD`.
- R2. `GitDiff` must serialize in camelCase with at least: `isRepo` (boolean), `empty` (boolean, true when there is no diff), and `lines` (an array of `{ kind, text }` where `kind` is `add`, `del`, `meta`, or `context`), bounded by a fixed maximum.
- R3. Git must be invoked with working directory set to the workspace root and a fixed read-only argument list (e.g. `diff HEAD`). No shell, no user/model-supplied git arguments.
- R4. If no workspace is open, `git_diff` must return a readable `Err` and must not invoke git.
- R5. If the workspace is not a git repository, `git_diff` must return `isRepo: false`, `empty: true`, empty `lines`, and no error. If there are no changes, it must return `isRepo: true`, `empty: true`, empty `lines`.
- R6. Parsing the diff text into classified, capped lines must be a pure, unit-testable function (a line starting with `+` but not `+++` is `add`; `-` but not `---` is `del`; `diff`/`@@`/`index`/`+++`/`---` are `meta`; otherwise `context`).
- R7. Other git failures (missing binary, etc.) must return a readable `Err` without panicking. A repository with no commits (no `HEAD`) must degrade to `empty: true`, not an error.
- R8. This story must not run any mutating git command, stage, apply, or write; must not perform network operations; and must not add new Tauri capability permissions.
- R9. The renderer must, when a workspace is a git repository, display the diff in a calm review surface with added/removed lines visually distinguished, an empty state when there are no changes, and a readable error state on failure. Display must be transient renderer state only.
- R10. This story must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. With a git workspace that has uncommitted changes, `git_diff` returns `isRepo: true`, `empty: false`, and classified `lines` including `add`/`del`/`meta`/`context`.
- AC2. With a clean working tree, `git_diff` returns `isRepo: true`, `empty: true`, empty `lines`.
- AC3. With a non-git folder, `git_diff` returns `isRepo: false`, `empty: true`, no error.
- AC4. With no workspace open, `git_diff` returns `Err` and invokes no git.
- AC5. The diff parser classifies a representative unified diff correctly and caps the line list.
- AC6. The renderer shows the diff with added/removed lines distinguished, a calm empty state, and an error state.
- AC7. Rust unit coverage verifies the pure parser (each line kind, `+++`/`---` as meta not add/del, cap) and camelCase serialization.
- AC8. No code in this story runs a mutating git command, stages/applies, performs network operations, adds a capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the parser is tested against a representative unified-diff string. No git network operations.

Manual checks:

- Open the Synth repo, make an uncommitted change, and confirm the diff surface shows added/removed lines.
- Revert the change and confirm a calm empty state.
- Open a non-git folder and confirm the not-a-repo empty state.
- Confirm `src-tauri/capabilities/*.json` is unchanged and only read-only git args are used.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the diff surface with changes and the empty state.
- Short note confirming only read-only `git diff` is used, no mutation/network, and capabilities are unchanged.

## 5. Success criteria

- SC1. Synth shows the working-tree diff for the opened repository.
- SC2. Diff is read-only, jailed, fixed-argument, and bounded; lines are classified for review.
- SC3. Non-repository, no-commits, and clean cases degrade calmly.
- SC4. No mutation, staging, apply, network, or new capability is introduced.
- SC5. The slice stays story-sized and does not stage, apply, comment, or edit.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Diff correctness | classified lines for a real change; empty when clean | Parser tests / manual | @BrendanShields |
| Read-only safety | only read-only `git diff`; no shell; jailed cwd | Source review | @BrendanShields |
| Graceful degradation | non-repo / no-commits / clean handled calmly | Rust tests / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Capability containment | 0 new Tauri capabilities; 0 mutating git | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-026 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `GitDiff` camelCase shape and a `git_diff` command running read-only, bounded `git diff HEAD`, jailed.
- A pure unified-diff parser classifying lines (add/del/meta/context), capped.
- Typed non-repo/no-commits/clean handling.
- A calm renderer diff review surface.

### Out of scope

- Staging, applying, reverting, or editing from the diff.
- Per-file selection, side-by-side view, syntax highlighting, or inline comments.
- Diff of a specific commit/range beyond the working tree vs `HEAD`.
- Mutation, network, or persistence.

## 8. Technical design

### Rust/Tauri core

Extend the `git` module:

```text
GitDiff { is_repo, empty, lines }   // serde camelCase; lines: Vec<DiffLine { kind, text }>
parse_diff(diff: &str) -> Vec<DiffLine>   // pure, capped, classified
git_diff(state) -> Result<GitDiff, String>   // #[tauri::command]
```

`git_diff` reads the workspace root (Err if none) and runs `git diff HEAD` with `current_dir` the root, no shell. On success it returns `parse_diff(stdout)` with `empty` set from whether any lines exist. Non-repo and no-`HEAD` (no commits) stderr indications return `is_repo`/`empty` accordingly without error. `parse_diff` classifies each line and caps the list.

### React renderer

When a workspace is a git repository, call `git_diff` and render the lines in a calm monospace review surface, distinguishing `add`/`del` (restrained color), with an empty state and an error state. Transient state; refresh on demand (e.g. when git status refreshes).

### Styling

Reuse the reader/mono styles; add restrained add/remove line styling. No syntax highlighting or side-by-side.

## 9. Impact notes

- Data model impact: introduces a `GitDiff`/`DiffLine` IPC shape; no persisted entities.
- Security/privacy impact: read-only git per ADR-0011; fixed args, jailed cwd, no shell, bounded output. No mutation/network/capability.
- Observability impact: can be noted in the FS-011 session log.
- Performance impact: one bounded git diff invocation; negligible for typical changes.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: huge diffs. Mitigation: a parser cap on lines.
- Risk: argument interpolation. Mitigation: fixed argument array, no shell, no user/model text.
- Risk: misclassifying `+++`/`---` headers as add/del. Mitigation: the parser checks `+++`/`---` as meta first; tested.
- Dependency: FS-016 merged (git module, workspace root).

## 11. Open questions

None. This slice shows a read-only, classified, bounded working-tree diff for review; staging/applying/commenting are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-026 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-027-git-diff`
- Expected implementation PR title: `feat(FS-027): Read-only diff review`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-027/amendments/`.
