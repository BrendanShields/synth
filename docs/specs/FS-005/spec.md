---
spec_id: FS-005
title: Static spec detail selection
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
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
  - docs/adrs/ADR-0006-story-sized-immutable-feature-specs.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-005: Static spec detail selection

## 1. Problem statement

FS-004 introduces a static specs index and makes `/specs` route to a useful document-oriented shell section. The index can show which specs exist, but the user still cannot select one spec as the current artifact. Synth's product model centers work around an active spec contract, so the next small slice should let the user select a known feature spec and see a focused, static detail summary.

This story adds static spec-detail selection for the committed Synth feature specs. It extends the static Rust catalog from FS-004 with a small detail payload and lets React render one selected spec detail inside the existing shell. It intentionally does not read markdown files at runtime, parse document bodies, open repositories, implement a full document reader, or add editable artifacts.

The goal is to make the specs index feel like an artifact navigator while preserving the same safe boundary as FS-004: static Rust data, typed IPC, transient renderer state, and no workspace/document filesystem access.

## 2. Requirements

- R1. The Rust core must expose a Tauri command named `get_static_spec_detail` that accepts a `specId` string and returns a typed static spec-detail snapshot for a known feature spec. The command must not read directories, open arbitrary files, access the workspace, parse markdown, call providers, execute shell commands, or persist data.
- R2. The spec-detail payload returned to React must serialize in camelCase with these fields:
  - `specId`
  - `title`
  - `status`
  - `path`
  - `implementationBranch`
  - `route`
  - `summary`
  - `scope`
  - `limitations`
- R3. `get_static_spec_detail` must support all specs in the FS-004 static catalog at implementation time. For this story that means, at minimum, `FS-001`, `FS-002`, `FS-003`, `FS-004`, and `FS-005`.
- R4. `get_static_spec_detail` must match spec ids case-insensitively and normalize returned `specId` values to canonical uppercase ids such as `FS-001`.
- R5. If a requested spec id is not present in the static catalog, `get_static_spec_detail` must return a serialized error string that React can render.
- R6. The command router must support static detail routes for known specs using `/specs/<spec-id>`, case-insensitively. Examples:
  - `/specs/FS-001`
  - `/specs/fs-002`
- R7. A known static detail route must return `disposition: handled`, `target: spec-detail`, and a route resource id or equivalent typed field that identifies the canonical spec id. Existing FS-004 targets, including `/specs`, must remain unchanged.
- R8. Unknown static detail routes, such as `/specs/FS-999`, must return `disposition: unsupported`, `target: none`, and a message that the requested static spec detail is not available.
- R9. The React renderer must let the user select a spec from the specs index without typing, using a button or similarly accessible control per entry. Selection must call `get_static_spec_detail` and render the selected detail in transient renderer state.
- R10. Submitting a known `/specs/<spec-id>` command must call `route_command`, add the route result to the transient command log, fetch the static detail for the returned canonical spec id, and locally scroll to the spec-detail section.
- R11. The renderer must add a spec-detail section with `id="spec-detail"` that shows, at minimum, the selected detail's `specId`, `title`, `status`, `path`, `implementationBranch`, `summary`, `scope`, and `limitations`.
- R12. If `get_static_spec_detail` fails, the spec-detail section must show a clear non-crashing inline error while preserving the specs index and previous command log entries.
- R13. Selection and routed detail state must be transient renderer state only. It must not write to disk, app-local storage, browser storage, repo files, or URL/history state.
- R14. This story must not add runtime workspace filesystem access, directory scanning, markdown parsing, provider/network calls, shell execution, credential access, app-local persistence, policy decisions, or new Tauri capability permissions.
- R15. This story must not change the FS-001 runtime status contract, must not remove FS-002 `parse_command`, must preserve FS-003 route behavior, and must preserve FS-004 `list_specs_index` and `/specs` route behavior.

## 3. Acceptance criteria

- AC1. Given the specs index is rendered, when the user activates the FS-001 entry's select control, then the spec-detail section renders FS-001's static detail summary.
- AC2. Given the user submits `/specs/FS-002`, then the command log shows a handled route with `target: spec-detail`, the selected detail is FS-002, and the spec-detail section is brought into view.
- AC3. Given the user submits `/specs/fs-003`, then the route and detail lookup succeed and the rendered detail uses canonical `FS-003`.
- AC4. Given the user submits `/specs/FS-999`, then the command log shows an unsupported route and the currently selected spec detail is not replaced by a fake detail.
- AC5. Given the user submits `/specs`, then the FS-004 behavior still routes to the specs index, not to an individual detail.
- AC6. Given `get_static_spec_detail` returns an error, then the spec-detail section shows a readable detail-unavailable state without a blank screen or unhandled promise rejection.
- AC7. Rust unit coverage verifies known detail lookup, case-insensitive lookup, missing-spec errors, `/specs/<spec-id>` routing, unknown detail-route behavior, and camelCase serialization.
- AC8. Renderer helper/unit coverage verifies canonical spec-detail target handling or detail formatting, and existing route-target mapping remains stable.
- AC9. No code in this story opens repositories, scans directories, reads arbitrary workspace files at runtime, parses markdown from disk, executes shell commands, calls providers, persists operational data, requests additional Tauri plugin permissions, removes `parse_command`, removes `route_command`, removes `list_specs_index`, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks required before the implementation PR is ready:

