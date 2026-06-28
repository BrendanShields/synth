---
spec_id: FS-007
title: Ollama provider status
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
  - docs/adrs/ADR-0008-byok-provider-strategy.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-007: Ollama provider status

## 1. Problem statement

The walking skeleton (FS-001..FS-006) is entirely in-memory: the shell renders artifacts but cannot yet reach a model. The PRD's Phase 1 promises a Rust-owned provider abstraction with an Ollama provider (PRD §14, §15; ADR-0008). Before generation, the safest first slice is read-only provider *status*: can the trusted core reach the configured local Ollama, and is the configured model present?

This story adds a Rust-owned provider module with a single static default configuration (local Ollama) and one read-only command that queries Ollama's model list over localhost HTTP and reports a typed status. The renderer shows a quiet provider line. No generation, no streaming, no credentials, no user-entered endpoints, and no remote hosts yet.

This is deliberately the first outbound-network slice and it is kept minimal: localhost only, GET only, static configuration. Arbitrary/remote providers, credential handling, and policy gating arrive with the Phase 2 security kernel.

## 2. Requirements

- R1. The Rust core must own a provider module exposing a single static default `ProviderConfig` with at least: `kind` (`ollama`), `baseUrl` (`http://localhost:11434`), and `model` (`gemma4:e4b`). It must serialize to React in camelCase.
- R2. The Rust core must expose an async Tauri command `get_provider_status()` that performs a read-only `GET {baseUrl}/api/tags` against the configured Ollama endpoint and returns a typed `ProviderStatus`.
- R3. `ProviderStatus` must serialize in camelCase with these fields:
  - `kind`
  - `baseUrl`
  - `model`
  - `state` (`reachable` or `unreachable`)
  - `modelPresent` (boolean)
  - `availableModels` (array of model name strings)
  - `detail` (short human-readable status/explanation)
- R4. When the endpoint is reachable, `state` must be `reachable`, `availableModels` must list the model names returned by Ollama, and `modelPresent` must be true only if the configured `model` is among them.
- R5. When the endpoint is unreachable or returns an error, `get_provider_status` must not panic or fail the command; it must return `state: unreachable`, `availableModels: []`, `modelPresent: false`, and a readable `detail`.
- R6. Parsing the Ollama `/api/tags` response body into model names must be a pure, synchronous, unit-testable function with no network or async dependency. Model-presence comparison must be exact on the configured model name.
- R7. The renderer must show a single quiet provider status surface displaying, at minimum, the model and whether it is reachable. It must be calm and minimal: no verbose diagnostics, raw JSON, or noisy chrome.
- R8. This story must not add credential handling, user-entered or remote endpoints, request bodies, generation, streaming, shell execution, persistence, policy decisions, or new Tauri capability permissions. Network access is limited to a localhost GET to the static Ollama base URL.
- R9. This story must not change the FS-001 runtime status contract and must not change or remove any existing Tauri command (`parse_command`, `route_command`, `list_specs_index`, `get_static_spec_detail`).

## 3. Acceptance criteria

- AC1. `get_provider_status` reports `kind: ollama`, `model: gemma4:e4b`, and `baseUrl: http://localhost:11434`.
- AC2. Given a running local Ollama that has the configured model, when `get_provider_status` is called, then `state: reachable`, `modelPresent: true`, and the model appears in `availableModels`.
- AC3. Given no reachable Ollama, when `get_provider_status` is called, then `state: unreachable`, `modelPresent: false`, `availableModels: []`, and `detail` is a readable message, with no panic or unhandled command error.
- AC4. The pure parse function extracts the model-name list from a representative `/api/tags` JSON body, and the presence check returns true for the configured model and false for an absent one.
- AC5. The renderer shows the provider model and a reachable/unreachable indication quietly; the unreachable state reads as a calm message, not an error dump.
- AC6. `ProviderConfig` and `ProviderStatus` serialize in camelCase.
- AC7. No code in this story adds credentials, remote/user endpoints, generation, streaming, persistence, new Tauri capabilities, or changes existing command contracts.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `cargo test` must not perform network I/O. Only the pure parse/presence logic is unit-tested; live connectivity is verified manually.

Manual checks required before the implementation PR is ready:

