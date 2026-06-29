---
spec_id: FS-046
title: Extension permission scope
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0002-process-based-extensibility.md
  - docs/adrs/ADR-0014-supervised-command-execution.md
  - docs/adrs/ADR-0012-approval-gate-for-mutations.md
---

# FS-046: Extension permission scope

## 1. Problem statement

The PRD's Phase 5 calls for extension permissions (PRD §16), and ADR-0002's principle is that extensions provide capability but not authority — what an extension may touch should be declared and surfaced to the human, with the trusted core deciding. FS-037 declares extensions but says nothing about what they do. This story adds a declared permission scope to each extension and surfaces it at the approval gate when the extension runs, so the human authorizes the run knowing the claim.

The scope is declarative transparency, not a sandbox: enforcement remains the command gate (FS-035) — every run is still explicitly approved, jailed, and bounded. The scope makes the claim visible at the moment of authorization.

## 2. Requirements

- R1. The `Extension` shape must gain a `scope` field (declared permission), serialized camelCase, defaulting to a safe value (`read`) when absent so existing registries load unchanged.
- R2. Validating a scope must be a pure, unit-testable function accepting a fixed set: `read`, `write`, `network`, `shell`. `register_extension` must accept and validate the scope.
- R3. The core must expose `request_run_extension(id: u64) -> Result<ApprovalRequest, String>` that looks up the extension by id and records a gated run-command approval whose summary includes the extension name and declared scope, with the exact command preserved. It must not execute anything.
- R4. Running an extension must remain a gated command (FS-035/ADR-0014): excluded from auto-approval in every mode, jailed, captured, bounded; this story changes only how the run is requested and what the approval surface shows, not the execution discipline.
- R5. An unknown extension id returns a readable `Err`; with no workspace open, requesting returns a readable `Err` (consistent with command execution).
- R6. The renderer must show each extension's scope, allow choosing it when registering, and run an extension via `request_run_extension` so the approval surface shows the name and scope. Display is transient renderer state.
- R7. This story must not enforce/sandbox the scope (the gate is the enforcement), must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `Extension` serializes with a `scope`; an extension loaded from a pre-scope registry defaults to `read`.
- AC2. `is_valid_extension_scope` accepts only `read|write|network|shell`; `register_extension` rejects an invalid scope.
- AC3. `request_run_extension(id)` for an existing extension returns a gated run-command `ApprovalRequest` (`autoApprove: false`) whose summary contains the extension name and scope and whose command equals the extension's command; nothing runs.
- AC4. `request_run_extension(unknown_id)` returns `Err`; with no workspace open it returns `Err`.
- AC5. Resolving the approval runs the command via the existing gate (jailed, captured); auto-approval never applies.
- AC6. The renderer shows scopes, registers with a scope, and runs via `request_run_extension` (approval shows name + scope).
- AC7. Rust unit coverage verifies scope validation, the default-on-missing behavior, and that `request_run_extension` builds a non-auto-approved run-command request carrying the scope and exact command.
- AC8. No code in this story sandboxes/enforces the scope, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: scope validation and default behavior are pure; the request path is tested against in-memory/temp state. No network in tests.

Manual checks:

- Register an extension with `network` scope, run it, and confirm the approval surface shows the scope before you approve.
- Confirm a pre-existing extension (no scope) shows `read`.
- Confirm running still requires explicit approval (even in high-autonomy).
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the approval surface showing the extension name + scope.
- Short note confirming the gate is unchanged (scope is declarative) and capabilities unchanged.

## 5. Success criteria

- SC1. Extensions declare a permission scope, surfaced at the approval gate.
- SC2. The run discipline (gated, jailed, bounded, never auto-approved) is unchanged.
- SC3. Existing registries load unchanged (scope defaults to `read`).
- SC4. No sandbox/enforcement or new capability is introduced (the gate is the control).
- SC5. The slice stays story-sized — declared scope + surfacing; runtime sandboxing is deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Scope validation | only read/write/network/shell accepted | Rust tests | @BrendanShields |
| Backward compat | pre-scope registries load with `read` | Rust tests | @BrendanShields |
| Gate integrity | run still gated, never auto-approved | Rust tests / source | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 sandboxing, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-045 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `scope` field on `Extension` with pure validation and a safe default.
- `request_run_extension` surfacing name + scope at the gate.
- A renderer scope selector, scope display, and run-via-extension.

### Out of scope

- Runtime sandboxing/enforcement of the scope (seccomp, namespaces, FS restriction).
- Multiple scopes per extension or fine-grained capability grammars.
- MCP protocol brokering or subagent definitions.
- Per-scope auto-approval policy (run is always gated).

## 8. Technical design

### Rust/Tauri core

In `extensions`: add `scope` to `Extension` (`#[serde(default = "default_scope")]`, `default_scope() -> "read"`), `is_valid_extension_scope`, and a `scope` parameter on `register_extension`. Add `request_run_extension(app, approvals, workspace, autonomy, id)` that loads the registry, finds the extension (Err if absent), and records a run-command approval with a scope-annotated summary (e.g. `Run extension {name} ({scope}): {command}`), `auto_approve` false, command = the extension command. Execution stays the `RunCommand` arm.

### React renderer

Add a scope `<select>` to the register form, show each extension's scope, and change "Run" to call `request_run_extension(id)` (the approval overlay then shows the name + scope).

### Styling

Reuse existing extension/control styles.

## 9. Impact notes

- Data model impact: adds a `scope` field to `Extension` (back-compatible via default); no new stores.
- Security/privacy impact: surfaces the extension's claimed scope at the trusted approval moment (ADR-0002); enforcement remains the command gate (ADR-0014). No sandbox, no capability.
- Observability impact: the scope is visible at approval and in the event log.
- Performance impact: negligible.
- Migration/backward compatibility impact: existing registries load with `scope: read`; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: implying the scope is enforced/sandboxed. Mitigation: the spec and UI frame it as a declared claim; the gate is the enforcement; sandboxing is explicitly out of scope.
- Risk: breaking existing registries. Mitigation: `scope` defaults to `read` on load; tested.
- Dependency: FS-037 extension registry, FS-035 command gate.

## 11. Open questions

None. This slice declares and surfaces an extension permission scope at the gate; runtime sandboxing is deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-045 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-046-extension-permissions`
- Expected implementation PR title: `feat(FS-046): Extension permission scope`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-046/amendments/`.
