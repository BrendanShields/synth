---
spec_id: FS-002
title: Command dock parsing and intent routing
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

# FS-002: Command dock parsing and intent routing

## 1. Problem statement

FS-001 shipped the Synth shell and the runtime event bridge, but it left the bottom command dock as a disabled, read-only placeholder whose copy states that command handling arrives in a later spec. The command/input dock is the product's primary control surface: the PRD defines a grammar (`/`, `?`, `@`, `#`, `!`, `>`, and natural language) that the user types to navigate, question, reference, tag, run shell, and steer.

This story makes the command dock accept input and turns raw text into a typed, classified command intent owned by the Rust/Tauri core. It establishes the first piece of the ERD's "Command / Input Router" inside the trusted kernel. It deliberately stops at classification and routing disposition: it does not execute navigation, ask a provider, resolve references, run shell commands, mutate files, or persist anything. The shell `!` verb is recognized but explicitly marked as requiring approval that is not yet available.

## 2. Requirements

- R1. The command dock input introduced in FS-001 must become interactive: it must no longer be `disabled` or `readOnly`, and the user must be able to type and submit text.
- R2. The Rust core must expose a Tauri command named `parse_command` that accepts a single raw input string and returns a typed parsed-command snapshot. Parsing must not execute, navigate, call providers, read or write files, run shell commands, or persist data.
- R3. The parsed-command payload returned to React must serialize in camelCase with these fields:
  - `raw`: the original input string exactly as received;
  - `kind`: one of `navigate`, `ask`, `reference`, `tag`, `shell`, `steer`, `natural`, `empty`;
  - `verb`: the leading grammar glyph (`/`, `?`, `@`, `#`, `!`, `>`) or an empty string for `natural` and `empty`;
  - `argument`: the remainder after the grammar glyph, trimmed; an empty string when there is no remainder;
  - `requiresApproval`: a boolean that is `true` only for the `shell` kind and `false` for every other kind;
  - `summary`: a short human-readable description that names the recognized intent and states that the corresponding action arrives in a later spec or is not yet available.
- R4. Classification must follow this grammar, using only the first non-whitespace character as the prefix:
  - `/` → `navigate`
  - `?` → `ask`
  - `@` → `reference`
  - `#` → `tag`
  - `!` → `shell` (`requiresApproval` is `true`)
  - `>` → `steer`
  - any other non-empty content → `natural`
  - input that is empty or only whitespace → `empty`
- R5. Parsing rules must be unambiguous:
  - leading whitespace is ignored when detecting the prefix;
  - a recognized prefix with no remaining text yields that verb's `kind` with `argument` set to an empty string (it is not reclassified as `natural` or `empty`);
  - a grammar glyph that appears anywhere other than the first non-whitespace position does not change the kind (for example, `fix the /runtime route` is `natural`).
- R6. On submit, the React renderer must send the raw input to `parse_command`, clear the input field, and display the returned parsed command in a transient, in-session command log. Submitting empty or whitespace-only input must be a no-op that performs no invocation and adds no log entry.
- R7. The command log entries rendered must show, at minimum, the recognized `kind`, the `argument`, the `requiresApproval` state, and the `summary`. The most recent entry must be visually identifiable as the latest.
- R8. The command log must be transient renderer state only. It must not be written to disk, the app-local store, or any repository file, and it must reset on reload.
- R9. If `parse_command` invocation fails, the renderer must show a clear, non-crashing inline error in the dock area without discarding previously parsed log entries and without an unhandled promise rejection.
- R10. This story must not change the FS-001 runtime status contract (`get_runtime_status`, `announce_runtime_status`, the `synth-runtime-status` event, or the `RuntimeStatus`/`RuntimeEvent` field values).
- R11. This story must not add workspace filesystem access, provider/network calls, credential access, shell execution, app-local persistence, policy decisions, or new Tauri capability permissions.

## 3. Acceptance criteria

