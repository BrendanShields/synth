---
spec_id: FS-016
title: Read-only git status
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

# FS-016: Read-only git status

## 1. Problem statement

Synth can open a repository and read its documents, but it cannot yet see the repository's git state. The PRD treats branches, commits, and PRs as first-class artifacts (PRD §18) and the V1 loop implements specs on their own branches. The first, safe step is observation: report the current branch and whether the working tree is clean or dirty.

This story adds a read-only git surface (ADR-0011): the trusted core runs the system `git` CLI with a fixed, read-only argument set, working directory set to the jailed workspace root (ADR-0010), and returns a typed status. It performs no mutation (no branch/commit/push), passes no user- or model-supplied text as git arguments, and invokes no shell. A non-repository or missing `git` is reported as a typed state, not an error.

## 2. Requirements

- R1. The Rust core must expose a Tauri command `git_status() -> Result<GitStatus, String>` that, for the open workspace, runs the system `git` CLI read-only and returns the repository status.
- R2. `GitStatus` must serialize in camelCase with at least: `isRepo` (boolean), `branch` (string; empty when unknown/detached), `clean` (boolean), and `changes` (array of short change descriptors, each a porcelain status line).
- R3. Git must be invoked with its working directory set to the open workspace root and with a hard-coded, read-only argument list (e.g. `status --porcelain=v1 --branch`). No shell may be used, and no user- or model-supplied string may be passed as a git argument.
- R4. If no workspace is open, `git_status` must return a readable `Err` and must not invoke git.
- R5. If the workspace is not a git repository, `git_status` must return `isRepo: false` (with `clean: true`, empty `branch`, empty `changes`), not an error.
- R6. If the `git` binary is missing or the invocation fails for a non-"not-a-repo" reason, `git_status` must return a readable `Err` without panicking.
- R7. Parsing git porcelain output into branch, clean flag, and changes must be a pure, unit-testable function with no process or filesystem dependency. The `changes` list must be capped at a fixed maximum to bound output.
- R8. This story must not run any mutating git command (branch, add, commit, checkout, push, etc.), must not write to the workspace, must not perform network operations, and must not add new Tauri capability permissions (git runs as a core-side child process, not via a renderer capability).
- R9. The renderer must, when a workspace is open, display the git status quietly: the branch and clean/dirty state, and a calm "not a git repository" state when applicable. Status display must be transient renderer state only.
- R10. This story must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. With a workspace that is a git repository, `git_status` returns `isRepo: true`, the current `branch`, `clean` reflecting the working tree, and `changes` listing modified/untracked entries.
- AC2. With a clean repository, `clean` is true and `changes` is empty; with uncommitted changes, `clean` is false and `changes` is non-empty.
- AC3. With a workspace that is not a git repository, `git_status` returns `isRepo: false`, `clean: true`, empty `branch` and `changes`, and no error.
- AC4. With no workspace open, `git_status` returns `Err` and does not invoke git.
- AC5. The porcelain parser extracts the branch from the `## branch...upstream` header, sets `clean` from the presence of change lines, lists changes, and caps the list at the maximum.
- AC6. The renderer shows the branch and clean/dirty state quietly, and a calm not-a-repo state when applicable.
- AC7. Rust unit coverage verifies the pure parser (branch extraction, clean vs dirty, cap) and camelCase serialization. No test invokes a live network or depends on a specific external repo.
- AC8. No code in this story runs a mutating git command, writes to the workspace, performs network operations, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the porcelain parser is tested against representative captured output strings (clean, dirty, detached/no-branch). `cargo test` performs no git network operations.

Manual checks:

- Open the Synth repo as a workspace and confirm the branch and clean/dirty state display.
- Make an uncommitted change and confirm the status shows dirty with the change listed.
- Open a non-git folder and confirm the calm not-a-repository state.
- Confirm `src-tauri/capabilities/*.json` is unchanged and that only read-only git arguments are used (source review).

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of git status for an opened repository (clean and dirty).
- Short note confirming only read-only, fixed git arguments are used, no mutation/network, and capabilities are unchanged.

