---
spec_id: FS-003
title: Slash command navigation routing
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

# FS-003: Slash command navigation routing

## 1. Problem statement

FS-002 makes the command dock interactive and teaches the Rust core to classify raw input into typed command intents, but those intents still do not route anywhere. The next safe, story-sized step is to let the command dock handle one low-risk class of command: slash navigation within the already-rendered Synth shell.

This story adds a Rust-owned routing disposition for parsed commands and lets React perform only local, non-privileged view/section navigation when the routed command targets an existing in-memory shell section. Unsupported commands remain honest no-ops, and shell commands remain explicitly blocked because approvals and safe command execution do not exist yet.

The goal is to prove that Synth can move from "input parsed" to "intent routed" without crossing any trust boundary: no workspace access, provider calls, shell execution, persistence, policy decisions, or document loading.

## 2. Requirements

- R1. The Rust core must expose a Tauri command named `route_command` that accepts a single raw input string and returns a typed command-route snapshot. Routing must reuse the FS-002 parsing rules as the authoritative classifier and must not duplicate parser logic in React.
- R2. The route payload returned to React must serialize in camelCase with these fields:
  - `parsed`: the FS-002 parsed-command payload;
  - `disposition`: one of `handled`, `unsupported`, `blocked`, `empty`;
  - `target`: one of `summary`, `runtime-status`, `event-stream`, `phase`, `none`;
  - `message`: a short human-readable explanation of what happened or why no action happened.
- R3. `route_command` must handle only supported slash-navigation commands. The supported navigation arguments are:
  - `/summary` → `target: summary`
  - `/runtime-status` and `/runtime` → `target: runtime-status`
  - `/event-stream` and `/events` → `target: event-stream`
  - `/phase` → `target: phase`
- R4. A supported slash-navigation route must return `disposition: handled` and a non-`none` target. It must not itself mutate UI state; React performs the local section navigation after receiving the route payload.
- R5. Unknown slash-navigation arguments, including `/specs`, must return `disposition: unsupported`, `target: none`, and a message that the route is not available yet.
- R6. Non-navigation parsed kinds (`ask`, `reference`, `tag`, `steer`, and `natural`) must return `disposition: unsupported`, `target: none`, and a message that the corresponding behavior arrives in a later spec.
- R7. Shell commands (`!`) must return `disposition: blocked`, `target: none`, preserve `parsed.requiresApproval: true`, and state that shell execution requires approval and is not yet available. Shell commands must not execute.
- R8. Empty or whitespace-only input must remain a renderer-side no-op: React must not invoke `route_command`, add a log entry, or navigate. If `route_command` is called directly with empty input, it must return `disposition: empty`, `target: none`.
- R9. On non-empty submit, the React renderer must call `route_command`, clear the input field, render the returned parsed command and route disposition in the transient command log, and perform local navigation only when `disposition` is `handled` and `target` is not `none`.
- R10. Local navigation must only scroll or focus an existing in-memory element in the current shell. It must not change browser location, create routes, read documents, open repositories, or persist navigation state.
- R11. The command log entry must visibly show the parsed `kind`, `argument`, `requiresApproval`, route `disposition`, route `target`, and route `message`. The most recent entry must remain visually identifiable as latest.
- R12. If routing invocation fails or a handled target cannot be found in the rendered shell, the renderer must show a clear non-crashing dock error while preserving previously rendered command log entries.
- R13. This story must not change the FS-001 runtime status contract or remove the FS-002 `parse_command` command. It must not add workspace filesystem access, provider/network calls, credential access, shell execution, app-local persistence, policy decisions, or new Tauri capability permissions.

## 3. Acceptance criteria

- AC1. Given the running shell, when the user submits `/runtime-status`, then the command log shows `kind: navigate`, `argument: runtime-status`, `disposition: handled`, `target: runtime-status`, and the runtime-status section is brought into view.
- AC2. Given the user submits `/runtime`, then the route is handled as an alias for `runtime-status`.
- AC3. Given the user submits `/events`, then the route is handled as an alias for `event-stream`.
- AC4. Given the user submits `/specs`, then the command log shows `disposition: unsupported`, `target: none`, and no view/document navigation occurs.
- AC5. Given the user submits `? what does this mean`, then the command log shows `kind: ask`, `disposition: unsupported`, and no provider call occurs.
- AC6. Given the user submits `! cargo test`, then the command log shows `kind: shell`, `requiresApproval: true`, `disposition: blocked`, `target: none`, and no shell command executes.
- AC7. Given empty or whitespace-only input, submitting remains a no-op with no route invocation and no log entry.
- AC8. Given a handled route target is not found in the rendered DOM, the dock shows a readable route-unavailable error and preserves existing log entries.
- AC9. Rust unit coverage verifies every supported navigation route and alias, unknown slash routes, all unsupported non-navigation kinds, blocked shell behavior, empty direct routing, and camelCase serialization.
- AC10. No code in this story opens repositories, reads or writes workspace files, executes shell commands, calls providers, persists operational data, requests additional Tauri plugin permissions, removes `parse_command`, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks required before the implementation PR is ready:

- Run the app in development mode and submit `/summary`, `/runtime-status`, `/runtime`, `/event-stream`, `/events`, and `/phase`; confirm each handled route brings the corresponding section into view.
- Submit `/specs`, `? what does this mean`, `@docs/PRD.md`, `#FS-003`, `> stop`, natural language, and `! cargo test`; confirm all are unsupported or blocked as specified and perform no privileged action.
- Confirm submitting empty input still does nothing and that the input clears after each non-empty submit.
- Confirm browser/devtools console has no unhandled routing invocation errors.
- Confirm `src-tauri/capabilities/*.json` does not gain new filesystem, shell, network, dialog, or credential permissions for this story.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot or short screen recording showing at least one handled route, one unsupported route, and one blocked shell route in the command log.
- Short note confirming no new privileged capabilities were added, `parse_command` remains available, and the FS-001 status contract is unchanged.

