---
spec_id: FS-006
title: Active spec artifact context
status: Draft for review
type: feature-spec
created: 2026-06-28
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0001-rust-native-runtime.md
  - docs/adrs/ADR-0006-story-sized-immutable-feature-specs.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-006: Active spec artifact context

## 1. Problem statement

FS-005 lets the user select a known feature spec and view its static detail, but the shell still has no notion of a *current artifact*. The PRD's interface direction (PRD §19.1) calls for "one artifact at a time" with a "tiny contextual status" that always tells the user what they are working on. Today the selected spec detail is rendered as a section, but nothing persistently signals which spec is active, and there is no way to clear the selection back to a neutral state.

This story adds an active-artifact context to the shell. When a spec is selected — from the FS-005 specs index control or a `/specs/<spec-id>` command — it becomes the shell's active artifact, surfaced in a small, always-visible contextual status. The user can clear the active artifact with an accessible control to return to "no active artifact".

The goal is to make the command-native shell feel like it is centered on a single artifact, while staying inside the same safe boundary as FS-004 and FS-005: the active artifact is derived entirely from the Rust-owned spec detail already provided by `get_static_spec_detail`, held only in transient renderer state, with no new IPC contract, capability, or persistence.

## 2. Requirements

- R1. The renderer must maintain a single "active artifact" derived from the FS-005 selected spec detail. At any time there is either exactly one active spec artifact or none.
- R2. Selecting a spec — via the FS-005 specs-index select control or a handled `/specs/<spec-id>` route — must set that spec as the active artifact, using the canonical spec id from the Rust-owned detail/route payload.
- R3. The shell must render a persistent, always-visible contextual status that shows the active artifact's canonical `specId` and `title` when one is active, and a clear neutral label (for example, "No active artifact") when none is active.
- R4. The renderer must provide an accessible control to clear the active artifact. Activating it must return the shell to the no-active-artifact state, clearing the selected detail and any detail error, and must not navigate away or scroll.
- R5. While an artifact is active, the matching entry in the specs index must be marked as current in an accessible way (for example, `aria-current`), and no entry may be marked current when there is no active artifact.
- R6. Clearing the active artifact must be a renderer-side no-op with respect to the trusted core: it must not call any Tauri command, read documents, persist state, or change browser location/history.
- R7. The contextual status and active-artifact selection must be transient renderer state only. They must not write to disk, app-local storage, browser storage, repo files, or URL/history state.
- R8. This story must not change the FS-001 runtime status contract and must not change or remove any existing Tauri command (`parse_command`, `route_command`, `list_specs_index`, `get_static_spec_detail`) or its payload shape. It introduces no new Tauri command and no new route target.
- R9. This story must not add runtime workspace filesystem access, directory scanning, markdown parsing, provider/network calls, shell execution, credential access, app-local persistence, policy decisions, or new Tauri capability permissions.
- R10. Existing FS-003/FS-004/FS-005 behavior — slash navigation, the specs index, `/specs`, `/specs/<spec-id>` detail routing, the command log, and the spec-detail section — must remain unchanged.

## 3. Acceptance criteria

- AC1. Given no spec has been selected, when the shell loads, then the contextual status shows the neutral no-active-artifact label and no specs-index entry is marked current.
- AC2. Given the specs index is rendered, when the user selects the FS-002 entry, then the contextual status shows `FS-002` and its title, and the FS-002 entry is marked current.
- AC3. Given the user submits `/specs/FS-003`, then the active artifact becomes FS-003, the contextual status reflects FS-003, and the FS-003 entry is marked current.
- AC4. Given an active artifact, when the user activates the clear control, then the contextual status returns to the neutral label, no entry is marked current, and the spec-detail section returns to its empty state without navigation.
- AC5. Given the user submits `/specs/FS-999`, then the active artifact is unchanged (the prior selection, or none) and no fake artifact becomes active.
- AC6. Clearing the active artifact triggers no Tauri command invocation and no console errors.
- AC7. Renderer helper/unit coverage verifies active-artifact formatting (active and neutral states) and the clear transition, and existing route-target and helper coverage remains stable.
- AC8. No code in this story adds a Tauri command, changes an existing command payload, opens repositories, reads workspace files, persists operational data, requests additional Tauri plugin permissions, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks required before the implementation PR is ready:

- Run the app in development mode; confirm the contextual status starts neutral.
- Select two different specs from the index and confirm the contextual status and the current-entry marker update each time.
- Submit `/specs/FS-001` and confirm the contextual status updates and the entry is marked current.
- Activate the clear control and confirm the status returns to neutral, the current marker clears, and the spec-detail section returns to its empty state.
- Submit `/specs/FS-999` and confirm the active artifact does not change.
- Confirm browser/devtools console has no errors and that clearing performs no IPC call.
- Confirm `src-tauri/capabilities/*.json` does not gain new filesystem, shell, network, dialog, or credential permissions for this story.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot showing the contextual status with an active artifact and the marked current index entry, and a second showing the neutral state after clearing.
- Short note confirming no Tauri command was added or changed, no new privileged capabilities were added, and the FS-001 status contract is unchanged.