- Run the app in development mode and select at least two specs from the specs index; confirm the detail section updates.
- Submit `/specs/FS-001`, `/specs/fs-002`, `/specs/FS-999`, and `/specs`; confirm handled, canonical, unsupported, and index-route behaviors respectively.
- Confirm unsupported ask/reference/tag/steer/natural commands and blocked shell commands still behave as before.
- Confirm browser/devtools console has no unhandled detail lookup or routing invocation errors.
- Confirm `src-tauri/capabilities/*.json` does not gain new filesystem, shell, network, dialog, or credential permissions for this story.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot showing the specs index, command log after a `/specs/<spec-id>` route, and the selected spec-detail section.
- Short note confirming the detail catalog is static, no runtime workspace/document reading was added, no new privileged capabilities were added, `parse_command`, `route_command`, and `list_specs_index` remain available, and the FS-001 status contract is unchanged.

## 5. Success criteria

- SC1. A known feature spec can become the active visible detail artifact from either the specs index or a slash command.
- SC2. Static spec-detail selection works without runtime document filesystem access.
- SC3. Unknown spec-detail requests are clearly unsupported and do not create fake state.
- SC4. FS-004's specs index remains stable and useful as the parent artifact.
- SC5. The implementation remains story-sized and does not become a full markdown reader, editor, workspace opener, or dynamic file indexer.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Static detail coverage | Detail lookup supports every static catalog spec at implementation time | Rust tests and UI screenshot | @BrendanShields |
| Detail route behavior | Known `/specs/<spec-id>` routes return handled + `spec-detail`; unknown ids return unsupported | Rust tests | @BrendanShields |
| Selection usability | At least one accessible select control per spec index entry | Manual UI review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Privileged scope containment | 0 new filesystem/shell/network/credential capabilities and 0 runtime directory/file scans | Capability diff and source review | @BrendanShields |
| Contract stability | `parse_command`, `route_command`, `list_specs_index`, and FS-001 status contract remain available/unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Add static spec-detail payload types with camelCase serialization.
- Add `get_static_spec_detail(specId: String) -> Result<StaticSpecDetail, String>` and register it in `tauri::generate_handler!`.
- Extend the static Rust catalog to include detail summaries for committed feature specs.
- Extend route handling for `/specs/<spec-id>` to known static details.
- Add `spec-detail` as a route target and render a `spec-detail` section.
- Add accessible selection controls in the specs index.
- Store selected detail and detail errors in transient React state only.
- Add Rust and frontend/helper tests for detail lookup, routing, and target handling.

### Out of scope

- Runtime directory scanning or reading arbitrary markdown files.
- Parsing markdown frontmatter from disk.
- Rendering full spec document bodies.
- Editing specs, approving specs, or creating amendments.
- Opening external repositories or building a workspace document reader.
- Resolving `@` references or `#` tags.
- Asking questions about a spec or calling a model provider.
- Shell execution, command policy, approval prompts, or audit events.
- Persistence of selected spec, command history, document history, routes, or navigation state.
- URL/history changes, router libraries, tabs, search, fuzzy command palette, or autocomplete.

## 8. Technical design

### Rust/Tauri core

Extend the FS-004 static specs catalog module. Keep the catalog static and in Rust code; do not read from `docs/specs/` at runtime.

The Rust core owns this additional type:

```text
StaticSpecDetail
```

`StaticSpecDetail` contains the fields listed in R2. Details can be concise summaries of the approved specs rather than full markdown bodies.

Expose this command:

```text
get_static_spec_detail(spec_id: String) -> Result<StaticSpecDetail, String>
```

Lookup rules:

- Trim whitespace.
- Normalize spec ids case-insensitively.
- Accept canonical ids such as `FS-001`.
- Return a readable error for unknown ids.

Extend `route_command` so `/specs/<spec-id>` is handled for known static specs and unsupported for unknown ids. To avoid React parsing route arguments, the route payload should include a typed resource id, selected id, or equivalent field containing the canonical spec id for handled spec-detail routes.

### React renderer

Extend the specs-index UI from FS-004 with a select control per entry. Selecting an entry invokes `get_static_spec_detail(spec.specId)` and stores the returned detail in component state.

Add a spec-detail section:

```text
<section id="spec-detail">...</section>
```

When `route_command` returns a handled `spec-detail` route, React uses the canonical spec id from the route payload to fetch the detail, renders it, and scrolls to `id="spec-detail"`.

The section should show a quiet empty state before selection, a loading state while a detail request is active if needed, and a readable error state if lookup fails.

### Styling

Reuse the existing specs-index metadata styling. Add only minimal styling for selected/detail states and the entry select control. Avoid a full document reader layout, markdown rendering, tabs, sidebars, syntax highlighting, or a new design system.

## 9. Impact notes

- Data model impact: introduces a static `StaticSpecDetail` IPC shape and optionally a route resource-id field; no persisted entities or migrations.
- Security/privacy impact: no runtime filesystem, shell, network, credential, workspace, provider, or persistence access; all detail data is static and non-sensitive.
- Observability impact: selected detail state is visible in-session only; no event store, audit log, or persistence is added.
- Performance impact: negligible; one local IPC call per selection/detail route and small static data.
- Migration/backward compatibility impact: builds additively on FS-004; existing `/specs` route and index remain stable.

## 10. Risks and dependencies

- Risk: static detail summaries could drift from markdown specs. Mitigation: keep summaries concise, visibly static, and replace with real document reading in a later spec.
- Risk: individual detail routes could be mistaken for a full document reader. Mitigation: UI copy and scope boundaries must state that full markdown rendering arrives later.
- Risk: route payload changes could break existing route rendering. Mitigation: make route resource-id behavior additive and preserve existing fields and values.
- Dependency: FS-004 implementation must be merged before this implementation begins because this story extends `list_specs_index`, `/specs`, and the specs section.

## 11. Open questions

None. This slice intentionally implements only static detail selection for known committed specs.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-004 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-005-static-spec-detail-selection`
- Expected implementation PR title: `FS-005 Static spec detail selection`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-005/amendments/`.
