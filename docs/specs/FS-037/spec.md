---
spec_id: FS-037
title: Extension registry
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
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
  - docs/adrs/ADR-0014-supervised-command-execution.md
---

# FS-037: Extension registry

## 1. Problem statement

The PRD's Phase 5 makes Synth extensible through process-based extensions — MCP servers, skills, custom tools — that provide capability but not authority, routed through the trusted core (PRD §16, ADR-0002). The foundation is a registry: a place to declare extensions (name, kind, launch command) so they can later be run. With the gated command runner (FS-035) already in place, a declared extension can be run through that same supervised boundary.

This story adds the registry: declare extensions, list them, and run one via the existing command gate. It stores declarations in app-local storage (ADR-0003), not the repo. It does not auto-run anything, broker MCP protocol, or grant any new authority — running an extension is just a gated command.

## 2. Requirements

- R1. The Rust core must expose `register_extension(name: String, kind: String, command: String) -> Result<Extension, String>` that validates the fields and appends a new extension to the app-local registry, returning it.
- R2. `Extension` must serialize in camelCase with at least: `id`, `name`, `kind`, and `command`.
- R3. The kind must be one of a fixed set (`tool`, `mcp`, `skill`); validating the kind must be a pure, unit-testable function. The name and command must be non-empty and within length caps.
- R4. The core must expose `list_extensions() -> Vec<Extension>` returning the declared extensions, and `remove_extension(id: u64) -> Result<(), String>` removing one by id.
- R5. The registry must be stored in the OS app-data directory (resolved via the Tauri path API), never in the workspace/repo. Loading/saving the registry to a path must be pure-ish, unit-testable functions; a missing/malformed registry loads as empty.
- R6. Running an extension must go through the existing command gate (FS-035): a renderer "Run" reuses `request_run_command(extension.command)`, which requires explicit approval and runs in the jailed workspace. This story adds no new execution path and no new authority.
- R7. The renderer must list the declared extensions, provide a form to register one (name, kind, command), allow removing one, and run one via the command gate. Registry display is transient renderer state derived from the core.
- R8. This story must not auto-run any extension, must not broker MCP/tool protocols, must not write to the workspace/repo, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `register_extension("ripgrep", "tool", "rg --version")` returns an `Extension` with an id and the fields and persists it; `list_extensions` then includes it.
- AC2. `register_extension` with an invalid kind, empty name, or empty command returns `Err` and registers nothing.
- AC3. `remove_extension(id)` removes the extension; `list_extensions` no longer includes it; removing an unknown id returns `Err`.
- AC4. With no registry file, `list_extensions` returns an empty list (no error).
- AC5. Running an extension from the renderer issues a `request_run_command` for the extension's command, which requires explicit approval (never auto-approved, per FS-035) and runs in the jailed workspace.
- AC6. The registry file is in the app-data directory, not the workspace/repo.
- AC7. Rust unit coverage verifies kind validation, the load/save round-trip against a temp file (including remove and malformed-tolerance), and camelCase serialization.
- AC8. No code in this story auto-runs an extension, brokers a protocol, writes to the workspace/repo, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: load/save/remove are tested against a temporary registry file; kind validation is pure. No network or workspace I/O in tests.

Manual checks:

- Register an extension, confirm it lists; remove it, confirm it’s gone.
- Run an extension and confirm it goes through the approval surface (gated), then shows output.
- Register with an invalid kind and confirm a calm validation error.
- Confirm the registry file is under the app-data directory and `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the registry with an extension and the approval-gated run.
- Short note confirming app-local storage, gated run (no new authority), and unchanged capabilities.

## 5. Success criteria

- SC1. Extensions can be declared, listed, and removed in app-local storage.
- SC2. Running an extension reuses the gated command boundary — no new authority.
- SC3. The registry is private operational state, never in the repo.
- SC4. No auto-run, protocol brokering, workspace write, or new capability is introduced.
- SC5. The slice stays story-sized and is the foundation for later MCP/skill brokering.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Registry CRUD | register/list/remove correct; malformed tolerated | Rust tests | @BrendanShields |
| Kind validation | only tool/mcp/skill accepted | Rust tests | @BrendanShields |
| Gated run | run goes through approval; never auto-approved | Source review / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 auto-run, 0 workspace writes, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-036 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `register_extension` / `list_extensions` / `remove_extension` over an app-local registry.
- An `Extension` camelCase shape; pure kind validation and load/save helpers.
- A renderer registry list, register form, remove, and gated run.

### Out of scope

- MCP protocol brokering, tool schema discovery, or subagent orchestration.
- Auto-running extensions or running them outside the command gate.
- Capability grants per extension or policy beyond the existing command gate.
- Editing extensions (remove + re-register suffices here).
- Bundling or installing extensions.

## 8. Technical design

### Rust/Tauri core

Add an `extensions` module:

```text
Extension { id, name, kind, command }                 // serde camelCase, (de)serializable
is_valid_extension_kind(kind) -> bool                  // pure: tool | mcp | skill
load_registry(path) -> Vec<Extension>                  // [] if missing/malformed
save_registry(path, &[Extension]) -> Result<(), String>
register_extension(app, name, kind, command) -> Result<Extension, String>   // #[tauri::command]
list_extensions(app) -> Vec<Extension>                  // #[tauri::command]
remove_extension(app, id) -> Result<(), String>         // #[tauri::command]
```

The registry is a JSON array at `{app_data_dir}/extensions.json`. `register_extension` validates, loads, assigns the next id (max + 1), pushes, and saves. `remove_extension` loads, removes by id (Err if absent), saves. Running is not added here — the renderer reuses `request_run_command`.

### React renderer

Add an extensions surface: a list (name · kind · command) with Remove and Run per row, and a register form (name, kind select, command). Run calls the existing `request_run_command(command)` flow (gated). Refresh the list after register/remove.

### Styling

Reuse list/control styles.

## 9. Impact notes

- Data model impact: introduces an `Extension` IPC shape and an app-local JSON registry; private operational state (ADR-0003).
- Security/privacy impact: declarations are app-local (never the repo); running an extension is a gated command with no new authority (ADR-0002, ADR-0014). No capability added.
- Observability impact: register/remove/run can be noted in the event log.
- Performance impact: negligible; small registry file.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: an extension command being dangerous. Mitigation: running goes through the command gate (explicit approval, jailed, bounded); no auto-run.
- Risk: registry corruption. Mitigation: malformed registry loads empty; saves are whole-file writes.
- Risk: scope creep into MCP brokering. Mitigation: this slice only declares/lists/runs-as-command; protocol brokering is a later spec.
- Dependency: FS-032 (app-local storage pattern), FS-035 (gated command run).

## 11. Open questions

None. This slice provides an app-local extension registry with gated run; MCP/skill brokering and capability grants are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-036 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-037-extension-registry`
- Expected implementation PR title: `feat(FS-037): Extension registry`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-037/amendments/`.
