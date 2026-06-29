---
spec_id: FS-043
title: Session replay
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
---

# FS-043: Session replay

## 1. Problem statement

FS-042 records sessions as a tree of parent-linked nodes. The PRD's Phase 4 calls for session replay (PRD §23): being able to take any point in a session and see the exact path of decisions that led there. With the tree and the pure `path_to_root` ancestry already in place, replay is the read-only step that exposes that path: select a node, get its root→node decision chain. This closes the last Phase 4 gap, built on the real session structure rather than a flat scrubber.

This is read-only: it reconstructs and shows the decision path to a node. It does not re-execute actions, branch, or modify the tree.

## 2. Requirements

- R1. The Rust core must expose `replay_path(nodeId: u64) -> Vec<SessionNode>` returning the root→node ancestry of the given node, computed via the existing pure `path_to_root` over the loaded session tree.
- R2. An unknown `nodeId` must return an empty path (no error); the function must not panic on a broken/looping tree (the underlying pure function is already bounded).
- R3. The command must be read-only: it must not append to or modify the session tree, run any model, or perform git/network/filesystem mutation.
- R4. The renderer must let the user select a node in the session tree and show its replay path (the ordered root→node chain) in a calm surface, with the path clearing when deselected. Replay/selection state must be transient renderer state only.
- R5. This story must not re-execute actions, branch/fork, edit the tree, add new Tauri capability permissions, or change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. For a tree `root → a → b`, `replay_path(b.id)` returns `[root, a, b]` in order.
- AC2. `replay_path(unknown_id)` returns an empty list (no error).
- AC3. `replay_path` does not modify the session tree (the store is unchanged after the call).
- AC4. The renderer shows the replay path when a node is selected and clears it when deselected.
- AC5. Rust unit coverage verifies the ordered path, the unknown-id empty case, and that the command path delegates to the pure ancestry function (covered via that function's tests plus a wrapper test over in-memory nodes).
- AC6. No code in this story re-executes actions, branches, edits the tree, performs mutation, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: ancestry is tested purely (FS-042 plus a wrapper test over in-memory nodes); no I/O or model in tests.

Manual checks:

- Open a workspace and approve a couple of actions to grow the tree, select the latest node, and confirm the root→node path shows; deselect and confirm it clears.
- Select a leaf and confirm the path matches the visible indentation.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of a selected node and its replay path.
- Short note confirming read-only replay (no re-execution) and unchanged capabilities.

## 5. Success criteria

- SC1. Any session node can be selected to reveal the decision path that led to it.
- SC2. Replay is read-only — no re-execution, branching, or tree changes.
- SC3. Unknown selections degrade calmly to an empty path.
- SC4. No mutation or new capability is introduced.
- SC5. The slice stays story-sized — path reconstruction only; re-execution and branching are out.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Path correctness | root→node order; unknown → empty | Rust tests | @BrendanShields |
| Read-only | tree unchanged after replay | Source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 re-execution, 0 mutation, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-042 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `replay_path` command returning a node's root→node ancestry via `path_to_root`.
- A renderer node-selection + replay-path surface.

### Out of scope

- Re-executing or simulating the actions along the path.
- Branching/forking from a node, or editing/pruning the tree.
- Diff/state reconstruction at a node beyond the recorded labels.
- Cross-session comparison or timeline animation.

## 8. Technical design

### Rust/Tauri core

In `session_tree`, add:

```text
replay_path(app, nodeId) -> Vec<SessionNode>    // #[tauri::command]; load nodes, path_to_root(nodes, nodeId)
```

It loads the session tree and returns `path_to_root(&nodes, node_id)` — read-only, reusing the FS-042 pure function. Register the command.

### React renderer

Make session-tree nodes selectable. On select, call `replay_path(nodeId)` and render the returned chain (root→node) in a calm surface; clear on deselect. Transient state.

### Styling

Reuse the tree/list styles; a simple selected state and path list.

## 9. Impact notes

- Data model impact: no new persisted entities; returns a `SessionNode` chain.
- Security/privacy impact: read-only ancestry over the app-local tree; no mutation, model, network, or capability.
- Observability impact: completes session replay — the decision path to any node is inspectable.
- Performance impact: O(depth) per replay.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: implying replay re-runs actions. Mitigation: replay is read-only path reconstruction; re-execution is explicitly out of scope.
- Risk: broken tree links. Mitigation: the underlying `path_to_root` is bounded and robust (FS-042).
- Dependency: FS-042 (session tree + `path_to_root`).

## 11. Open questions

None. This slice reconstructs and shows the decision path to a node; re-execution, branching, and state diffing are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-042 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-043-session-replay`
- Expected implementation PR title: `feat(FS-043): Session replay`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-043/amendments/`.
