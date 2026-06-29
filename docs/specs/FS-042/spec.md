---
spec_id: FS-042
title: Session tree foundation
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
  - docs/adrs/ADR-0001-rust-native-runtime.md
---

# FS-042: Session tree foundation

## 1. Problem statement

The PRD describes sessions as trees that can be inspected and replayed (PRD §23) — branches of decisions and actions, not just a flat log. The current event log (FS-032) is linear and cannot express that a decision branched or which path led to a node. This story adds the session-tree substrate: nodes with parent links, persisted app-local, plus the pure ancestry logic replay needs. It is the foundation true session replay and richer observability build on; the flat event log stays as-is for chronological activity.

This story models and persists the tree and computes ancestry; it does not add replay playback, branching UI, or rewrite history.

## 2. Requirements

- R1. The Rust core must define `SessionNode { id, parentId, kind, label, detail }` (camelCase; `parentId` optional — absent for a root) and serialize/parse one node per line, mirroring the FS-032 record pattern.
- R2. The core must expose `append_session_node(parentId: Option<u64>, kind, label, detail) -> Result<SessionNode, String>` that assigns the next id and appends the node to an app-local store (`{app_data_dir}/session-tree.jsonl`), and `load_session_tree() -> Vec<SessionNode>` returning all nodes in append order.
- R3. Appending must validate that a non-null `parentId` refers to an existing node; an unknown parent is a readable `Err` (roots use `parentId: null`).
- R4. Computing the ancestry path of a node must be a pure, unit-testable function `path_to_root(nodes, id) -> Vec<SessionNode>` returning root→node order; an unknown id yields an empty path; the function must be robust to a missing/broken parent link (it stops rather than looping).
- R5. The store and parsing must tolerate malformed lines (skipped), and a missing store loads as empty.
- R6. The renderer must display the session tree (indented by depth) from `load_session_tree`, and record nodes at natural points (a root when a workspace opens; a child when an approval is approved), tracking the current node. Tree display/current-node are transient renderer state.
- R7. This story must not modify or replace the FS-032 event log, must not add replay playback or branching/editing, must not perform any network operation, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `append_session_node(None, "session", "open", "repo")` returns a root node (`parentId: null`) with an id and persists it; `load_session_tree` includes it.
- AC2. `append_session_node(Some(root_id), ...)` appends a child; `load_session_tree` returns both in append order.
- AC3. `append_session_node(Some(unknown_id), ...)` returns `Err` and appends nothing.
- AC4. `path_to_root(nodes, leaf_id)` returns the root→leaf chain; an unknown id returns an empty vec; a broken parent link terminates the path without looping.
- AC5. A missing store loads empty; malformed lines are skipped.
- AC6. The renderer shows the tree indented by depth and records a root on workspace open and a child on an approved approval.
- AC7. Rust unit coverage verifies append/load round-trip (incl. unknown-parent rejection and malformed tolerance), `path_to_root` (normal, unknown, broken link), and camelCase serialization including `parentId`.
- AC8. No code in this story modifies the FS-032 log, adds replay/branching, performs network I/O, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: append/load are tested against a temporary store file; `path_to_root` is pure. No network in tests.

Manual checks:

- Open a workspace (root appears), approve an action (a child appears under it), and confirm the tree shows indented.
- Confirm the FS-032 event log is unaffected.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the indented session tree.
- Short note confirming the event log is untouched, app-local store, no replay/branching, and unchanged capabilities.

## 5. Success criteria

- SC1. Sessions can be recorded as a tree of parent-linked nodes, persisted app-local.
- SC2. Ancestry (the path replay needs) is computed by a robust pure function.
- SC3. The flat event log is untouched; the tree is additive.
- SC4. No replay/branching, network, or new capability is introduced.
- SC5. The slice stays story-sized — substrate only; playback and branching UI are deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Tree integrity | unknown parent rejected; append order preserved | Rust tests | @BrendanShields |
| Ancestry correctness | path_to_root correct; robust to unknown/broken | Rust tests | @BrendanShields |
| Isolation | FS-032 log unchanged | Source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 replay/branching, 0 network, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-041 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `SessionNode` shape and an app-local `session-tree.jsonl` store.
- `append_session_node` / `load_session_tree` and pure `path_to_root`.
- A renderer indented tree view recording a root (workspace open) and a child (approved approval).

### Out of scope

- Replay playback (stepping through to reconstruct a state), branching/forking UI, or editing/pruning the tree.
- Replacing or migrating the FS-032 flat event log.
- Attaching rich payloads/diffs to nodes, or cross-session/project trees.
- Visualizing the tree graphically beyond an indented list.

## 8. Technical design

### Rust/Tauri core

Add a `session_tree` module mirroring `events`:

```text
SessionNode { id, parentId, kind, label, detail }     // serde camelCase, (de)serializable
serialize_node / parse_node                            // one JSON object per line
append_node(path, node) / load_nodes(path)             // file I/O, malformed-tolerant
path_to_root(nodes: &[SessionNode], id) -> Vec<SessionNode>   // pure, robust to broken links
append_session_node(app, parentId, kind, label, detail) -> Result<SessionNode, String>   // command
load_session_tree(app) -> Vec<SessionNode>             // command
```

`append_session_node` validates the parent (if any) against the loaded nodes, assigns `id = node count`, and appends. `path_to_root` walks `parentId` from the node up to a root, guarding against missing links and cycles (bounded by node count).

### React renderer

Track `currentNodeId`. On workspace open, append a root node and set current. On an approved approval, append a child under current and advance current. Render the tree as an indented list (depth from `path_to_root` length). Keep state transient.

### Styling

Reuse list styles; indent by depth.

## 9. Impact notes

- Data model impact: introduces a `SessionNode` shape and a `session-tree.jsonl` store, additive to the FS-032 log.
- Security/privacy impact: app-local store; no network, no secrets, no capability.
- Observability impact: the substrate for replay and tree-shaped observability.
- Performance impact: a bounded append/read; ancestry is O(depth).
- Migration/backward compatibility impact: additive; the flat event log and all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: cycles/broken links in the tree. Mitigation: append validates parents; `path_to_root` is bounded by node count and stops on a missing link; tested.
- Risk: confusion with the flat event log. Mitigation: the tree is additive and separate; the event log is untouched.
- Risk: scope creep into replay/branching. Mitigation: this slice is substrate only.
- Dependency: FS-032 app-local persistence pattern.

## 11. Open questions

None. This slice provides the session-tree substrate and ancestry; replay playback and branching UI are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-041 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-042-session-tree`
- Expected implementation PR title: `feat(FS-042): Session tree foundation`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-042/amendments/`.
