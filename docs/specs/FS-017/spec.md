---
spec_id: FS-017
title: Read-only git log
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

# FS-017: Read-only git log

## 1. Problem statement

FS-016 reports the current branch and clean/dirty state, but Synth cannot yet see the repository's recent history. Commits are first-class artifacts in the PRD (§18), and the spec-to-PR loop reasons about what has already landed. The next safe, read-only git slice is recent commit history.

This story adds a read-only `git log` surface under the same constraints as ADR-0011: the system `git` CLI, run in the jailed workspace root, with a fixed read-only argument set and no shell. It returns a bounded list of recent commits (short hash + subject). It performs no mutation, no network, and reads nothing user/model-supplied as git arguments.

## 2. Requirements

- R1. The Rust core must expose a Tauri command `git_log() -> Result<Vec<GitCommit>, String>` returning the open workspace's most recent commits, newest first.
- R2. `GitCommit` must serialize in camelCase with at least: `short` (abbreviated hash) and `subject` (commit summary line).
- R3. Git must be invoked with working directory set to the open workspace root and a hard-coded read-only argument list bounded by a maximum count (e.g. `log --max-count=20 --pretty=format:%h %s`). No shell, no user/model-supplied git arguments.
- R4. If no workspace is open, `git_log` must return a readable `Err` and must not invoke git.
- R5. If the workspace is not a git repository or has no commits yet, `git_log` must return an empty list (not an error).
- R6. Any other git failure (e.g. missing binary) must return a readable `Err` without panicking.
- R7. Parsing the log output into commits must be a pure, unit-testable function with no process/filesystem dependency, capped at a fixed maximum.
- R8. This story must not run any mutating git command, write to the workspace, perform network operations, or add new Tauri capability permissions.
- R9. The renderer must, when a workspace is a git repository, display the recent commits as a quiet list, with a calm empty state otherwise. Display must be transient renderer state only.
- R10. This story must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. With a git workspace, `git_log` returns recent commits newest-first, each with a short hash and subject.
- AC2. The list is bounded by the maximum count.
- AC3. With a non-git folder or an empty repository, `git_log` returns an empty list, no error.
- AC4. With no workspace open, `git_log` returns `Err` and invokes no git.
- AC5. The parser splits each line into short hash and subject, skips malformed lines, and caps the list.
- AC6. The renderer shows the recent commits quietly, with a calm empty state.
- AC7. Rust unit coverage verifies parsing (normal, malformed, cap) and camelCase serialization.
- AC8. No code in this story runs a mutating git command, writes, performs network operations, adds a capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the parser is tested against representative captured log output. No git network operations in tests.

Manual checks:

- Open the Synth repo as a workspace and confirm recent commit subjects display.
- Open a non-git folder and confirm an empty list, not an error.
- Confirm `src-tauri/capabilities/*.json` is unchanged and only read-only git arguments are used.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of recent commits for an opened repo.
- Short note confirming only read-only, fixed git arguments are used, no mutation/network, and capabilities are unchanged.

## 5. Success criteria

- SC1. Synth shows the opened repository's recent commit history.
- SC2. Git log is read-only, jailed, fixed-argument, and bounded.
- SC3. Non-repository and empty-repository cases degrade calmly.
- SC4. No mutation, network, or new capability is introduced.
- SC5. The slice stays story-sized and does not show diffs or perform any mutation.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Log correctness | recent commits parsed newest-first, bounded | Parser tests / manual | @BrendanShields |
| Read-only safety | only fixed read-only git args; no shell; jailed cwd | Source review | @BrendanShields |
| Graceful degradation | non-repo / empty repo → empty list; missing git → error | Rust tests / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Capability containment | 0 new Tauri capabilities; 0 mutating git commands | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-016 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `GitCommit` camelCase shape and a `git_log` command running read-only, bounded git log, jailed.
- A pure log parser (short hash + subject, capped) with unit tests.
- Typed handling of non-repo/empty-repo cases.
- A quiet renderer recent-commits list.

### Out of scope

- Diffs, file changes per commit, or full commit metadata (author/date/body).
- Any mutating git command or remote operation.
- Branch/tag listing beyond the current branch (FS-016).
- Persistence or policy/approval gating.

## 8. Technical design

### Rust/Tauri core

Extend the `git` module:

```text
GitCommit { short, subject }   // serde camelCase
parse_log(output: &str) -> Vec<GitCommit>   // pure, capped
git_log(state) -> Result<Vec<GitCommit>, String>   // #[tauri::command]
```

`git_log` reads the workspace root (Err if none) and runs `git log --max-count=N --pretty=format:%h %s` with `current_dir` set to the root, no shell. On success it returns `parse_log(stdout)`. Non-repo / no-commits stderr indications return an empty list; other failures return `Err`. `parse_log` splits each non-empty line on the first space into short hash and subject, skips malformed lines, and caps the result.

### React renderer

When a workspace is a git repository, call `git_log` and render the commits as a quiet list (short hash + subject), with a calm empty state. Transient state. The FS-011 session log may note the fetch.

### Styling

Reuse the quiet list/mono styles; add nothing beyond a simple list.

## 9. Impact notes

- Data model impact: introduces a `GitCommit` IPC shape; no persisted entities.
- Security/privacy impact: read-only git per ADR-0011 — fixed args, jailed cwd, no shell, no user/model args, bounded count. No mutation, network, or capability.
- Observability impact: can be noted in the FS-011 session log; no event store yet.
- Performance impact: one bounded git log invocation; negligible.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: unbounded output. Mitigation: `--max-count` and a parser cap.
- Risk: argument interpolation. Mitigation: fixed argument array, no shell, no user/model text.
- Risk: parser fragility on unusual subjects. Mitigation: split on first space only; subjects keep their spaces; malformed lines skipped.
- Dependency: FS-016 merged (git module, workspace root).

## 11. Open questions

None. This slice reports read-only, bounded recent commit history for the opened workspace.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-016 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-017-git-log`
- Expected implementation PR title: `feat(FS-017): Read-only git log`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-017/amendments/`.
