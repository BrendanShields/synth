---
spec_id: FS-032
title: App-local event persistence
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
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
---

# FS-032: App-local event persistence

## 1. Problem statement

The FS-011 event log is transient — it vanishes on restart. The PRD makes observability a first-class, durable surface (PRD §21) and stores private operational records (transcripts, tool-call logs, audit) in app-local storage, not the repository (PRD §17.2, ADR-0003). This story adds the first durable record: append session events to an app-local JSONL file and load the recent tail on startup, so the activity log survives restarts.

This is private operational truth: it lives in the OS app-data directory, never in the workspace/repo, and is append-only. It is the seed of the durable event store and audit log; richer session trees and replay remain later specs.

## 2. Requirements

- R1. The Rust core must expose `append_event(kind: String, label: String, detail: String) -> Result<EventRecord, String>` that appends a typed event record (with a monotonically assigned id and the fields) as one JSON line to an app-local events file, returning the stored record.
- R2. The Rust core must expose `load_events(limit: u32) -> Vec<EventRecord>` returning up to the most recent `limit` records from the app-local events file, newest first. A missing file returns an empty list (not an error).
- R3. `EventRecord` must serialize in camelCase with at least: `id`, `kind`, `label`, and `detail`.
- R4. The events file must live in the OS app-data directory (resolved via the Tauri path API), never inside the workspace or repository. It must be append-only; this story must not rewrite or truncate prior content beyond the load cap.
- R5. Serializing a record to a JSONL line and parsing a JSONL line back must be pure, unit-testable functions; a malformed line must be skipped on load, not panic.
- R6. Appending must be resilient: a write failure returns a readable `Err` without panicking; reading tolerates a missing or partially-written file.
- R7. The renderer must persist each recorded session event via `append_event` (in addition to the existing in-memory log) and load the recent events on startup via `load_events`, so the event-stream surface shows prior-session activity. The in-memory cap (FS-011) still bounds what is displayed.
- R8. This story must not write to the workspace/repo, must not log secrets, must not add new Tauri capability permissions (persistence uses core-side `std::fs` to the app-data dir), and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `append_event("command", "navigate", "handled → specs")` returns an `EventRecord` with an id and the fields, and writes one JSON line to the app-local events file.
- AC2. After several appends, `load_events(2)` returns the two most recent records, newest first.
- AC3. With no events file, `load_events` returns an empty list and no error.
- AC4. The pure serializer produces a single-line JSON record; the pure parser round-trips it; a malformed line is skipped by the loader.
- AC5. On startup the renderer loads recent events and shows them; new events are appended and displayed.
- AC6. The events file is in the app-data directory, not the workspace; no workspace/repo file is written.
- AC7. Rust unit coverage verifies record serialization/parsing (round-trip + malformed skip), the append/load tail against a temporary file, and camelCase serialization.
- AC8. No code in this story writes to the workspace/repo, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: append/load are tested against a temporary file path (not the real app-data dir); serialization/parsing are pure. No network I/O.

Manual checks:

- Run the app, perform a few actions, restart, and confirm prior-session events appear in the event stream.
- Confirm the events file is created under the app-data directory and not in the repo/workspace.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- The app-data events file path and a sample line (no secrets).
- Short note confirming app-local-only writes, append-only, and unchanged capabilities.

## 5. Success criteria

- SC1. Session events persist across restarts in app-local storage.
- SC2. Records are append-only JSONL in the app-data dir, never in the repo (ADR-0003).
- SC3. Serialization/parsing are pure and tested; malformed lines are tolerated.
- SC4. No workspace writes, secrets, or new capability are introduced.
- SC5. The slice stays story-sized and does not build session trees, replay, or redaction.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Persistence | events survive restart; load returns recent tail | Rust tests / manual | @BrendanShields |
| Location | events in app-data dir, never the repo/workspace | Source review / manual | @BrendanShields |
| Robustness | missing/malformed file tolerated; write error typed | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 workspace writes, 0 secrets, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-031 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `append_event` / `load_events` over an app-local JSONL file in the app-data dir.
- An `EventRecord` camelCase shape and pure serialize/parse helpers.
- Renderer persistence of recorded events and loading the recent tail on startup.
- Rust unit tests for serialize/parse and append/load against a temp file.

### Out of scope

- Session trees, replay, or compaction (PRD §23).
- A full audit log of tool calls/approvals/policy (a later phase; this is the seed).
- Redaction of secrets (events here carry no secrets) or export bundles.
- Rotating/compacting the events file beyond the load cap.
- Storing transcripts or model runs.

## 8. Technical design

### Rust/Tauri core

Add an `events` module:

```text
EventRecord { id, kind, label, detail }   // serde camelCase
serialize_record(record) -> String         // pure: one JSON line
parse_record(line) -> Option<EventRecord>  // pure: None on malformed
append_record(path, record) -> Result<(), String>   // append one line
load_records(path, limit) -> Vec<EventRecord>        // tail, newest first
append_event(app, kind, label, detail) -> Result<EventRecord, String>   // #[tauri::command]
load_events(app, limit) -> Vec<EventRecord>                              // #[tauri::command]
```

The commands resolve `{app_data_dir}/events.jsonl` via the Tauri path API (creating the dir if needed), assign the id (e.g. a process-lifetime counter in managed state, or line count), and use `append_record`/`load_records`. `append_record` opens the file in append mode and writes `serialize_record(record) + "\n"`. `load_records` reads the file, parses each line (skipping malformed), and returns the last `limit` reversed. Id assignment uses an `EventCounter` managed state seeded from the file length on first use, or a simple atomic counter; ids need only be unique within a run for display.

### React renderer

Extend the FS-011 `recordEvent` flow to also call `append_event` (fire-and-forget) for each event. On startup, call `load_events(MAX_SESSION_EVENTS)` and seed the in-memory log so prior-session activity shows. The displayed log stays bounded by the FS-011 cap.

### Styling

No new styling; reuse the FS-011 event list.

## 9. Impact notes

- Data model impact: introduces an `EventRecord` IPC shape and an app-local JSONL file; the first durable operational record (ADR-0003 app-local domain).
- Security/privacy impact: writes only to the app-data directory (never the repo/workspace); events carry no secrets; no capability added. Redaction/export are later.
- Observability impact: the event log becomes durable across restarts — the seed of the audit log/event store.
- Performance impact: one small append per event; a bounded tail read on startup; negligible.
- Migration/backward compatibility impact: additive; the in-memory log still works if persistence fails.

## 10. Risks and dependencies

- Risk: writing to the repo by mistake. Mitigation: the path is resolved from the app-data dir via the Tauri path API; tests/source confirm no workspace write.
- Risk: unbounded file growth. Mitigation: append-only with a bounded tail load; rotation/compaction is a later spec (noted, not silently capped).
- Risk: malformed/partial lines. Mitigation: the loader skips unparseable lines.
- Dependency: FS-011 (in-session event log) and the Tauri path API.

## 11. Open questions

None. This slice persists session events append-only in app-local storage and loads the recent tail; trees, replay, and full audit are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-031 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-032-event-persistence`
- Expected implementation PR title: `feat(FS-032): App-local event persistence`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-032/amendments/`.