- AC1. Given the running shell, when the user focuses the command dock, then the input accepts typing and submission instead of being disabled.
- AC2. Given the input `/specs`, when the user submits it, then the rendered log shows a `navigate` command with `argument` `specs` and a summary indicating navigation arrives later.
- AC3. Given the input `! cargo test`, when the user submits it, then the rendered log shows a `shell` command with `argument` `cargo test` and `requiresApproval` true, and nothing is executed.
- AC4. Given plain text such as `add a workspace opener`, when the user submits it, then the rendered log shows a `natural` command with an empty `verb`.
- AC5. Given input that is empty or only spaces, when the user submits it, then no log entry is added and no `parse_command` invocation is made.
- AC6. Given a recognized prefix with no remainder such as `?`, when parsed, then the result is kind `ask` with an empty `argument` (not `natural` or `empty`).
- AC7. Given a glyph that is not in the first position such as `email me @ 5pm`, when parsed, then the result is kind `natural`.
- AC8. Rust unit coverage verifies the classification of every grammar prefix, the leading-whitespace rule, the prefix-with-no-remainder rule, the not-first-position rule, the empty/whitespace rule, and that `requiresApproval` is true only for `shell`.
- AC9. When `parse_command` fails, the dock shows a readable error state and previously parsed entries remain visible.
- AC10. No code in this story opens repositories, reads or writes workspace files, executes shell commands, calls providers, persists operational data, or requests additional Tauri plugin permissions, and the FS-001 runtime status contract is unchanged.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks required before the implementation PR is ready:

- Run the app in development mode and submit one example of each grammar prefix plus a natural-language line, and confirm the log classifications match the acceptance criteria.
- Confirm submitting empty input does nothing and that the input clears after each non-empty submit.
- Confirm browser/devtools console has no unhandled invocation errors.
- Confirm `src-tauri/capabilities/*.json` does not gain new filesystem, shell, network, dialog, or credential permissions for this story.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the command dock showing a parsed log with at least the `navigate`, `shell`, and `natural` kinds.
- Short note confirming no new privileged capabilities were added and the FS-001 status contract is unchanged.

## 5. Success criteria

- SC1. The command dock is interactive and turns raw input into a typed, classified intent.
- SC2. The Rust trusted core owns command classification, beginning the ERD Command / Input Router.
- SC3. The grammar from the PRD command dock is recognized end to end without executing any privileged action.
- SC4. The UI honestly communicates that recognized intents are not yet actionable and that shell intent requires approval that is not yet available.
- SC5. The implementation remains story-sized and does not introduce navigation, provider calls, reference resolution, shell execution, persistence, or a policy engine.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Grammar coverage | All 8 kinds (`navigate`, `ask`, `reference`, `tag`, `shell`, `steer`, `natural`, `empty`) classified by `parse_command` | Rust tests and screenshot | @BrendanShields |
| Dock interactivity | Dock accepts input, clears on submit, and renders a transient log | Manual UI review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Privileged scope containment | 0 new filesystem/shell/network/credential capabilities and 0 executed actions | Capability diff and source review | @BrendanShields |
| Contract stability | 0 changes to the FS-001 runtime status contract | PR diff review | @BrendanShields |
| Scope drift | 0 navigation handlers, provider calls, reference resolvers, persistence layers, or policy engine added | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Enable the FS-001 command dock input for typing and submission.
- Add a Rust `ParsedCommand` type with camelCase serialization.
- Add the Tauri command `parse_command` and register it in `tauri::generate_handler!`.
- Implement grammar classification for `/`, `?`, `@`, `#`, `!`, `>`, natural language, and empty input.
- Render a transient, in-session command log of parsed intents.
- Mark the `shell` verb as requiring approval that is not yet available.
- Add Rust unit tests for classification and a renderer unit test for the dock log helper.

### Out of scope