- With local Ollama running and the model pulled, run the app and confirm the provider surface shows the model as reachable.
- Stop Ollama (or point at an unused port in a scratch build) and confirm the surface shows a calm unreachable state without crashing.
- Confirm `src-tauri/capabilities/*.json` gains no new filesystem, shell, dialog, or credential permissions.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Output of a live `get_provider_status` (or equivalent curl to `/api/tags`) showing the model reachable.
- Short note confirming localhost-only GET, no credentials/remote endpoints, and unchanged existing contracts.

## 5. Success criteria

- SC1. The trusted core can report whether the configured local Ollama provider is reachable and whether the model is present.
- SC2. The provider status is typed, camelCase, and rendered quietly in the shell.
- SC3. Unreachable providers degrade gracefully with no panic.
- SC4. The network surface stays minimal: localhost GET, static config, no credentials.
- SC5. The slice stays story-sized and does not become generation, streaming, or a provider settings UI.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Status correctness | Reachable + modelPresent true when Ollama has the model | Live status output | @BrendanShields |
| Graceful failure | Unreachable returns typed status with no panic | Rust review / manual | @BrendanShields |
| Parse coverage | Pure parse + presence logic covered by Rust tests | `cargo test` output | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 credentials, 0 remote endpoints, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-006 commands/contracts unchanged | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A Rust provider module with a static default Ollama `ProviderConfig`.
- An async `get_provider_status()` command performing a localhost GET to `/api/tags`.
- A typed, camelCase `ProviderStatus` with reachable/unreachable handling.
- A pure parse + model-presence function with unit tests.
- A quiet renderer surface for provider model + reachability.

### Out of scope

- Text generation, chat, or streaming.
- User-entered, remote, or multiple providers; provider settings UI.
- Credential/API-key handling and secret storage.
- Model selection or role assignment.
- Policy/approval gating of network access (Phase 2).
- Persistence of provider config or status.
- OpenAI-compatible provider (later spec).

## 8. Technical design

### Rust/Tauri core

Add a `provider` module. Introduce direct dependencies `reqwest` (with JSON and rustls TLS) and `tokio`; both already resolve transitively via Tauri.

Types:

```text
ProviderConfig { kind, base_url, model }   // serde camelCase
ProviderStatus { kind, base_url, model, state, model_present, available_models, detail }
```

`default_provider_config()` returns the static Ollama config. A pure function parses an Ollama `/api/tags` JSON body into `Vec<String>` model names, and a pure helper computes model presence against the configured model.

```text
get_provider_status() -> ProviderStatus   // async #[tauri::command]
```

The command does a localhost GET to `/api/tags` with a short timeout. Any transport/parse error maps to a `state: unreachable` status with a readable `detail`; it never returns `Err`. Register it in `tauri::generate_handler!`.

### React renderer

Add a small, quiet provider status surface (a single line: model and reachable/unreachable). Fetch `get_provider_status` once on load into transient state, mirroring the existing runtime-status fetch pattern. Keep copy minimal and calm per the shell's editorial, spacious aesthetic.

### Styling

Reuse existing muted/mono text styles. Add only what is needed for a single quiet status line. No badges with high-saturation colors, no raw diagnostics.

## 9. Impact notes

- Data model impact: introduces `ProviderConfig`/`ProviderStatus` IPC shapes; no persisted entities or migrations.
- Security/privacy impact: first outbound network call, limited to a localhost GET to a static base URL; no credentials, no remote hosts, no request bodies. Policy gating arrives in Phase 2.
- Observability impact: status is in-session only; no event store or audit log yet.
- Performance impact: one short-timeout GET per status fetch; negligible.
- Migration/backward compatibility impact: additive; all FS-001..FS-006 contracts unchanged.

## 10. Risks and dependencies

- Risk: outbound network could be seen as crossing a trust boundary prematurely. Mitigation: localhost-only, GET-only, static config, no credentials; remote/policy handling explicitly deferred to Phase 2.
- Risk: live network in tests would be flaky. Mitigation: only pure parse/presence logic is unit-tested; connectivity is verified manually.
- Risk: a hung endpoint could stall the command. Mitigation: a short request timeout and graceful unreachable status.
- Dependency: FS-006 implementation merged; a local Ollama with the configured model for manual verification.

## 11. Open questions

None. This slice intentionally implements read-only status for a single static local Ollama provider.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-006 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-007-ollama-provider-status`
- Expected implementation PR title: `feat(FS-007): Ollama provider status`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-007/amendments/`.
