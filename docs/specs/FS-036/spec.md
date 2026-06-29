---
spec_id: FS-036
title: Improvement signals
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

# FS-036: Improvement signals

## 1. Problem statement

The PRD treats errors, repeated failures, amendments, and friction as improvement signals that should be surfaced (PRD §22). FS-032 persists session events durably; this story reads them and derives signals — patterns worth the user's attention, such as repeated errors, command failures, and spec amendments. It completes the Phase 4 observability surface with a read-only, deterministic analysis over the existing event log.

This is read-only and advisory: it analyses recorded events and reports signals. It changes nothing, runs no model, and is fully deterministic.

## 2. Requirements

- R1. The Rust core must expose `improvement_signals() -> Vec<Signal>` that loads recent app-local events (FS-032) and returns derived signals, ordered by importance then recency.
- R2. Deriving signals from a slice of event records must be a pure, deterministic, unit-testable function (no I/O, no model).
- R3. `Signal` must serialize in camelCase with at least: `kind`, `summary` (human-readable), and `count` (how many events contributed).
- R4. The analyzer must derive at least these signal kinds:
  - `repeated-errors`: two or more `error`-kind events among the recent window;
  - `command-failure`: events whose detail indicates a failed command (e.g. contains `[exit` with a non-zero code, or `timed out`);
  - `amendment`: events indicating a spec amendment was requested/saved.
  A kind with no contributing events must not be emitted.
- R5. The analysis window and any per-kind counts must be bounded; the function must not be sensitive to malformed records (they are simply not matched).
- R6. The renderer must show the current signals (kind + summary + count) in a calm surface, with a clear empty state when there are none. Signal state must be transient renderer state only.
- R7. This story must not write events, run a model, perform git/network/filesystem mutation, add a Tauri capability, or change the FS-001 runtime status contract. It only reads the app-local event log.

## 3. Acceptance criteria

- AC1. Given recent events containing two or more `error` events, `improvement_signals` includes a `repeated-errors` signal with `count` ≥ 2.
- AC2. Given an event whose detail contains `[exit 1]` or `timed out`, a `command-failure` signal is emitted.
- AC3. Given events indicating a spec amendment, an `amendment` signal is emitted.
- AC4. Given no matching events, `improvement_signals` returns an empty list.
- AC5. The pure analyzer is deterministic for a given input and ignores malformed/non-matching records.
- AC6. The renderer shows signals (kind + summary + count) and a calm empty state.
- AC7. Rust unit coverage verifies each signal kind, the no-signal case, ordering, and camelCase serialization, against in-memory event records (no I/O).
- AC8. No code in this story writes events, runs a model, performs mutation, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the analyzer is tested against in-memory event records; no I/O or model in tests.

Manual checks:

- Trigger a failed command and an error, then confirm the corresponding signals appear.
- Save an amendment and confirm the amendment signal appears.
- With a clean session, confirm the calm empty state.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the signals surface with a few signals and the empty state.
- Short note confirming read-only analysis, no model, and unchanged capabilities.

## 5. Success criteria

- SC1. Synth surfaces deterministic improvement signals from the event log.
- SC2. The analyzer is pure and robust to malformed records.
- SC3. Signals are advisory and read-only.
- SC4. No writes, model calls, mutation, or new capability are introduced.
- SC5. The slice stays story-sized and does not act on signals or learn from them.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Signal coverage | repeated-errors / command-failure / amendment derived | Rust tests | @BrendanShields |
| Determinism | same input → same output; malformed ignored | Rust tests | @BrendanShields |
| Read-only | 0 writes, 0 model, 0 mutation | Source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 new capabilities | Capability diff | @BrendanShields |
| Contract stability | FS-001..FS-035 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An `improvement_signals` command loading recent events and returning derived signals.
- A pure, deterministic signal analyzer with unit tests.
- A `Signal` camelCase shape.
- A renderer signals surface with an empty state.

### Out of scope

- Acting on signals (changing prompts, policy, or behaviour).
- Model-derived sentiment or loop/stall detection beyond the deterministic rules here.
- Persisting signals or aggregating across sessions/projects.
- Recommendations or auto-remediation.

## 8. Technical design

### Rust/Tauri core

Add a `signals` module:

```text
Signal { kind, summary, count }            // serde camelCase
detect_signals(events: &[EventRecord]) -> Vec<Signal>   // pure, deterministic
improvement_signals(app) -> Vec<Signal>     // #[tauri::command] (loads events, then detect_signals)
```

`detect_signals` scans the records (bounded window): counts `error`-kind events (`repeated-errors` when ≥ 2), matches command-failure markers in `detail` (`[exit ` with non-zero / `timed out`), and matches amendment markers (e.g. `amendments/` in detail or a save-amendment label). It emits a `Signal` per non-empty kind, ordered by a fixed importance then count. `improvement_signals` loads recent events via the FS-032 loader and delegates.

### React renderer

On load (and optionally on demand), call `improvement_signals` and render the signals (kind + summary + count) in a calm surface near the event stream, with an empty state. Transient state.

### Styling

Reuse the event/list styles.

## 9. Impact notes

- Data model impact: introduces a `Signal` IPC shape; no persisted entities.
- Security/privacy impact: read-only analysis of the app-local event log; no model, no mutation, no capability.
- Observability impact: turns recorded events into actionable signals — the seed of the improvement loop.
- Performance impact: a bounded scan over recent events; negligible.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: noisy or misleading signals. Mitigation: a small set of conservative, deterministic rules; advisory only.
- Risk: sensitivity to malformed records. Mitigation: non-matching records are ignored; tested.
- Dependency: FS-032 (app-local event persistence) and the event record shape.

## 11. Open questions

None. This slice derives deterministic signals from the event log and surfaces them; model-derived signals and remediation are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-035 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-036-improvement-signals`
- Expected implementation PR title: `feat(FS-036): Improvement signals`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-036/amendments/`.
