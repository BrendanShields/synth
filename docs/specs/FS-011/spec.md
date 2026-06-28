---
spec_id: FS-011
title: In-session event log
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
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-011: In-session event log

## 1. Problem statement

The PRD makes observability a first-class product surface (PRD §21): sessions, commands, tool calls, answers, and errors should be visible as structured events. The shell already has an "Event stream" section, but it only shows the FS-001 bootstrap snapshot. Meanwhile the app now produces real activity — routed commands (FS-003/FS-005/FS-008) and streamed answers (FS-010) — that vanishes without a trace.

This story turns the Event stream section into a live, in-session event log that records what happened this session: each routed command and each ask lifecycle (asked → answered or failed). It is in-memory and transient — the durable event store and audit log remain Phase 4 — but it gives the user a quiet, continuous record of session activity, consistent with the documents-first, observable-by-default direction.

## 2. Requirements

- R1. The renderer must maintain a bounded, in-session list of structured `SessionEvent`s in transient state, newest first, capped at a fixed maximum.
- R2. A `SessionEvent` must have at least: a stable `id`, a `kind` (`command`, `answer`, or `error`), a short `label`, and a `detail` string.
- R3. The renderer must record a `command` event whenever a command route is produced (any disposition), summarizing the parsed kind and route disposition/target.
- R4. The renderer must record an `answer` event when an ask completes (the model returns a final answer), and an `error` event when an ask fails, including whether the answer was grounded in a spec.
- R5. Appending an event must be a pure, unit-testable function that prepends the event and enforces the cap.
- R6. Formatting a `SessionEvent` for display must be a pure, unit-testable function.
- R7. The Event stream section must render the event log newest-first as a quiet list. When empty, it must show a calm placeholder. The existing FS-001 bootstrap runtime event may remain as the stream's origin/first context but must not block the live log.
- R8. The event log must be transient renderer state only: no persistence to disk, app-local storage, browser storage, repo files, or URL/history.
- R9. This story must not add a Tauri command, change the FS-001 runtime status contract, change any existing command payload, or add provider, filesystem, persistence, policy, or new Tauri capability behavior. It observes data the app already produces.
- R10. Recording events must not change existing behavior: command routing, navigation, spec selection, and streamed answers must work exactly as before.

## 3. Acceptance criteria

- AC1. `appendSessionEvent` prepends the newest event and enforces the cap (e.g., with cap 2, a third event drops the oldest).
- AC2. `formatSessionEvent` renders a readable one-line summary for `command`, `answer`, and `error` kinds.
- AC3. Submitting `/specs` records a `command` event showing the navigate kind and handled/specs disposition.
- AC4. Submitting `? ...` and receiving an answer records an `answer` event; a failed ask records an `error` event.
- AC5. A grounded ask's `answer` event indicates the grounding spec id; an ungrounded ask's does not.
- AC6. The Event stream section renders the log newest-first and shows a calm placeholder before any activity.
- AC7. Frontend unit coverage verifies the bounded append and event formatting for each kind.
- AC8. No code in this story adds a Tauri command, persistence, provider/filesystem access, new capabilities, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks:

- Submit a few commands (`/specs`, `/runtime`, a `?` question) and confirm each appears in the Event stream log newest-first.
- Confirm a failed ask (stop Ollama) appears as an `error` event.
- Confirm the log is capped and the oldest entries drop.
- Confirm `src-tauri/capabilities/*.json` gains no new permissions.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the Event stream section showing a live session log.
- Short note confirming no Tauri command/capability was added and existing contracts are unchanged.

## 5. Success criteria

- SC1. Session activity (commands and asks) is visible as a quiet, continuous in-session log.
- SC2. The log is bounded and newest-first.
- SC3. Event shaping and formatting are pure and unit-tested.
- SC4. No new privileged behavior or persistence is introduced.
- SC5. The slice stays story-sized and does not become an audit log, event store, or persisted session model.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Activity coverage | Commands and ask outcomes recorded as events | Manual UI review / screenshot | @BrendanShields |
| Bounded log | Append caps the list and drops oldest | Frontend tests | @BrendanShields |
| Pure-logic coverage | Append + format covered by tests | `bun run test` output | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 commands, 0 persistence, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-010 contracts unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A transient, bounded, newest-first `SessionEvent` log in the renderer.
- Recording command-route events and ask outcome (answer/error) events.
- Pure append and format helpers with unit tests.
- Rendering the log in the existing Event stream section.

### Out of scope

- Any Tauri command, Rust core change, or new IPC shape.
- Persistence, an event store, or an audit log (Phase 4).
- Tool-call, approval, or policy events (those behaviors do not exist yet).
- Filtering, search, export, or replay of events.
- Timestamps sourced from the trusted core (kept simple; ordering is by insertion).

## 8. Technical design

### React renderer

Add to `runtime.ts`:

```text
type SessionEvent = { id, kind, label, detail }
appendSessionEvent(events, event, max?) -> SessionEvent[]   // pure, bounded, newest-first
formatSessionEvent(event) -> string                          // pure
```

In `App.tsx`, keep a `sessionEvents` state. Record a `command` event in `submitCommand` after each `route_command` result, and `answer`/`error` events in the ask flow (on the streaming done/error handling). Derive a stable `id` from an incrementing counter. Render the list in the Event stream section, newest-first, with a calm empty placeholder. The FS-001 bootstrap runtime event may remain as a small origin line above the log.

### Styling

Reuse existing quiet list/mono styles; add minimal styling for log entries. No badges, colors, or chrome beyond a low-contrast list.

## 9. Impact notes

- Data model impact: introduces a renderer-only `SessionEvent` shape; no IPC, no persisted entities.
- Security/privacy impact: none; observes existing in-session data, no new access.
- Observability impact: first user-visible in-session activity log; durable event store remains deferred.
- Performance impact: negligible; bounded in-memory list.
- Migration/backward compatibility impact: additive and renderer-only; all contracts unchanged.

## 10. Risks and dependencies

- Risk: the log could grow into an unbounded memory leak. Mitigation: a fixed cap enforced by the tested append helper.
- Risk: event recording could entangle with existing handlers. Mitigation: record from existing result/event handlers without altering their behavior; covered by unchanged routing/answer tests.
- Dependency: FS-010 merged (ask lifecycle and streamed answers are the primary event sources).

## 11. Open questions

None. This slice provides a bounded, transient, in-session event log only.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-010 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-011-session-event-log`
- Expected implementation PR title: `feat(FS-011): In-session event log`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-011/amendments/`.
