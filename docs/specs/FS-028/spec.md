---
spec_id: FS-028
title: Configurable provider settings
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
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-028: Configurable provider settings

## 1. Problem statement

The provider is hardcoded (`localhost:11434`, `gemma4:e4b`). The PRD's BYOK strategy (PRD §14, ADR-0008) requires the user to configure the provider — at minimum the base URL and model. This story makes the provider configuration live, trusted core state so the user can point Synth at a different local Ollama endpoint or model, with all generation and status using the configured values.

This slice keeps the existing Ollama API shape (a later spec adds OpenAI-compatible request/response shapes); it makes the *configuration* settable. The default remains the current local Ollama + `gemma4:e4b`, so existing behaviour is unchanged until the user changes it.

## 2. Requirements

- R1. The Rust core must hold the provider configuration as live state in Tauri managed state, defaulting to the current values (`kind: ollama`, `baseUrl: http://localhost:11434`, `model: gemma4:e4b`).
- R2. The core must expose `get_provider_config() -> ProviderConfig` and `set_provider_config(baseUrl: String, model: String) -> Result<ProviderConfig, String>` that validates and stores the base URL and model and returns the new config.
- R3. `ProviderConfig` must serialize in camelCase with at least `kind`, `baseUrl`, and `model` (matching the existing shape).
- R4. Validation must be pure and unit-testable: the base URL must be a non-empty `http://` or `https://` URL with no whitespace; the model must be non-empty and within a reasonable length. Invalid input returns a readable `Err` and does not change the stored config.
- R5. `get_provider_status`, `ask_model`, `ask_spec`, `ask_stream`, and `draft_spec` must use the configured provider values from managed state rather than a hardcoded constant. Behaviour with the default config must be identical to today.
- R6. The provider config is not persisted to disk in this slice (process-lifetime state); credential handling remains out of scope (the Ollama path needs none).
- R7. The renderer must show the current provider config and let the user edit the base URL and model, calling `set_provider_config` and reflecting the result and validation errors. The provider status indicator (FS-007) must continue to reflect reachability of the configured provider.
- R8. This story must not add new Tauri capability permissions (generation remains a Rust-side HTTP call), must not change the FS-001 runtime status contract, and must not break any existing command.

## 3. Acceptance criteria

- AC1. `get_provider_config()` returns the default `ollama` / `http://localhost:11434` / `gemma4:e4b`.
- AC2. `set_provider_config("http://localhost:11434", "llama3:8b")` returns the updated config and a subsequent `get_provider_config()` reflects it.
- AC3. `set_provider_config` with an invalid base URL (empty, `ftp://x`, contains a space) or empty model returns `Err` and does not change the stored config.
- AC4. After changing the model, `get_provider_status` reports presence for the configured model, and `ask_model`/`draft_spec` use the configured base URL and model.
- AC5. The renderer edits and saves provider settings, reflects validation errors, and the status indicator reflects the configured provider.
- AC6. Rust unit coverage verifies base-URL/model validation (valid and each invalid class) and that the config functions round-trip; existing provider tests remain intact with the default config.
- AC7. No code in this story adds a Tauri capability, persists the config to disk, handles credentials, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: validation and config round-trip are unit-tested without network. Live generation against the configured provider is verified manually/eval with the default model.

Manual checks:

- Confirm the default provider config shows and `gemma4:e4b` is reachable.
- Change the model to another pulled model and confirm the status updates and an ask uses it.
- Enter an invalid base URL and confirm a calm validation error and no change.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- A note/eval showing generation against the configured (default) provider still works.
- Short note confirming no capability/persistence/credential changes and the default behaviour is unchanged.

## 5. Success criteria

- SC1. The provider base URL and model are configurable live core state (BYOK, ADR-0008).
- SC2. All generation and status use the configured values; the default is unchanged.
- SC3. Configuration is validated; invalid input is rejected calmly.
- SC4. No persistence, credentials, or new capability are introduced.
- SC5. The slice stays story-sized and does not add OpenAI-compatible API shapes or model-role assignment.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Config round-trip | get/set reflect changes; default correct | Rust tests | @BrendanShields |
| Validation | invalid base URL/model rejected; config unchanged | Rust tests | @BrendanShields |
| Used everywhere | status + ask + draft use the configured values | Source review / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 persistence, 0 credentials, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-027 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Provider config as live managed state with `get_provider_config` / `set_provider_config` and pure validation.
- Threading the configured values through `get_provider_status`, `ask_model`, `ask_spec`, `ask_stream`, `draft_spec`.
- A renderer provider-settings control (base URL + model).
- Rust unit tests for validation and round-trip.

### Out of scope

- OpenAI-compatible request/response/streaming shapes (a later spec).
- Credential/API-key handling and secret storage.
- Model-role assignment (planner/builder/etc.).
- Persisting the config across sessions.
- Adding/removing providers or a provider marketplace.

## 8. Technical design

### Rust/Tauri core

Introduce a `ProviderState(Mutex<ProviderConfig>)` managed in Tauri state, defaulting to the current values. Replace `default_provider_config()` call sites in `get_provider_status`, `ask_model`, `ask_spec`, `ask_stream`, and `draft_spec` with a read of the managed state. Add `get_provider_config(state)` and `set_provider_config(state, base_url, model)` commands; `set_provider_config` validates via a pure `is_valid_base_url` and a model-length check, updating the state on success. Keep `default_provider_config()` as the default constructor for the state.

Async commands that need the config read it from state at the start (cloning the small `ProviderConfig`) so no lock is held across awaits.

### React renderer

Add a provider-settings control (base URL + model inputs) near the provider status line; on save call `set_provider_config`, reflect the result or validation error, and refresh the provider status. Keep it calm and minimal.

### Styling

Reuse the workspace control styles for the inputs; keep the provider line quiet.

## 9. Impact notes

- Data model impact: provider config becomes managed state; no new IPC shape beyond the existing `ProviderConfig`; nothing persisted.
- Security/privacy impact: no credentials (Ollama needs none); generation remains a localhost-by-default Rust HTTP call; a user-set base URL could point elsewhere, but no credentials are sent and policy/credential handling is a later phase. No capability added.
- Observability impact: config changes can be noted in the FS-011 session log.
- Performance impact: negligible.
- Migration/backward compatibility impact: additive; the default config preserves current behaviour.

## 10. Risks and dependencies

- Risk: a user-set base URL pointing at an unexpected host. Mitigation: validation restricts to http(s) URLs; no credentials are sent; remote-provider policy is a later phase.
- Risk: threading the config breaks existing generation. Mitigation: the default equals today's constants; existing provider tests run with the default.
- Risk: holding a lock across await. Mitigation: read and clone the config before awaiting.
- Dependency: FS-007/FS-008/FS-010/FS-024 (provider status, generation, streaming, drafting).

## 11. Open questions

None. This slice makes the Ollama provider base URL and model configurable; OpenAI-compatible shapes, credentials, and roles are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-027 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-028-configurable-provider`
- Expected implementation PR title: `feat(FS-028): Configurable provider settings`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-028/amendments/`.
