---
spec_id: FS-039
title: Knowledge capture
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
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
---

# FS-039: Knowledge capture

## 1. Problem statement

The PRD's Phase 7 introduces a knowledge system: durable findings captured as committed markdown the project can revisit (PRD §18). In Synth's layer model, durable knowledge is committed project truth — it belongs in the repo (`docs/`), like specs and ADRs, not in app-local storage. This story adds knowledge capture: save a titled knowledge note to `docs/knowledge/<slug>.md` through the approval gate (ADR-0012), confined to the workspace (ADR-0010), and list the captured notes.

Capture is a gated, confined write — the same discipline as spec/amendment saves. It does not auto-commit, run a model, or push; it writes one markdown file into the workspace on explicit approval.

## 2. Requirements

- R1. The Rust core must expose `request_save_knowledge(slug: String, content: String) -> Result<ApprovalRequest, String>` that validates the slug and content, requires an open workspace, and records a pending approval (reusing the FS-018 store) capturing the target `docs/knowledge/<slug>.md` and content. It must not write anything.
- R2. The slug must be validated by a pure, unit-testable function: lowercase letters, digits, and hyphens only, non-empty, within a length cap, no path separators or traversal. Content must be non-empty and within a length cap.
- R3. `resolve_approval` must, for a pending knowledge save and only when `approved`, write the content to `docs/knowledge/<slug>.md` within the jailed workspace (creating the directory), returning the written path. A denial writes nothing.
- R4. Writing the knowledge file must be a confined, unit-testable function that refuses any path escaping the workspace root (ADR-0010), mirroring the spec/amendment writers.
- R5. The core must expose `list_knowledge() -> Result<Vec<KnowledgeNote>, String>` returning the notes under `docs/knowledge/` (each with `slug`, `title`, `path`); a missing directory yields an empty list. `KnowledgeNote` must serialize in camelCase. The title is derived from the note's first markdown heading, falling back to the slug.
- R6. The renderer must provide a capture form (slug + content) that requests a gated save, show the approval surface, and list the captured notes. Capture/list display is transient renderer state derived from the core.
- R7. This story must not auto-commit/auto-push, must not run a model, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract. It writes only within the workspace, only on approval.

## 3. Acceptance criteria

- AC1. `request_save_knowledge("routing-grammar", "# Routing grammar\n...")` with a workspace open returns an `ApprovalRequest` with `action: save-knowledge`, a command referencing `docs/knowledge/routing-grammar.md`, and records a pending approval; nothing is written.
- AC2. `request_save_knowledge` with an invalid slug (uppercase, spaces, `/`, `..`, empty) or empty content returns `Err` and records nothing.
- AC3. `request_save_knowledge` with no workspace open returns `Err`.
- AC4. `resolve_approval(id, true)` writes `docs/knowledge/<slug>.md` within the workspace and returns its path; `resolve_approval(id, false)` writes nothing.
- AC5. The knowledge writer refuses a slug/path that escapes the workspace root.
- AC6. `list_knowledge` returns the notes under `docs/knowledge/` with slug/title/path (title from the first heading, else the slug); a missing directory yields an empty list.
- AC7. Rust unit coverage verifies slug validation (good/bad), the confined writer (including escape refusal), title derivation, and camelCase serialization.
- AC8. No code in this story auto-commits/pushes, runs a model, writes outside the workspace, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the writer and listing are tested against a temporary directory; slug validation and title derivation are pure. No network or model in tests.

Manual checks:

- Capture a knowledge note, approve, and confirm `docs/knowledge/<slug>.md` exists with the content; confirm it lists.
- Deny a capture and confirm nothing is written.
- Capture with an invalid slug and confirm a calm validation error.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the gated capture and the knowledge list.
- Short note confirming confined write, gated save (no auto-commit/model), and unchanged capabilities.

## 5. Success criteria

- SC1. Durable knowledge can be captured as committed markdown under `docs/knowledge/`, gated and confined.
- SC2. The note list surfaces captured knowledge.
- SC3. Capture writes only within the workspace, only on approval.
- SC4. No auto-commit/push, model call, or new capability is introduced.
- SC5. The slice stays story-sized and does not add tagging, search, linking, or commit-on-save.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Slug safety | invalid/traversal slugs rejected | Rust tests | @BrendanShields |
| Confinement | writer refuses escapes (ADR-0010) | Rust tests | @BrendanShields |
| Gate integrity | nothing written at request time; only on approval | Rust tests / source | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 auto-commit, 0 model, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-038 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `request_save_knowledge` reusing the FS-018 store, with a `save-knowledge` pending action.
- A confined knowledge-file writer and pure slug validation.
- `list_knowledge` with title derivation.
- A renderer capture form (gated) and note list.

### Out of scope

- Auto-committing or pushing captured knowledge (use the existing gated commit/push).
- Tagging, search, backlinks/linking, or a knowledge graph.
- Editing/deleting notes from the UI (filesystem/git handles that).
- Reading note bodies in the UI beyond listing (the doc reader covers reading).
- Model-suggested knowledge capture.

## 8. Technical design

### Rust/Tauri core

In `workspace`: add `is_valid_knowledge_slug(slug) -> bool` (pure), `write_knowledge_file(root, slug, content) -> Result<String, String>` (confined to `docs/knowledge/<slug>.md`, mirroring the spec/amendment writers), `KnowledgeNote { slug, title, path }` (camelCase), `knowledge_title_from(content, slug) -> String` (first `# ` heading or slug), and `list_knowledge_in(root) -> Vec<KnowledgeNote>` plus a `list_knowledge` command.

In `approvals`: add `PendingAction::SaveKnowledge { slug, content }`, `request_save_knowledge` (validates slug + content, requires a workspace, records the pending action; not in the auto-approval allow-list, so it always prompts), and a `resolve_approval` arm that writes via `write_knowledge_file`.

### React renderer

Add a knowledge surface: a capture form (slug + content) that calls `request_save_knowledge` through the existing approval flow, and a list of notes (title · path). Refresh the list after a save resolves.

### Styling

Reuse list/control styles.

## 9. Impact notes

- Data model impact: introduces a `KnowledgeNote` IPC shape and a `save-knowledge` pending action; notes are committed markdown under `docs/knowledge/`.
- Security/privacy impact: a gated, confined workspace write (ADR-0010/ADR-0012); no auto-commit/push, no model, no capability added.
- Observability impact: capture request/approve/deny is recorded in the event log.
- Performance impact: one small file write per capture; a bounded directory scan to list.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: path traversal via the slug. Mitigation: strict slug validation plus the confined writer (ADR-0010), both tested.
- Risk: surprise repo writes. Mitigation: capture is gated (explicit approval) and writes only within the workspace; committing stays a separate gated action.
- Risk: scope creep into a knowledge graph. Mitigation: this slice only captures and lists; tagging/linking/search are deferred.
- Dependency: FS-012 workspace jail, FS-018 gate, FS-025/FS-030 confined-writer pattern.

## 11. Open questions

None. This slice captures and lists committed knowledge notes under the gate; linking, search, and graphs are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-038 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-039-knowledge-capture`
- Expected implementation PR title: `feat(FS-039): Knowledge capture`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-039/amendments/`.
