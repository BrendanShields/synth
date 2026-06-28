# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Synth is

A local-first Tauri desktop harness for agentic software development. The thesis: **the harness is the product** — models are hot-swappable; the value is the scaffold (specs, approvals, tool boundaries, observability) around them. Currently in the early "walking skeleton" phase (FS-001..FS-004 implemented).

## Commands

Package manager is **bun** (tauri.conf.json and README drive everything through it, despite a stray `pnpm-lock.yaml`).

```bash
bun install                                       # deps
bun run dev                                        # vite dev server (http://localhost:1420)
bun run tauri dev                                  # full desktop app
bun run build                                      # tsc + vite build (frontend typecheck + bundle)
bun run test                                       # vitest (frontend, runs once)
bunx vitest run src/runtime.test.ts                # single frontend test file
cargo test --manifest-path src-tauri/Cargo.toml    # Rust core tests
```

There is no separate lint step. `bun run build` is the frontend gate (it runs `tsc`); `cargo test` is the Rust gate. Run all three before considering an implementation done — every spec lists exactly these as the required automated checks.

## Architecture: the trust boundary is the point

Four layers, and which layer owns a behavior is a hard rule, not a style preference:

```
React renderer (src/)        = quiet visual surface. Owns NO truth.
Rust/Tauri core (src-tauri/) = trusted product kernel. Owns ALL truth and decisions.
External processes           = supervised tool/extension boundary (not built yet)
Repo docs (docs/)            = committed project truth
```

Concretely:
- **The Rust core decides; React renders.** Command classification, routing dispositions, runtime status — all computed in Rust as pure, unit-tested functions (`parse_raw_command`, `route_raw_command`, `bootstrap_runtime_status`). React calls them over IPC and displays the result. Never reimplement a Rust decision in TypeScript. `src/runtime.ts` holds only formatting/display helpers and mirrored types.
- **IPC contract is camelCase.** Rust structs use `#[serde(rename_all = "camelCase")]`; enums use `rename_all = "lowercase"` or `"kebab-case"`. The TS types in `App.tsx`/`runtime.ts` are hand-mirrored from these — keep both sides in sync, and the serde tests in each Rust module exist specifically to lock the wire shape.
- **Tauri commands** are registered in `src-tauri/src/lib.rs` via `generate_handler!`. Each lives in its own module (`command_dock.rs`, `runtime_status.rs`) with a thin `#[tauri::command]` wrapper over a pure function that does the real work and carries the tests.
- **Privilege is gated by capabilities.** `src-tauri/capabilities/*.json` defines what the app may touch. Adding filesystem/shell/network/credential access means editing these — current specs explicitly forbid it, and the shell (`!`) command kind is deliberately `blocked` because approvals don't exist yet.

## How work is structured: spec-to-PR, planning-gated

This repo is documents-first. The workflow below is not optional and governs every code-changing task — follow it before writing code.

### Classify the request first (PRD §6)

- **Question / explanation** ("how does routing work?") — no spec. Inspect and answer. Offer to capture durable findings as an ADR/knowledge doc only if warranted.
- **Component-level change** ("refactor this hook", "add loading state") — a spec is required before edits, scoped to the component but still carrying the six non-negotiable sections.
- **Project-level work** ("add auth", "introduce a workflow engine") — requires the merged planning baseline (already in place: `docs/PRD.md`, `docs/engineering/ERD.md`, `docs/adrs/`).

If classification is ambiguous, state how you're treating it and why before proceeding.

### Two PRs per feature spec, merged in order

Every feature `FS-NNN` ships as **two separate PRs**, and **each must be merged before the next step begins** — this is the central discipline. The PR history (`gh pr list`) shows the pattern for FS-001..FS-005:

1. **Spec PR (first).**
   - Branch: `docs/fs-NNN-<slug>`
   - Adds only `docs/specs/FS-NNN/spec.md` (copy `docs/templates/feature-spec.md`, strip the instructional text).
   - Commit: `spec(FS-NNN): <title> spec`.
   - PR title: `spec(FS-NNN): <title> spec`.
   - The spec must define all six non-negotiable sections (problem statement, requirements, acceptance criteria, tests/verification plan, success criteria, metrics) and its §12 declares the impl branch + expected impl PR title. **Do not start implementation until this PR is reviewed and merged.**

