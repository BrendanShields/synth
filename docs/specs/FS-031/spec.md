---
spec_id: FS-031
title: Model-role assignment
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
  - docs/adrs/ADR-0008-byok-provider-strategy.md
---

# FS-031: Model-role assignment

## 1. Problem statement

The PRD's provider strategy stores a default model plus optional per-role model overrides (PRD §14.1): planner, builder, adversary, summarizer, and requirements-critic. Synth has a configurable default model (FS-028/FS-029) but no role assignments. This story adds role→model overrides as trusted core state, resolution (a role uses its override when set, otherwise the default model), and one concrete consumer so roles demonstrably affect generation: spec drafting uses the `planner` role.

Auto-selection by role/task/cost (PRD §14.2) remains a roadmap item; this slice establishes the manual default + role-override model the PRD requires for v1.

## 2. Requirements

- R1. The Rust core must hold per-role model overrides as live managed state for the fixed role set: `planner`, `builder`, `adversary`, `summarizer`, `requirements_critic`. An unset role has no override.
- R2. Resolving a role's model must be a pure, unit-testable function: it returns the role's override when set, otherwise the configured default model.
- R3. The core must expose `get_model_roles() -> Vec<RoleAssignment>` returning, for each role, its resolved `model` and whether it is `overridden`. `RoleAssignment` serializes in camelCase with `role`, `model`, `overridden`.
- R4. The core must expose `set_model_role(role: String, model: String) -> Result<(), String>` that validates the role (rejecting unknown roles) and either sets the override (non-empty model) or clears it (empty/whitespace model). Validating the role name must be a pure, unit-testable function.
- R5. `draft_spec` must resolve and use the `planner` role's model (override or default) for generation, demonstrating role consumption without changing other commands.
- R6. Role overrides must be process-lifetime state only: not persisted to disk; no credentials are involved.
- R7. The renderer must show the role assignments (role + resolved model + whether overridden) and let the user set or clear a role override, calling `set_model_role` and reflecting the result.
- R8. This story must not add new Tauri capability permissions, must not change the FS-001 runtime status contract, and must not alter existing generation behaviour beyond `draft_spec` resolving the planner role (which defaults to the same default model when unset).

## 3. Acceptance criteria

- AC1. With no overrides, `get_model_roles` returns all five roles with `model` equal to the configured default and `overridden: false`.
- AC2. `set_model_role("planner", "llama3:8b")` then `get_model_roles` shows planner `model: llama3:8b`, `overridden: true`, and the others unchanged.
- AC3. `set_model_role("planner", "")` clears the override; planner resolves to the default again.
- AC4. `set_model_role("bogus", "x")` returns `Err` and changes nothing.
- AC5. `draft_spec` uses the resolved planner model; with no planner override it is identical to today (default model). (Verified by the eval against the default.)
- AC6. The renderer shows the five roles and lets the user set/clear an override, reflecting the resolved model.
- AC7. Rust unit coverage verifies role-name validation, role resolution (override vs default), and camelCase serialization; existing generation tests run on the default.
- AC8. No code in this story persists overrides to disk, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks:

- Confirm the five roles show the default model and `overridden: false` initially.
- Override `planner`, draft a spec, and confirm it still works (using the planner model); clear it and confirm it reverts.
- Set an invalid role and confirm rejection.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- An Ollama eval confirming `draft_spec` still works via the planner role with the default model.
- Short note confirming overrides are in-memory, no capability/persistence change, and other commands are unchanged.

## 5. Success criteria

- SC1. Synth stores a default model plus optional per-role overrides (PRD §14.1).
- SC2. A role resolves to its override or the default, by a tested pure function.
- SC3. At least one consumer (`draft_spec`/planner) demonstrates roles affecting generation.
- SC4. No persistence, credentials, or new capability is introduced.
- SC5. The slice stays story-sized and does not implement auto-selection (§14.2) or wire every command to a role.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Resolution correctness | role → override or default, per the rule | Rust tests | @BrendanShields |
| Role validation | unknown role rejected; set/clear works | Rust tests | @BrendanShields |
| Consumer wired | draft_spec uses the planner model | Source review / eval | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 persistence, 0 credentials, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-030 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Per-role model overrides as managed state for the five roles.
- Pure role-name validation and role resolution.
- `get_model_roles` / `set_model_role` commands.
- Wiring `draft_spec` to the `planner` role.
- A renderer control for role overrides.

### Out of scope

- Auto-selection by role/task/cost/privacy/latency (PRD §14.2, roadmap).
- Wiring every command to a role (only `draft_spec`/planner in this slice).
- Persisting role assignments or project-level config.
- Capability-aware suggestions or outcome-informed recommendations.

## 8. Technical design

### Rust/Tauri core

Add a `ModelRolesState(Mutex<HashMap<String,String>>)` managed in Tauri state (role → override model). A `ROLES` constant lists the five roles; `is_valid_role(role)` checks membership. `resolve_model_for_role(role, overrides, default_model) -> String` returns the override or the default (pure). `get_model_roles` reads the provider default (from `ProviderState`) and the overrides, returning a `RoleAssignment { role, model, overridden }` per role. `set_model_role` validates the role and sets/clears the override. `draft_spec` resolves the planner model and generates with a config whose model is the resolved planner model (cloning the provider config and overriding `model`).

### React renderer

Add a roles control listing the five roles with their resolved model and an input to set/clear an override; saving calls `set_model_role` and refreshes the list. Keep it calm and minimal.

### Styling

Reuse the provider/workspace control styles.

## 9. Impact notes

- Data model impact: introduces a `RoleAssignment` IPC shape and an in-memory role-override map; nothing persisted.
- Security/privacy impact: none beyond existing generation; no credentials, no capability.
- Observability impact: role changes can be noted in the FS-011 session log.
- Performance impact: negligible.
- Migration/backward compatibility impact: additive; with no overrides, behaviour is identical to today.

## 10. Risks and dependencies

- Risk: implying auto-selection. Mitigation: this slice is manual overrides only; auto-select is roadmap.
- Risk: only one consumer wired. Mitigation: that is intentional for story size; resolution is general and reusable.
- Dependency: FS-028/FS-029 (configurable provider default) and FS-024 (draft_spec).

## 11. Open questions

None. This slice provides default + per-role model overrides with one consumer; auto-selection is deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-030 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-031-model-roles`
- Expected implementation PR title: `feat(FS-031): Model-role assignment`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-031/amendments/`.