## 5. Success criteria

- SC1. Slash-navigation commands can produce a safe, observable handled route from the Rust core to the React shell.
- SC2. Unsupported and blocked commands are clearly distinguished from handled commands without silently doing work.
- SC3. The command dock demonstrates the first routed action while staying within an in-memory UI-only boundary.
- SC4. Shell execution, provider behavior, reference resolution, persistence, and policy remain deferred.
- SC5. The implementation remains story-sized and builds directly on FS-002 without broadening the command system into a command palette or workflow engine.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Supported route coverage | 100% of supported slash routes and aliases covered by Rust tests | `cargo test` output | @BrendanShields |
| Unsupported/blocked coverage | Unknown slash, ask, reference, tag, steer, natural, shell, and empty dispositions covered | `cargo test` output | @BrendanShields |
| Dock routing visibility | Log displays parsed kind, argument, requiresApproval, disposition, target, and message | Manual UI review / screenshot | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Privileged scope containment | 0 new filesystem/shell/network/credential capabilities and 0 executed privileged actions | Capability diff and source review | @BrendanShields |
| Contract stability | `parse_command` remains available and FS-001 runtime status contract unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Add a Rust `CommandRoute` / routed-command payload with camelCase serialization.
- Add `route_command(input: String) -> CommandRoute` and register it in `tauri::generate_handler!`.
- Reuse FS-002 parsing logic in the Rust core.
- Route only `/summary`, `/runtime-status`, `/runtime`, `/event-stream`, `/events`, and `/phase`.
- Render route disposition/target/message in the transient dock log.
- Scroll or focus existing shell sections for handled routes.
- Show non-crashing route errors while preserving previous log entries.
- Add Rust unit tests for route decisions and frontend helper tests for route-log formatting or target handling.

### Out of scope

- New app routes, router libraries, URL changes, or browser history behavior.
- A `/specs` index or document reader.
- Reading markdown files or loading repo documents.
- Asking the current artifact or calling a model provider.
- Resolving `@` references or `#` tags.
- Shell execution, command policy, approval prompts, or audit events.
- Persistence of command history, routes, or navigation state.
- Command palette overlay, autocomplete, fuzzy search, or live per-keystroke parsing.
- Event-store integration or audit-log integration for routed commands.
- Removing or changing the FS-002 `parse_command` contract.

## 8. Technical design

### Rust/Tauri core

Extend the FS-002 command-dock module rather than creating a second parser. The core owns route evaluation through a pure, unit-testable function, for example:

```text
route_raw_command(input: &str) -> CommandRoute
```

Expose this Tauri command:

```text
route_command(input: String) -> CommandRoute
```

The route payload contains:

```text
parsed: ParsedCommand
disposition: handled | unsupported | blocked | empty
target: summary | runtime-status | event-stream | phase | none
message: String
```

Route evaluation rules:

- Parse first with the FS-002 parser.
- If parsed kind is `empty`, return `empty`.
- If parsed kind is `shell`, return `blocked`.
- If parsed kind is not `navigate`, return `unsupported`.
- If parsed kind is `navigate`, match the trimmed, lowercased argument against the supported route table.
- Return `unsupported` for unknown navigation arguments.

The core does not mutate state or perform navigation; it only returns the routing decision.

### React renderer

Replace the submit-time `parse_command` invocation with `route_command` for non-empty submissions. Keep `parse_command` available for compatibility and tests. The renderer stores transient routed-command entries in the same bounded log pattern introduced by FS-002.

For handled routes, React maps the `target` string to an existing element id and calls a local UI-only navigation helper, such as:

```text
scrollHandledRoute(target)
```

This helper may use `document.getElementById(target)?.scrollIntoView({ block: "start" })` and may focus the target when appropriate. If the target is missing, it returns a typed failure result so the dock can render an inline error without discarding the log.

Route display should extend the FS-002 log entry with two additional fields: `disposition` and `target`, plus the route message.

### Styling

Reuse the FS-002 command log styling. Add small, low-contrast metadata for route `disposition` and `target`. Handled, unsupported, blocked, and empty states may use subtle text labels, but do not introduce high-saturation status colors or a broader design system.

## 9. Impact notes

- Data model impact: introduces the `CommandRoute` IPC shape but no persisted entities or migrations; the command log remains transient renderer state.
- Security/privacy impact: no new filesystem, shell, network, credential, or workspace access; shell input remains blocked and never executed.
- Observability impact: route decisions are visible in-session only; no event store, audit log, or persistence is added.
- Performance impact: negligible; one local IPC call per non-empty submit and one DOM lookup for handled local navigation.
- Migration/backward compatibility impact: builds additively on FS-002; `parse_command` remains available and the FS-001 runtime status contract is unchanged.

## 10. Risks and dependencies

- Risk: local navigation could look like a full document/navigation system. Mitigation: route only to existing shell sections and mark unknown routes as unsupported.
- Risk: route handling could drift from parsing. Mitigation: `route_command` reuses the FS-002 parser in Rust and React only renders route results.
- Risk: shell commands might appear closer to execution. Mitigation: shell routes are blocked with an explicit approval-not-available message and no shell plugin/capability changes.
- Dependency: FS-002 implementation must be merged before this implementation begins because this story builds on its parser, `ParsedCommand` payload, interactive dock, and transient log.

## 11. Open questions

None. This slice intentionally supports only local slash navigation to already-rendered shell sections.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-002 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-003-slash-command-navigation`
- Expected implementation PR title: `FS-003 Slash command navigation routing`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-003/amendments/`.