## 5. Success criteria

- SC1. The shell continuously communicates which spec, if any, is the active artifact.
- SC2. The user can both set (via index or command) and clear the active artifact.
- SC3. The active-artifact context is derived from existing Rust-owned data with no new IPC surface.
- SC4. The change stays within in-memory renderer state and adds no new privileged behavior.
- SC5. The implementation remains story-sized and does not become a multi-artifact workspace, tab system, or persisted session model.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Contextual status coverage | Status reflects active and neutral states for every selection path (index + command) | Manual UI review / screenshot | @BrendanShields |
| Clear behavior | Clear returns to neutral with no IPC call and no navigation | Manual UI review and source review | @BrendanShields |
| Current-entry marking | Exactly one index entry marked current when active, none when neutral | Renderer tests and manual review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Privileged scope containment | 0 new Tauri commands, 0 new filesystem/shell/network/credential capabilities | Capability diff and source review | @BrendanShields |
| Contract stability | All FS-001..FS-005 commands and contracts remain available/unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Track a single active spec artifact in transient renderer state, derived from the FS-005 selected detail.
- Render a persistent contextual status for the active artifact, with a neutral empty state.
- Add an accessible control to clear the active artifact.
- Mark the active spec's index entry as current.
- Add renderer helper/unit tests for active-artifact formatting and the clear transition.

### Out of scope

- Any new or changed Tauri command, IPC payload, or route target.
- Reading spec markdown bodies, directory scanning, or any runtime document filesystem access.
- Multiple simultaneous artifacts, tabs, history, or an artifact switcher beyond the existing index/command paths.
- Persistence of the active artifact, selection, or any session state.
- Resolving `@` references or `#` tags, asking the active artifact (`?`), or steering (`>`).
- Provider calls, shell execution, approvals, policy, or audit events.
- URL/history changes, router libraries, search, fuzzy command palette, or autocomplete.

## 8. Technical design

### Rust/Tauri core

No change. The active artifact is derived entirely from the existing FS-005 `get_static_spec_detail` payload and the FS-005 `/specs/<spec-id>` route resource id. This story deliberately adds no command, route target, or IPC field so the trusted core stays exactly as FS-005 left it.

### React renderer

Introduce a small notion of the active artifact on top of the FS-005 selected-detail state:

- Treat the selected spec detail as the active artifact; "no active artifact" is the absence of a selected detail.
- Add a persistent contextual status element in the shell (for example, near the header or as a compact status line) that renders the active artifact's `specId` and `title`, or a neutral label when none is active.
- Add an accessible clear control (a button) that resets the selected detail and any detail error to none. This is a pure local state reset and must not invoke a Tauri command.
- Mark the active spec's specs-index entry as current (for example, `aria-current="true"`), reusing the existing per-entry rendering.

Factor the active-artifact presentation into small, pure helpers in `src/runtime.ts` (for example, a formatter that maps an optional detail to its display label) so the behavior is unit-testable without a DOM.

### Styling

Reuse the existing editorial, low-contrast styling. Add only minimal styling for the contextual status and the clear control. Avoid tabs, a switcher, badges with high-saturation status colors, or a new design system.

## 9. Impact notes

- Data model impact: none; no new IPC shape, no persisted entities, no migrations. The active artifact is transient renderer state.
- Security/privacy impact: none; no new filesystem, shell, network, credential, workspace, provider, or persistence access. Clearing performs no IPC.
- Observability impact: active-artifact state is visible in-session only; no event store, audit log, or persistence is added.
- Performance impact: negligible; no new IPC calls and only small local state updates.
- Migration/backward compatibility impact: builds additively on FS-005; all existing commands, routes, and sections are unchanged.

## 10. Risks and dependencies

- Risk: the contextual status could grow into a multi-artifact or tabbed workspace. Mitigation: scope to exactly one active artifact derived from the existing selection, and defer multi-artifact UX to a later spec.
- Risk: clearing could be mistaken for a privileged action. Mitigation: clear is a pure local state reset with no IPC call, asserted by source review and tests.
- Risk: active-state marking could drift from the selected detail. Mitigation: derive both the contextual status and the current-entry marker from the same single selected-detail state.
- Dependency: FS-005 implementation must be merged before this implementation begins because this story builds on the selected spec detail, the specs index, and the `/specs/<spec-id>` route.

## 11. Open questions

None. This slice intentionally surfaces and clears a single active spec artifact from existing Rust-owned data without adding any new IPC surface.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-005 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-006-active-spec-artifact-context`
- Expected implementation PR title: `feat(FS-006): Active spec artifact context`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-006/amendments/`.
