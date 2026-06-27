---
spec_id: FS-004
title: Specs index reader shell
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

# FS-004: Specs index reader shell

## 1. Problem statement

FS-003 proves that slash commands can route safely to existing in-memory shell sections, but `/specs` still remains unsupported. That is acceptable for the routing slice, but Synth is a documents-first product: specs are the unit of implementation, and the command dock needs an early, safe way to bring a spec-oriented artifact surface into view.

This story adds a minimal specs index reader shell. The index is a static, Rust-owned catalog of the repo-versioned feature specs that are committed with Synth itself at implementation time. The React renderer displays that catalog as a focused artifact section, and `/specs` routes to it. This is not yet a workspace document reader: it does not scan directories, open repositories, read arbitrary files at runtime, parse markdown bodies, resolve links, or persist navigation state.

The goal is to make the first document-oriented command useful while staying inside the same trust boundary as the walking skeleton: typed IPC, in-memory rendering, no privileged filesystem access, and no policy engine.

## 2. Requirements

- R1. The Rust core must expose a Tauri command named `list_specs_index` that returns a typed static specs-index snapshot. The command must not read directories, open arbitrary files, access the workspace, call providers, execute shell commands, or persist data.
- R2. The specs-index payload returned to React must serialize in camelCase with these fields:
  - `artifactType`: `specs-index`
  - `generatedFrom`: `static-rust-catalog`
  - `specs`: an ordered array of spec summaries
  - `summary`: a short human-readable description of the index and its static limitation
- R3. Each spec summary must serialize in camelCase with these fields:
  - `specId`
  - `title`
  - `status`
  - `path`
  - `implementationBranch`
  - `route`
- R4. The static catalog must include entries for all feature specs committed in `docs/specs/` at implementation time. For this story that means, at minimum:
  - `FS-001`
  - `FS-002`
  - `FS-003`
  - `FS-004`
- R5. The static catalog must be ordered by `specId` ascending.
- R6. The route contract from FS-003 must be extended so `/specs` returns `disposition: handled`, `target: specs`, and a message indicating that the specs index is in view. This must reuse the FS-003 parser/router shape rather than adding a separate renderer-only command path.
- R7. The `RouteTarget` / target union must add `specs` while preserving the existing FS-003 targets (`summary`, `runtime-status`, `event-stream`, `phase`, `none`) and existing aliases.
- R8. The React renderer must call `list_specs_index` on startup or before the specs section is first rendered, store the result in transient renderer state only, and render a specs-index section with `id="specs"`.
- R9. The specs-index section must show, at minimum, the index `summary` plus each spec's `specId`, `title`, `status`, `path`, and `implementationBranch`.
- R10. On submit of `/specs`, the renderer must call `route_command`, add the route result to the transient command log, and locally scroll to the rendered `specs` section only when the returned route is handled.
- R11. If `list_specs_index` fails, the renderer must show a clear non-crashing inline error in the specs section. If `/specs` is routed before the specs section is available, the existing route-unavailable dock error behavior from FS-003 must apply without discarding previous log entries.
- R12. This story must not add runtime workspace filesystem access, directory scanning, markdown parsing, provider/network calls, shell execution, credential access, app-local persistence, policy decisions, or new Tauri capability permissions.
- R13. This story must not change the FS-001 runtime status contract, must not remove FS-002 `parse_command`, and must preserve FS-003 handled routes and unsupported/blocked route behavior except for changing `/specs` from unsupported to handled.

## 3. Acceptance criteria

- AC1. Given the app has loaded, when the specs-index section is visible, then it shows a static catalog containing at least FS-001, FS-002, FS-003, and FS-004.
- AC2. Given the user submits `/specs`, then the command log shows `kind: navigate`, `argument: specs`, `disposition: handled`, `target: specs`, and the specs-index section is brought into view.
- AC3. Given the user submits existing FS-003 handled routes (`/summary`, `/runtime`, `/events`, `/phase`), then those routes still work as before.
- AC4. Given the user submits unsupported non-navigation input such as `? what does this mean`, then it remains unsupported and no provider call occurs.
- AC5. Given the user submits `! cargo test`, then it remains blocked with `requiresApproval: true` and no shell command executes.
- AC6. Given `list_specs_index` fails, then the specs section renders a readable specs-index-unavailable state instead of a blank screen or unhandled promise rejection.
- AC7. Rust unit coverage verifies the static catalog contents, ordering, camelCase serialization, and `/specs` routing behavior.
- AC8. Renderer helper/unit coverage verifies the specs-index formatting or route target mapping for the new `specs` target.
- AC9. No code in this story opens repositories, scans directories, reads arbitrary workspace files at runtime, executes shell commands, calls providers, persists operational data, requests additional Tauri plugin permissions, removes `parse_command`, removes `route_command`, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks required before the implementation PR is ready:

- Run the app in development mode and confirm the specs-index section renders a static list of feature specs.
- Submit `/specs` and confirm it routes to the specs-index section and logs a handled route with `target: specs`.
- Submit `/runtime`, `/events`, `/phase`, an unsupported ask command, and a blocked shell command to confirm FS-003 behavior remains intact.
- Confirm browser/devtools console has no unhandled specs-index or routing invocation errors.
- Confirm `src-tauri/capabilities/*.json` does not gain new filesystem, shell, network, dialog, or credential permissions for this story.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot showing the specs-index section and command log after `/specs`.
- Short note confirming the catalog is static, no runtime workspace/document reading was added, no new privileged capabilities were added, `parse_command` and `route_command` remain available, and the FS-001 status contract is unchanged.

