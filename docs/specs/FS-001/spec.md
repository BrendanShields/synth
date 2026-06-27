---
spec_id: FS-001
title: Runtime event bridge and Synth shell
status: Draft for review
type: feature-spec
created: 2026-06-27
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0001-rust-native-runtime.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-001: Runtime event bridge and Synth shell

## 1. Problem statement

Synth's repository has an approved planning baseline, but the application still renders the starter Tauri/Vite/React greeting scaffold. The first implementation story needs to replace that scaffold with a small vertical slice that proves Synth's core architectural boundary: the Rust/Tauri core is the trusted product kernel, and the React renderer is a thin visual surface subscribing to runtime state.

This story creates the first visible Synth shell and a typed runtime-status bridge from Rust to React. It does not implement providers, workspace opening, policy enforcement, persistence, agent execution, or real command handling.

## 2. Requirements

- R1. The starter greeting experience must be removed from the rendered app, including the Vite/Tauri/React logo row, starter headline, greeting form, and `greet` command usage.
- R2. The Rust core must expose a typed runtime status snapshot through a Tauri command named `get_runtime_status`.
- R3. The Rust core must expose a Tauri command named `announce_runtime_status` that emits a runtime-status event and returns the same event payload to the caller.
- R4. The runtime-status event channel name must be `synth-runtime-status`.
- R5. The runtime status payload returned to React must serialize in camelCase with these fields:
  - `productName`: `Synth`
  - `appVersion`: current Rust package version
  - `runtimeBoundary`: `rust-tauri-core`
  - `rendererBoundary`: `react-thin-renderer`
  - `autonomyMode`: `supervised`
  - `planningGate`: `clear`
  - `workspaceState`: `not_opened`
  - `providerState`: `not_configured`
  - `eventStreamState`: `ready`
  - `summary`: `Planning baseline merged. Ready for Phase 1 walking skeleton.`
- R6. The runtime event payload must serialize in camelCase with these fields:
  - `eventId`: `runtime-status-bootstrap`
  - `eventType`: `runtime.status.snapshot`
  - `status`: the runtime status payload from R5
- R7. The React renderer must register a listener for `synth-runtime-status`, request the runtime status on startup, and render the latest command/event status it receives.
- R8. The React renderer must render a minimal Synth shell with:
  - a tiny contextual status line;
  - one central artifact/status panel;
  - a bottom command/input dock placeholder.
- R9. The command/input dock must be visual only in this story. It must not parse commands, execute shell commands, call providers, mutate files, or persist input.
- R10. If the runtime status command or event announcement fails, the renderer must show a clear non-crashing error state inside the shell.
- R11. This story must not add workspace filesystem access, provider/network calls, credential access, app-local persistence, or policy decisions.

## 3. Acceptance criteria

- AC1. Launching the frontend no longer shows starter Tauri/Vite/React logos, starter copy, the greeting form, or greeting output.
- AC2. The rendered UI visibly identifies the app as Synth and shows the runtime boundary, renderer boundary, autonomy mode, planning gate, workspace state, provider state, and event stream state from the Rust status payload.
- AC3. On startup, React calls `get_runtime_status` and renders the returned snapshot before or while the event announcement is processed.
- AC4. React listens on `synth-runtime-status`, calls `announce_runtime_status`, receives an event with `eventType: runtime.status.snapshot`, and updates the UI to show the last received event id.
- AC5. Rust unit coverage verifies that the bootstrap runtime status contains the expected static states and package version.
- AC6. The app handles command/event failure by rendering a readable runtime-unavailable state instead of a blank screen or unhandled promise rejection.
- AC7. No implementation code in this story opens repositories, reads arbitrary workspace files, executes shell commands, configures model providers, stores operational data, or requests additional Tauri plugin permissions.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks required before the implementation PR is ready:

- Run the app in development mode and confirm the visible shell matches the acceptance criteria.
- Confirm browser/devtools console has no unhandled runtime-status invocation or event-listener errors.
- Confirm `src-tauri/capabilities/*.json` does not gain new filesystem, shell, network, dialog, or credential permissions for this story.

Evidence to attach to the implementation PR:

- Automated command output summary.
- Screenshot of the Synth shell showing the runtime status and last event id.
- Short note confirming no new privileged capabilities were added.

## 5. Success criteria

