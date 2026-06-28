---
spec_id: FS-026
title: Autonomy mode
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
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-026: Autonomy mode

## 1. Problem statement

Synth supports the idea of two execution modes — supervised and high-autonomy (PRD §13) — and the PRD requires the active autonomy mode to be visible at all times (§13.3): the user should never wonder whether Synth is acting supervised or autonomously. Today the mode is a static label in the FS-001 runtime status snapshot; it cannot be read as live state or changed.

This story makes the autonomy mode real, trusted core state: it can be read and toggled, and it is shown in the shell. To stay safe, this slice establishes and surfaces the mode only — it does **not** change approval behaviour. Every mutation continues to require explicit approval (ADR-0012) regardless of mode; the high-autonomy *reduction* of routine prompts is deferred to a later spec that can classify action risk carefully and preserve the trust boundaries the PRD requires high autonomy to never bypass (§13.2).

## 2. Requirements

- R1. The Rust core must own the autonomy mode as live state in Tauri managed state, one of `supervised` or `high_autonomy`, defaulting to `supervised`.
- R2. The core must expose `get_autonomy_mode() -> AutonomyMode` and `set_autonomy_mode(mode: String) -> Result<AutonomyMode, String>`. `set_autonomy_mode` must accept only the two valid values (rejecting anything else with a readable `Err`) and return the new mode.
- R3. `AutonomyMode` must serialize in camelCase with at least a `mode` field (`supervised` or `high_autonomy`).
- R4. Validating/normalizing a mode string must be a pure, unit-testable function.
- R5. This story must not change approval behaviour: every mutating action still requires explicit approval via the FS-018 gate, in both modes. The mode must not auto-approve, skip, or weaken any approval in this slice.
- R6. The renderer must display the current autonomy mode at all times (a persistent, quiet indicator) and provide an accessible control to toggle it, calling `set_autonomy_mode` and reflecting the result. The displayed mode must stay in sync with the core state.
- R7. This story must not add new Tauri capability permissions, must not persist the mode to disk (process-lifetime state only), and must not change the FS-001 runtime status contract or any existing command.

## 3. Acceptance criteria

- AC1. `get_autonomy_mode()` returns `supervised` by default.
- AC2. `set_autonomy_mode("high_autonomy")` returns `high_autonomy`, and a subsequent `get_autonomy_mode()` returns `high_autonomy`.
- AC3. `set_autonomy_mode("bogus")` returns `Err` and does not change the mode.
- AC4. The pure mode validator accepts `supervised` and `high_autonomy` (case-insensitively) and rejects others.
- AC5. The renderer shows the current mode persistently and toggling it updates both the display and the core state.
- AC6. Submitting a mutating action (e.g. create branch) still produces an approval request requiring explicit approval in both modes (approval behaviour is unchanged).
- AC7. Rust unit coverage verifies the pure validator and camelCase serialization; the get/set/default behaviour is covered.
- AC8. No code in this story auto-approves a mutation, persists the mode, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks:

- Confirm the shell shows the current autonomy mode persistently.
- Toggle the mode and confirm the indicator updates and that `get_autonomy_mode` reflects it.
- Request a mutation in high-autonomy mode and confirm it still requires explicit approval.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the mode indicator in both states.
- Short note confirming approval behaviour is unchanged in both modes and capabilities are unchanged.

## 5. Success criteria

- SC1. The autonomy mode is live, trusted core state, readable and toggleable.
- SC2. The active mode is visible at all times (PRD §13.3).
- SC3. Approval behaviour is unchanged; no mutation is auto-approved.
- SC4. No persistence or new capability is introduced.
- SC5. The slice stays story-sized and defers high-autonomy prompt reduction.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Mode correctness | get/set/default behave; invalid rejected | Rust tests | @BrendanShields |
| Visibility | mode shown persistently and stays in sync | Manual review | @BrendanShields |
| Gate unchanged | mutations still require approval in both modes | Manual / source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 auto-approvals, 0 persistence, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-025 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Core autonomy-mode state with `get_autonomy_mode` / `set_autonomy_mode` and a pure validator.
- An `AutonomyMode` camelCase shape.
- A persistent renderer mode indicator and toggle.
- Rust unit tests for validation and get/set.

### Out of scope

- Auto-approving or reducing prompts in high-autonomy mode (a later, carefully-scoped spec).
- Bypassing any trust boundary (PRD §13.2 — never in any mode).
- Persisting the mode across sessions or recording mode-change events durably.
- Per-action risk classification or a policy engine.

## 8. Technical design

### Rust/Tauri core

Add an `autonomy` module: an `AutonomyMode { mode }` camelCase shape, an `AutonomyState(Mutex<String>)` managed in Tauri state defaulting to `supervised`, a pure `normalize_mode(&str) -> Option<&'static str>` accepting the two values case-insensitively, and `get_autonomy_mode` / `set_autonomy_mode` commands. `set_autonomy_mode` rejects invalid values. Register the commands and manage the state. No approval code is touched.

### React renderer

Fetch `get_autonomy_mode` on load and show a persistent, quiet indicator (e.g. in the footer alongside the artifact status). Provide an accessible toggle that calls `set_autonomy_mode` and updates the indicator. Keep it calm and minimal.

### Styling

Reuse the footer/status styles; add a small mode indicator and toggle consistent with the existing chrome.

## 9. Impact notes

- Data model impact: introduces an `AutonomyMode` IPC shape and process-lifetime managed state; nothing persisted.
- Security/privacy impact: none; the mode is surfaced state and does not change enforcement. Approval remains required for all mutations in both modes.
- Observability impact: mode can be noted in the FS-011 session log; durable mode-change events are future.
- Performance impact: negligible.
- Migration/backward compatibility impact: additive; all prior contracts unchanged. The FS-001 status snapshot still reports its static label and is not changed by this slice.

## 10. Risks and dependencies

- Risk: implying high-autonomy reduces prompts now. Mitigation: this slice explicitly does not change approval behaviour; that reduction is a later spec with risk classification.
- Risk: mode display drifting from core state. Mitigation: the renderer reads `get_autonomy_mode` and updates from the `set` result.
- Dependency: FS-025 merged (managed-state pattern, shell footer).

## 11. Open questions

None. This slice establishes and surfaces the autonomy mode without changing enforcement.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-025 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-026-autonomy-mode`
- Expected implementation PR title: `feat(FS-026): Autonomy mode`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-026/amendments/`.