## 5. Success criteria

- SC1. `/specs` becomes the first document-oriented command that routes to a useful artifact surface.
- SC2. Synth displays an ordered specs index without adding privileged runtime filesystem access.
- SC3. The Rust core owns the specs-index data contract and route target, preserving the trusted-kernel pattern.
- SC4. Existing command parsing/routing behavior remains stable except for the intentional `/specs` upgrade.
- SC5. The implementation remains story-sized and does not become a general document reader, workspace opener, or file indexer.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Catalog completeness | Static catalog includes every committed feature spec at implementation time | Rust tests and UI screenshot | @BrendanShields |
| `/specs` route behavior | `/specs` returns `handled` and target `specs` | Rust tests and manual UI review | @BrendanShields |
| Regression containment | Existing FS-003 routes and unsupported/blocked behaviors unchanged | Rust tests and manual checks | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Privileged scope containment | 0 new filesystem/shell/network/credential capabilities and 0 runtime directory/file scans | Capability diff and source review | @BrendanShields |
| Contract stability | `parse_command`, `route_command`, and FS-001 runtime status contract remain available/unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Add Rust specs-index payload types with camelCase serialization.
- Add `list_specs_index() -> SpecsIndex` and register it in `tauri::generate_handler!`.
- Define a static Rust catalog for committed feature specs.
- Extend `RouteTarget` and `route_command` so `/specs` routes to `target: specs`.
- Add a `specs` section to the current shell.
- Render spec metadata from the static index.
- Preserve the transient command route log behavior introduced by FS-003.
- Add Rust and frontend/helper tests for catalog and route target behavior.

### Out of scope

- Runtime directory scanning or reading arbitrary markdown files.
- Opening or indexing external repositories.
- Parsing markdown frontmatter from disk.
- Rendering full spec document bodies.
- Navigating to a specific individual spec detail view.
- Resolving `@` references or `#` tags.
- Asking questions about a spec or calling a model provider.
- Shell execution, command policy, approval prompts, or audit events.
- Persistence of command history, document history, routes, or navigation state.
- Event-store integration or audit-log integration.
- Broader command palette, autocomplete, fuzzy search, or keyboard shortcut work.

## 8. Technical design

### Rust/Tauri core

Add a focused specs-index module under `src-tauri/src/` or extend a small document-catalog module if one exists by implementation time. Keep `src-tauri/src/lib.rs` as the registration point.

The Rust core owns these types:

```text
SpecsIndex
SpecIndexEntry
```

`SpecsIndex` contains:

```text
artifactType: specs-index
generatedFrom: static-rust-catalog
specs: Vec<SpecIndexEntry>
summary: String
```

`SpecIndexEntry` contains the fields listed in R3. The static catalog should be simple Rust data construction, not runtime file IO. Tests should assert the required spec ids and ordering.

Expose this command:

```text
list_specs_index() -> SpecsIndex
```

Extend the command router from FS-003:

```text
/specs -> disposition: handled, target: specs
```

Do not remove or rename `parse_command` or `route_command`.

### React renderer

Add a `SpecsIndex` TypeScript type mirroring the Rust payload. Fetch the index once per app load using `list_specs_index` and store it in transient component state.

Add a new section to the central document body:

```text
<section id="specs">...</section>
```

Render the index as a quiet metadata list/table consistent with the existing editorial shell. Include a readable loading state and a non-crashing unavailable state.

Update route target helpers so `specs` maps to `id="specs"`. Keep `/specs` navigation UI-only: call `scrollIntoView` on the existing element after `route_command` returns a handled `specs` target.

### Styling

Reuse the existing document/status/log visual language. Add only the minimal CSS needed for a specs metadata list. Avoid a full document reader layout, tabs, sidebars, syntax highlighting, or a new design system.

## 9. Impact notes

- Data model impact: introduces static `SpecsIndex` / `SpecIndexEntry` IPC shapes but no persisted entities or migrations.
- Security/privacy impact: no runtime filesystem, shell, network, credential, workspace, or provider access; the index is static and non-sensitive.
- Observability impact: specs-index state is visible in-session only; no event store, audit log, or persistence is added.
- Performance impact: negligible; one local IPC call and a small static list render.
- Migration/backward compatibility impact: builds additively on FS-003; `/specs` changes from unsupported to handled while other routes and the FS-001/FS-002/FS-003 contracts remain stable.

## 10. Risks and dependencies

- Risk: a static specs index could be mistaken for a real workspace document reader. Mitigation: UI and summary copy must state that the catalog is static and that workspace document reading arrives later.
- Risk: static catalog entries can drift when future specs are added. Mitigation: Rust tests assert catalog completeness for the known committed specs at implementation time; future specs can update the catalog until a dynamic reader exists.
- Risk: `/specs` route implementation could accidentally broaden into file access. Mitigation: scope explicitly forbids runtime directory scanning, markdown reading, and new filesystem capabilities.
- Dependency: FS-003 implementation must be merged before FS-004 implementation begins because this story extends `route_command`, `RouteTarget`, and the routed command log.

## 11. Open questions

None. This slice intentionally implements only a static specs index and `/specs` route.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-003 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-004-specs-index-view`
- Expected implementation PR title: `FS-004 Specs index reader shell`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-004/amendments/`.