- SC1. Synth has its first real app surface instead of the starter scaffold.
- SC2. The Rust trusted runtime can provide a typed status snapshot to the React renderer.
- SC3. The Rust trusted runtime can emit a typed event that the React renderer consumes.
- SC4. The UI communicates the Phase 1 walking-skeleton state without implying unsupported workspace, provider, policy, or agent capabilities.
- SC5. The implementation remains story-sized and does not introduce Phase 2 security/workspace behavior or Phase 3 spec-to-PR behavior.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Runtime bridge completeness | `get_runtime_status` command and `synth-runtime-status` event both consumed by React | Implementation diff and screenshot | @BrendanShields |
| Starter scaffold removal | No rendered starter greeting/logos/form remain | Manual UI review and source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Privileged scope containment | 0 new filesystem/shell/network/credential capabilities | Capability diff review | @BrendanShields |
| Scope drift | 0 providers, workspace opener, persistence layer, command parser, or policy engine added | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Replace the starter app UI with a Synth-branded walking-skeleton shell.
- Add Rust-side serializable runtime status/event types.
- Add Tauri commands `get_runtime_status` and `announce_runtime_status`.
- Register both commands in `tauri::generate_handler!`.
- Add a small React runtime-status client/listener.
- Render a non-functional bottom command/input dock placeholder.
- Add Rust unit tests for the bootstrap status values.

### Out of scope

- Opening or indexing repositories.
- Reading or writing workspace files.
- App-local event store or audit log persistence.
- Model provider configuration or provider calls.
- Agent loop, sessions, compaction, or subagents.
- Policy engine or approvals.
- Real command parsing, slash commands, shell execution, or command palette behavior.
- Multi-window behavior, tray behavior, updater behavior, signing, or installer work.
- Full visual design system beyond the minimum shell needed for this story.

## 8. Technical design

### Rust/Tauri core

Add a focused runtime status module under `src-tauri/src/` and keep `src-tauri/src/main.rs` as the thin passthrough.

The Rust core owns these types:

```text
RuntimeStatus
RuntimeEvent
```

`RuntimeStatus` contains the fields listed in R5 and uses `serde` camelCase serialization. `appVersion` is sourced from `env!("CARGO_PKG_VERSION")`. The bootstrap status is static for this story.

`RuntimeEvent` contains the fields listed in R6 and wraps the `RuntimeStatus` snapshot.

Expose these commands:

```text
get_runtime_status() -> RuntimeStatus
announce_runtime_status(app: tauri::AppHandle) -> Result<RuntimeEvent, String>
```

`announce_runtime_status` builds the same bootstrap event returned to the caller, emits it on `synth-runtime-status`, and returns the event. If emit fails, it returns a serialized error string for React to render.

The existing starter `greet` command should be removed because no rendered UI uses it after this story.

### React renderer

Replace the starter `App.tsx` experience with a thin Synth shell. The renderer owns presentation state only:

```text
loading | ready | runtime-unavailable
```

On mount, React registers the `synth-runtime-status` listener, invokes `get_runtime_status`, then invokes `announce_runtime_status`. The latest command snapshot and latest event payload are kept in component state and rendered in the central panel. The listener is cleaned up on unmount.

The shell should include:

- a small status line like `Synth · Supervised · Planning clear · Runtime ready`;
- a central panel titled `Runtime event bridge` with the runtime fields;
- a bottom command dock placeholder with copy such as `Command dock placeholder — command handling arrives in a later spec`.

### Styling

Use the PRD visual direction at a light-touch level: off-white background, quiet typography, low-contrast borders, whitespace, and a bottom dock. Do not spend this story creating a reusable design system.

## 9. Impact notes

- Data model impact: no persisted entities or migrations are introduced.
- Security/privacy impact: no new filesystem, shell, network, credential, or workspace access is introduced.
- Observability impact: establishes the first runtime event shape, but does not persist events or create an audit log.
- Performance impact: negligible; startup performs two local IPC calls and one event listener registration.
- Migration/backward compatibility impact: removes only starter scaffold behavior that is not part of Synth's product contract.

## 10. Risks and dependencies

- Risk: Tauri event timing could make an automatic startup event flaky. Mitigation: React registers the listener before invoking `announce_runtime_status`, and the command returns the same payload it emits.
- Risk: The UI could imply capabilities that do not exist yet. Mitigation: status values must explicitly show workspace and provider as unavailable/not configured.
- Risk: The first shell could expand into a design-system task. Mitigation: keep styling local and minimal; reusable component extraction is deferred.
- Dependency: Tauri v2 IPC and event APIs already present through `@tauri-apps/api` and `tauri` dependencies.

## 11. Open questions

None. The approved first slice is the runtime event bridge and minimal Synth shell.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged.

- Scope approval: Approved by @BrendanShields on 2026-06-27
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-001-runtime-event-bridge`
- Expected implementation PR title: `FS-001 Runtime event bridge and Synth shell`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-001/amendments/`.
