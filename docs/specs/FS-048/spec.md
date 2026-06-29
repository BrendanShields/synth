---
spec_id: FS-048
title: Knowledge links
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

# FS-048: Knowledge links

## 1. Problem statement

The PRD's Phase 7 calls for a project knowledge graph (PRD §18). The first structure a graph needs is edges: links between knowledge notes. This story adds `[[slug]]` wikilinks — the same lightweight linking convention used elsewhere — between knowledge notes (FS-039), and exposes the resulting link graph, marking which links resolve to an existing note and which dangle. It is the edge layer the knowledge graph builds on.

This is read-only and deterministic: it parses links from note content and resolves them against the note set. It writes nothing and runs no model.

## 2. Requirements

- R1. Parsing links from note content must be a pure, unit-testable function `extract_note_links(content) -> Vec<String>` that returns the slugs inside `[[...]]` markers, trimmed and de-duplicated, ignoring empty/invalid link targets.
- R2. The core must expose `knowledge_links() -> Result<Vec<NoteLink>, String>` that, for each knowledge note, emits a `NoteLink` per outgoing link with `from` (source slug), `to` (target slug), and `resolved` (whether a note with the target slug exists). `NoteLink` must serialize camelCase.
- R3. Resolution must be against the set of existing knowledge notes (by slug); an unresolved link has `resolved: false` (a dangling link), not an error.
- R4. With no workspace open, `knowledge_links` returns a readable `Err`. A note set with no links yields an empty list.
- R5. The computation must be read-only over the jailed workspace (no writes, no git/network/model) and bounded by the existing knowledge reader.
- R6. The renderer must provide a control to view the links and show them (`from → to`, marking unresolved/dangling links), with a calm empty state. Link display must be transient renderer state.
- R7. This story must not modify notes, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `extract_note_links("see [[routing-grammar]] and [[ misc ]] and [[]]")` returns `["routing-grammar", "misc"]` (trimmed, empties ignored).
- AC2. Duplicate links in one note are de-duplicated.
- AC3. `knowledge_links` emits a resolved `NoteLink` when the target note exists and an unresolved one when it does not.
- AC4. No workspace open returns `Err`; a note set with no links returns an empty list.
- AC5. The computation is read-only; notes are unchanged afterward.
- AC6. The renderer shows links (`from → to`) and marks unresolved ones, with an empty state.
- AC7. Rust unit coverage verifies `extract_note_links` (extraction, trim, de-dup, empty-ignore), link resolution over an in-memory/temp note set (resolved + dangling), and camelCase serialization.
- AC8. No code in this story modifies notes, performs git/network/model I/O, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: extraction is pure; resolution is tested over a temporary knowledge set. No network or model.

Manual checks:

- Capture two notes where one links `[[the-other-slug]]`; view links and confirm a resolved edge.
- Link a non-existent slug and confirm it shows as unresolved.
- Confirm notes are unchanged and `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of resolved and dangling links.
- Short note confirming read-only computation, jail confinement, and unchanged capabilities.

## 5. Success criteria

- SC1. Knowledge notes can link to each other via `[[slug]]`, exposed as a link graph.
- SC2. Dangling links are surfaced (resolved flag), not errors.
- SC3. The computation is deterministic, read-only, and jail-confined.
- SC4. No note modification or new capability is introduced.
- SC5. The slice stays story-sized — link edges + resolution; graph visualization and backlink navigation are deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Link parsing | `[[slug]]` extracted, trimmed, de-duped, empties ignored | Rust tests | @BrendanShields |
| Resolution | existing → resolved; missing → dangling | Rust tests | @BrendanShields |
| Read-only | notes unchanged after computation | Source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 note writes, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-047 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Pure `extract_note_links` and `knowledge_links` resolution.
- A `NoteLink` camelCase shape.
- A renderer link-view control showing resolved/dangling edges.

### Out of scope

- Graph visualization (nodes/edges layout), backlink navigation, or clustering.
- Auto-creating notes for dangling links, or link auto-completion.
- Links to non-knowledge targets (specs/ADRs/files).
- Ranking or traversal/pathfinding over the graph.

## 8. Technical design

### Rust/Tauri core

In `knowledge`, add:

```text
NoteLink { from, to, resolved }                            // serde camelCase
extract_note_links(content) -> Vec<String>                  // pure
links_in(notes_with_content) -> Vec<NoteLink>               // pure resolution
knowledge_links(workspace) -> Result<Vec<NoteLink>, String>  // command
```

`extract_note_links` scans for `[[ ... ]]` spans and returns the trimmed inner slugs (de-duplicated, empties ignored). `links_in` builds the slug set, then for each note emits a `NoteLink` per outgoing link with `resolved = slugs.contains(to)`. `knowledge_links` reads the notes (FS-044 reader) and delegates.

### React renderer

Add a "View links" control (in the knowledge surface) that calls `knowledge_links` and lists edges (`from → to`), marking unresolved ones, with an empty state.

### Styling

Reuse list/notice styles; a subtle marker for dangling links.

## 9. Impact notes

- Data model impact: introduces a `NoteLink` IPC shape; no persisted entities.
- Security/privacy impact: read-only, jail-confined; no writes, model, network, or capability.
- Observability impact: exposes the knowledge link graph — the edge layer for a future graph view.
- Performance impact: a bounded scan over notes and their links.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: noisy `[[` usage. Mitigation: only well-formed `[[slug]]` spans with non-empty trimmed content are links; tested.
- Risk: implying graph navigation exists. Mitigation: this slice exposes edges + resolution only; visualization/backlinks are deferred.
- Dependency: FS-039 capture, FS-044 knowledge reader, FS-012 jail.

## 11. Open questions

None. This slice extracts and resolves `[[slug]]` links between notes; graph visualization and backlink navigation are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-047 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-048-knowledge-links`
- Expected implementation PR title: `feat(FS-048): Knowledge links`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-048/amendments/`.