## 5. Success criteria

- SC1. Synth reports the opened repository's branch and clean/dirty state.
- SC2. Git is invoked read-only, jailed to the workspace, with fixed arguments and no shell.
- SC3. Non-repository and missing-git cases degrade calmly.
- SC4. No mutation, network, or new capability is introduced.
- SC5. The slice stays story-sized and does not perform branch/commit/push or diffs.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Status correctness | branch + clean/dirty + changes correct on a real repo | Manual / parser tests | @BrendanShields |
| Read-only safety | only fixed read-only git args; no shell; jailed cwd | Source review | @BrendanShields |
| Graceful degradation | not-a-repo → typed state; missing git → readable error | Rust tests / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Capability containment | 0 new Tauri capabilities; 0 mutating git commands | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-015 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `GitStatus` camelCase shape and a `git_status` command running read-only git, jailed.
- A pure porcelain parser (branch, clean, capped changes) with unit tests.
- Typed handling of not-a-repo and missing-git cases.
- A quiet renderer git-status surface.

### Out of scope

- Any mutating git command (branch, add, commit, checkout, merge, push, tag).
- Diffs, file-level diff rendering, or commit history beyond branch/status.
- Remote operations, fetch/pull/push, or PR creation.
- Passing user/model text to git, or arbitrary git command execution.
- Persistence of git state or policy/approval gating (Phase 2 policy engine).

## 8. Technical design

### Rust/Tauri core

Add a `git` module (ADR-0011):

```text
GitStatus { is_repo, branch, clean, changes }   // serde camelCase
parse_status(porcelain: &str) -> GitStatus       // pure, capped changes
git_status(state) -> Result<GitStatus, String>    // #[tauri::command]
```

`git_status` reads the workspace root from managed state (Err if none) and runs `std::process::Command::new("git").current_dir(root).args(["status", "--porcelain=v1", "--branch"])` with no shell. On success it returns `parse_status(stdout)`. If git exits non-zero with a "not a git repository" indication, it returns a not-a-repo `GitStatus`. A missing binary or other failure returns a readable `Err`. `parse_status` reads the `## ...` header for the branch, treats remaining non-empty lines as changes, sets `clean` from their absence, and caps the list.

### React renderer

When a workspace is open, call `git_status` and render a quiet line: branch and clean/dirty (and a small changed-count or the first few entries), or a calm "not a git repository" state. Keep it transient. The FS-011 session log may note the status check.

### Styling

Reuse muted/mono styles; add a single quiet status line near the workspace surface. No diff view or chrome.

## 9. Impact notes

- Data model impact: introduces a `GitStatus` IPC shape; no persisted entities.
- Security/privacy impact: first external-process execution, introduced narrowly per ADR-0011 — one known binary, fixed read-only arguments, jailed working directory, no shell, no user/model-supplied args. No mutation, no network, no capability added.
- Observability impact: status can be noted in the FS-011 session log; no event store yet.
- Performance impact: one short git invocation per status check; negligible.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: command injection via interpolated arguments. Mitigation: fixed argument array, no shell, no user/model text passed to git.
- Risk: accidental mutation. Mitigation: only read-only subcommands; mutation is out of scope and a later spec.
- Risk: hanging git process. Mitigation: a bounded invocation (and a later spec may add timeouts); read-only `status` is fast.
- Risk: parser fragility. Mitigation: a pure parser tested against representative porcelain output.
- Dependency: FS-015 merged (workspace root); the system `git` binary for manual verification.

## 11. Open questions

None. This slice reports read-only git status for the opened workspace, jailed and fixed-argument.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-015 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-016-git-status`
- Expected implementation PR title: `feat(FS-016): Read-only git status`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-016/amendments/`.