- Executing navigation or changing the rendered view.
- Asking the current artifact or calling any model provider.
- Resolving `@` references or `#` tags to files, specs, sessions, or releases.
- Running shell commands or building a policy/approval engine.
- Steering an active agent turn.
- Persisting command history to the app-local store or any file.
- Emitting or persisting runtime events for parsed commands (event store is deferred).
- A command palette overlay, autocomplete, or live per-keystroke parsing UI.
- Changing the FS-001 runtime status values or event contract.

## 8. Technical design

### Rust/Tauri core

Add a focused command-parsing module under `src-tauri/src/` (for example `command_dock.rs`) and keep `src-tauri/src/lib.rs` as the thin registration point, mirroring how `runtime_status` is structured.

The Rust core owns this type:

```text
ParsedCommand
```

`ParsedCommand` contains the fields listed in R3 and uses `serde` camelCase serialization, consistent with `RuntimeStatus`. Classification is a pure function over the input string so it is directly unit-testable without a Tauri runtime.

Expose this command:

```text
parse_command(input: String) -> ParsedCommand
```

`parse_command` trims leading whitespace only to find the prefix, maps the first character to a `kind` per R4, computes the trimmed `argument`, sets `requiresApproval` true only for `shell`, and fills `summary` with an honest per-kind message. It performs no side effects.

Register `parse_command` alongside `get_runtime_status` and `announce_runtime_status` in the existing `invoke_handler`.

### React renderer

Update the dock in `App.tsx` so the input is interactive and submission is handled (Enter or form submit). The renderer owns presentation state only:

- a controlled input value;
- a transient list of `ParsedCommand` entries capped at a small bound (for example 20) so memory does not grow without limit.

On submit, the renderer trims the value; if empty it is a no-op. Otherwise it invokes `parse_command` with the raw value, clears the input, and prepends the result to the transient log. The latest entry is rendered first or otherwise marked as latest. Keep classification out of the renderer so there is a single authoritative parser in the core; the renderer may show a non-authoritative prefix glyph affordance, but the displayed classification comes from the core.

Add a small pure helper (for example in `src/runtime.ts` or a new module) that formats a `ParsedCommand` for display and that enforces the log cap, so the renderer has a unit-testable seam consistent with the FS-001 helper pattern.

On invocation failure, set a non-crashing inline error in the dock area and keep the existing log entries.

### Styling

Reuse the existing FS-001 dock and document styling. Render the command log within or directly above the dock using the established quiet, low-contrast visual language. Do not build a command palette overlay or a new design system in this story.

## 9. Impact notes

- Data model impact: introduces the `ParsedCommand` IPC shape but no persisted entities or migrations; the command log is transient renderer state.
- Security/privacy impact: no new filesystem, shell, network, credential, or workspace access; the `shell` verb is recognized but explicitly gated and never executed.
- Observability impact: parsed intents are visible in-session only; no event store, audit log, or persistence is added.
- Performance impact: negligible; one local IPC call per non-empty submit and a bounded in-memory list.
- Migration/backward compatibility impact: builds additively on FS-001 and leaves the FS-001 runtime status contract unchanged.

## 10. Risks and dependencies

- Risk: classification logic could drift between the core and the renderer. Mitigation: the core is the single authoritative parser; the renderer only displays results.
- Risk: the dock could imply that commands now act. Mitigation: each `summary` states that the action arrives in a later spec or is not yet available, and `shell` is flagged as requiring approval.
- Risk: this slice could expand into navigation or a command palette. Mitigation: scope boundaries exclude execution, resolution, persistence, and overlays.
- Dependency: FS-001 runtime bridge and shell, already merged on `main`, provide the dock, the IPC pattern, and the renderer helper/test pattern.

## 11. Open questions

None. The approved next slice is interactive command parsing and intent classification without execution.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-002-command-dock-parsing`
- Expected implementation PR title: `FS-002 Command dock parsing and intent routing`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-002/amendments/`.
