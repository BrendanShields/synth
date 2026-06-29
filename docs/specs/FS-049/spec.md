---
spec_id: FS-049
title: Subagent definitions
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0008-byok-provider-strategy.md
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
---

# FS-049: Subagent definitions

## 1. Problem statement

The PRD's Phase 5 calls for subagent definitions (PRD §16): named, reusable agent personas the harness can run for focused tasks. Synth already has model roles (FS-031) and a provider path (FS-008/FS-029); a subagent composes them — a name, a role (whose model it uses), and a system instruction — into a reusable persona. This story adds defining subagents (app-local) and running one against the provider with its role's model and instruction.

Definitions are app-local operational config (ADR-0003). Running a subagent is a single read-only generation grounded by the subagent's instruction; it executes no commands and writes nothing.

## 2. Requirements

- R1. The Rust core must expose `save_subagent(name, role, instructions) -> Result<Subagent, String>` validating the fields (name non-empty/capped; role one of the known roles via FS-031; instructions non-empty/capped) and appending to an app-local registry.
- R2. `Subagent` must serialize camelCase with at least `id`, `name`, `role`, and `instructions`.
- R3. The core must expose `list_subagents() -> Vec<Subagent>` and `remove_subagent(id) -> Result<(), String>`; the registry lives in the app-data directory (never the repo) and tolerates a missing/malformed file (loads empty). Load/save to a path must be unit-testable.
- R4. The core must expose `run_subagent(id, input) -> Result<String, String>` that resolves the subagent's role model (FS-031 override/default), builds a prompt from the subagent instruction plus the input, and returns the model's answer via the existing provider generation path. Building the subagent prompt must be a pure, unit-testable function.
- R5. Running a subagent must be read-only: a single localhost model generation; it must not execute commands, write files, or perform git/non-model network operations.
- R6. The renderer must let the user define subagents (name, role, instructions), list/remove them, and run one with an input, showing the answer. Display is transient renderer state.
- R7. This story must not add new Tauri capability permissions and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `save_subagent("Reviewer", "adversary", "Critique the input.")` returns a `Subagent` and persists it; `list_subagents` includes it.
- AC2. `save_subagent` with an empty name, unknown role, or empty instructions returns `Err` and stores nothing.
- AC3. `remove_subagent(id)` removes it; an unknown id returns `Err`; a missing registry lists empty.
- AC4. `build_subagent_prompt(instructions, input)` includes both the instruction and the input.
- AC5. `run_subagent(id, input)` uses the subagent's role model and returns an answer (verified live in the eval).
- AC6. Running a subagent performs no command execution, file write, or non-model network call.
- AC7. Rust unit coverage verifies field validation (incl. role check), the registry round-trip (save/list/remove, malformed-tolerance), the prompt builder, and camelCase serialization.
- AC8. No code in this story executes commands, writes the repo, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: validation, registry I/O (temp file), and the prompt builder are unit-tested; `cargo test` does no network. The run is verified by the eval.

Eval (required, attached to the PR): with local Ollama running `gemma4:e4b`, define a subagent (e.g. a terse summarizer) and run it on a sample input; confirm the answer reflects the subagent's instruction.

Manual checks:

- Define a subagent, run it, and confirm the persona/instruction shapes the answer.
- Define with an unknown role and confirm a calm validation error.
- Remove a subagent and confirm it’s gone.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- The eval: the subagent definition, the input, and the answer.
- Short note confirming read-only run (no command exec) and unchanged capabilities.

## 5. Success criteria

- SC1. Reusable subagent personas can be defined, listed, removed, and run.
- SC2. A subagent runs with its role's model and instruction.
- SC3. Running is read-only (a single generation), executing nothing.
- SC4. Definitions are app-local; no new capability.
- SC5. The slice stays story-sized — single-shot personas; multi-step orchestration and tool use are deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Definition validity | name/role/instructions validated | Rust tests | @BrendanShields |
| Registry CRUD | save/list/remove; malformed tolerated | Rust tests | @BrendanShields |
| Run grounding | answer reflects the instruction | Eval | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 command exec, 0 repo writes, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-048 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- `save_subagent` / `list_subagents` / `remove_subagent` over an app-local registry.
- A `Subagent` shape, pure validation and load/save, and a pure prompt builder.
- `run_subagent` (single generation via the role model).
- A renderer define/list/remove/run surface.

### Out of scope

- Multi-step subagent orchestration, tool use, or command execution by subagents.
- Subagents calling other subagents, or shared memory/state.
- Streaming subagent output (single-shot here).
- Per-subagent provider overrides beyond the role model.

## 8. Technical design

### Rust/Tauri core

Add a `subagents` module mirroring the registry pattern:

```text
Subagent { id, name, role, instructions }              // serde camelCase, (de)serializable
validate_subagent(name, role, instructions) -> Result<(), String>   // pure (role via crate::roles)
build_subagent_prompt(instructions, input) -> String    // pure
load_store(path) / save_store(path, &[Subagent])
save_subagent / list_subagents / remove_subagent        // commands (app-data store)
run_subagent(provider, roles, id, input) -> Result<String, String>   // command
```

`run_subagent` loads the subagent, resolves its role model via `crate::roles::resolve_model_for_role`, builds the prompt, and generates via the shared provider path. The store is `{app_data_dir}/subagents.json`.

### React renderer

Add a subagents surface: a define form (name, role select, instructions), a list with Remove and a Run (with an input), and the answer display. Transient state.

### Styling

Reuse list/control/answer styles.

## 9. Impact notes

- Data model impact: introduces a `Subagent` shape and an app-local store; reuses roles + provider.
- Security/privacy impact: app-local definitions; running is one localhost generation (default Ollama on-device); no command exec, no repo write, no capability.
- Observability impact: subagent runs can be noted in the event log.
- Performance impact: one generation per run.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: implying subagents can act (run tools). Mitigation: a subagent is a single read-only generation; tool use/orchestration is out of scope.
- Risk: registry corruption. Mitigation: malformed store loads empty; whole-file writes.
- Risk: sending input to a remote provider. Mitigation: default provider is local Ollama; remote is the user's BYOK choice.
- Dependency: FS-031 roles, FS-008/FS-029 provider generation, FS-037/FS-038 registry pattern.

## 11. Open questions

None. This slice defines and runs single-shot subagent personas; multi-step orchestration and tool use are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-048 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-049-subagent-definitions`
- Expected implementation PR title: `feat(FS-049): Subagent definitions`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-049/amendments/`.