2. **Implementation PR (second, only after the spec PR is merged).**
   - Branch: `synth/fs-NNN-<slug>` (the name the spec's §12 declared).
   - Adds the code + tests. Implement to the spec *exactly*, including honoring its "out of scope" list — specs are deliberately narrow to avoid crossing trust boundaries prematurely.
   - Commits: `feat(FS-NNN): ...`, `test(FS-NNN): ...`.
   - PR title: `feat(FS-NNN): <title>`.
   - PR description should summarize what changed, the verification results, and scope containment.
   - All three verification checks must pass before marking ready: `bun run build`, `bun run test`, `cargo test --manifest-path src-tauri/Cargo.toml`. Confirm no new privileged Tauri capabilities were added (`src-tauri/capabilities/*.json`) unless the spec scoped them.

Do not begin the next spec's work until the current spec's implementation PR is merged. Keep unrelated work out of a spec PR.

### Specs are immutable once merged

A merged spec is a contract. If implementation reveals it's wrong, incomplete, too broad, or blocked, **pause and write an amendment** in `docs/specs/FS-NNN/amendments/AMD-NNN.md` (template: `docs/templates/amendment.md`) — never edit the approved spec in place. Amendments are always approval-gated, in supervised and high-autonomy mode alike.

### Branch / PR / commit quick reference

| Phase | Branch | PR title | Commits |
| --- | --- | --- | --- |
| Spec | `docs/fs-NNN-<slug>` | `spec(FS-NNN): <title> spec` | `spec(FS-NNN):` |
| Implementation | `synth/fs-NNN-<slug>` | `feat(FS-NNN): <title>` | `feat(FS-NNN):` / `test(FS-NNN):` |
| Amendment | (new branch off a spec) | references the FS-NNN spec | `docs:` |

The SessionStart hook (`.claude/hooks/synth-next.sh`) computes the current pipeline state and the next action from git + `gh` each session — trust it over any state hardcoded here. The plan/backlog lives in `.synth/tasks.json`; add the next spec there (`"planned": true`) before writing it.

## Skills available in this repo

Project skills live in `.claude/skills/` and are vendored from upstream (tracked by content hash in `skills-lock.json`) — treat them as read-only and update them through their source, not by hand-editing. Reach for them as follows:

- **`tauri-v2`** — Tauri v2 work: editing `tauri.conf.json`, adding `#[tauri::command]` handlers, IPC patterns (`invoke`/`emit`/channels), and especially `src-tauri/capabilities/*.json` permission changes. Consult it before touching the Tauri/capability layer.
- **`rust-best-practices`** — writing or refactoring any Rust in `src-tauri/` (ownership/borrowing, `Result` error handling, idiomatic structure). The core's pure-function-plus-thin-command-wrapper pattern should stay idiomatic; this is the reference.
- **`rust-async-patterns`** — only when async/Tokio enters the core (provider streaming, the agent loop — Phase 1+). The current code is synchronous, so this is forward-looking.
- **`skill-creator`** — authoring or improving a skill itself; not part of feature implementation.

These cover the Rust + Tauri stack the trusted core is built on. The `tauri-v2` skill is the most load-bearing here because capability/IPC mistakes cross the trust boundary the whole product depends on.

## Key files

- `src-tauri/src/lib.rs` — Tauri entrypoint + command registration
- `src-tauri/src/command_dock.rs` — command parsing + routing (the `/ ? @ # ! >` prefix grammar)
- `src-tauri/src/runtime_status.rs` — runtime status snapshot + event emission
- `src/App.tsx` — the entire renderer shell (document UI + command dock)
- `src/runtime.ts` — TS-side formatting helpers and IPC type mirrors
- `docs/adrs/` — the binding architectural decisions; read the relevant ADRs a spec links before implementing it
