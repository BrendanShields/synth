---
spec_id: FS-029
title: OpenAI-compatible provider shape
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

# FS-029: OpenAI-compatible provider shape

## 1. Problem statement

FS-028 made the provider base URL and model configurable, but only the Ollama API shape is implemented. The PRD names OpenAI-compatible endpoints as a first-class v1 provider family (PRD §14, ADR-0008). This story adds the OpenAI-compatible request/response shape alongside Ollama, selected by the provider `kind`, so a user can point Synth at any OpenAI-compatible endpoint (with a bring-your-own API key).

The two shapes differ: Ollama uses `POST /api/generate` with a `prompt` and returns `response`; OpenAI-compatible uses `POST /v1/chat/completions` with a `messages` array and a `Bearer` key, returning `choices[].message.content`. This slice adds the OpenAI shape for non-streaming generation (`ask_model`, `ask_spec`, `draft_spec`) and status, and makes the `?` streaming path fall back to a single emitted chunk for the OpenAI kind (true SSE streaming for OpenAI is a later refinement). Ollama behaviour is unchanged and remains the default.

## 2. Requirements

- R1. The provider config must support a `kind` of `ollama` or `openai`, with the existing fields (`baseUrl`, `model`) plus an optional API key held in core state (not persisted to disk). `set_provider_config` must accept and validate the kind and store the optional key.
- R2. Building the generation request must be pure and kind-specific and unit-testable:
  - `ollama`: `POST {baseUrl}/api/generate` with `{ model, prompt, stream: false }`, no auth header.
  - `openai`: `POST {baseUrl}/v1/chat/completions` with `{ model, messages: [{ role: "user", content: prompt }], stream: false }` and an `Authorization: Bearer <key>` header when a key is set.
- R3. Parsing the non-streaming response must be pure and kind-specific and unit-testable:
  - `ollama`: read `response`.
  - `openai`: read `choices[0].message.content`.
  A malformed or empty body for either kind must return a typed error, not a panic.
- R4. `ask_model`, `ask_spec`, and `draft_spec` must generate using the configured kind via the shared generation path, returning a readable `Err` on transport/parse failure without panicking.
- R5. `get_provider_status` must reflect the configured kind: for `ollama` it queries `/api/tags` (as today); for `openai` it queries `GET {baseUrl}/v1/models` with the `Bearer` key and lists model ids. Unreachable or unauthorized endpoints return a typed `unreachable` status, not an error.
- R6. The `?` streaming command (`ask_stream`) must keep real token streaming for `ollama`; for `openai` it must perform a single non-streaming generation and emit the result as one chunk followed by the done event (no partial/incorrect SSE handling). The event contract (chunk/done/error tagged by requestId) is unchanged.
- R7. Validation of the kind must be pure and reject values other than `ollama`/`openai`. The base-URL validation from FS-028 still applies.
- R8. This story must not persist the API key or config to disk, must not log the API key, must not add new Tauri capability permissions (generation remains a Rust-side HTTP call), and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `build_request_body` for `ollama` produces `{ model, prompt, stream:false }`; for `openai` produces `{ model, messages:[{role:"user",content:prompt}], stream:false }`.
- AC2. `parse_answer` for `ollama` reads `response`; for `openai` reads `choices[0].message.content`; both return a typed error for a malformed/empty body.
- AC3. The OpenAI status query parses `data[].id` from a representative `/v1/models` body into the available-models list and computes `modelPresent` for the configured model.
- AC4. With kind `ollama` (default), all generation and status behave exactly as before (verified by existing tests + the Ollama eval).
- AC5. `set_provider_config(kind, baseUrl, model, apiKey)` stores the kind and key; an invalid kind returns `Err` and does not change the config.
- AC6. The renderer lets the user choose the provider kind and (for openai) enter an API key; the status indicator reflects the configured provider.
- AC7. Rust unit coverage verifies both request builders, both answer parsers (success + malformed), the OpenAI models parser, and kind validation. No test contacts a real OpenAI endpoint; the OpenAI path is exercised by pure builders/parsers only.
- AC8. No code in this story persists or logs the API key, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: the OpenAI path is unit-tested via pure builders/parsers against representative JSON; no live OpenAI request is made in tests. The Ollama path is verified live with `gemma4:e4b` (default behaviour unchanged).

Manual checks:

- With the default Ollama config, confirm status and an ask still work against `gemma4:e4b`.
- Set kind `openai` with a base URL + key for an OpenAI-compatible endpoint and confirm an ask returns content and status lists models. (Manual, with the user's own endpoint/key.)
- Confirm an invalid kind is rejected and the API key is never written to disk or logged.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- An Ollama eval confirming the default path still works.
- A note that the OpenAI path is covered by pure builders/parsers and verified manually against a user endpoint (no key in the repo/tests).

## 5. Success criteria

- SC1. Synth can generate and report status against either an Ollama or an OpenAI-compatible endpoint, selected by kind.
- SC2. The OpenAI shape (chat/completions + Bearer) and the Ollama shape are both correct, with pure tested builders/parsers.
- SC3. Default Ollama behaviour is unchanged.
- SC4. The API key is BYOK, in-memory only, never persisted or logged; no new capability is added.
- SC5. The slice stays story-sized: OpenAI streaming (SSE) and model-role assignment remain deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Shape correctness | both builders/parsers correct for representative payloads | Rust tests | @BrendanShields |
| Default unchanged | Ollama generation/status identical to today | Existing tests + eval | @BrendanShields |
| Kind validation | non-ollama/openai rejected | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Secret handling | key never persisted/logged; 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-028 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Provider `kind` (`ollama`/`openai`) + optional in-memory API key in config state.
- Pure kind-specific request builders and response parsers, with the shared generation path branching on kind.
- OpenAI status via `/v1/models`; Ollama status via `/api/tags`.
- `ask_stream` single-chunk fallback for the OpenAI kind; real streaming kept for Ollama.
- A renderer control for kind + API key.

### Out of scope

- True OpenAI SSE token streaming (a later spec).
- Persisting the API key or config; secret storage / OS keychain (a later phase).
- Model-role assignment (planner/builder/etc.).
- Provider-specific parameters (temperature, max tokens, tools).
- Additional provider families beyond Ollama and OpenAI-compatible.

## 8. Technical design

### Rust/Tauri core

Extend `ProviderConfig`/state with `kind` and an optional `api_key` (the key is not part of the camelCase status payload exposed to React beyond an "is set" indicator if needed; it is never logged). Add pure helpers:

```text
build_request_body(config, prompt) -> serde_json::Value   // branches on kind
request_endpoint(config) -> String                         // /api/generate vs /v1/chat/completions
parse_answer(kind, body) -> Result<String, String>         // response vs choices[0].message.content
parse_openai_models(body) -> Vec<String>                   // data[].id
is_valid_provider_kind(kind) -> bool
```

`generate(config, prompt)` builds the endpoint/body/headers per kind (adding `Authorization: Bearer` for openai when a key is set) and parses per kind. `get_provider_status` branches: Ollama `/api/tags` + `parse_ollama_models`; OpenAI `/v1/models` + `parse_openai_models`. `ask_stream` keeps the Ollama NDJSON loop and, for openai, calls `generate` once and emits a single chunk + done. `set_provider_config` gains `kind` and `api_key`, validating the kind.

### React renderer

Extend the provider-settings control (FS-028) with a kind selector (`ollama`/`openai`) and, when `openai`, an API-key input (masked). Saving calls `set_provider_config` with kind + key and refreshes status. The key is sent to the core but never displayed back or persisted.

### Styling

Reuse the FS-028 provider control styling; add a small kind selector and a masked key input.

## 9. Impact notes

- Data model impact: extends `ProviderConfig` with kind + optional in-memory key; no new persisted entities; the key is process-lifetime only.
- Security/privacy impact: introduces an API key (BYOK) held in memory and sent as a Bearer header to the configured endpoint; never persisted or logged. Generation remains a Rust-side HTTP call; no capability added. A user-set base URL could be remote — that is the BYOK intent; remote-provider policy is a later phase.
- Observability impact: provider kind changes can be noted in the FS-011 session log (never the key).
- Performance impact: negligible; one request per generation/status.
- Migration/backward compatibility impact: additive; default kind `ollama` preserves current behaviour.

## 10. Risks and dependencies

- Risk: leaking the API key. Mitigation: the key is never persisted, never logged, never returned to the renderer; only sent as a Bearer header to the configured endpoint.
- Risk: incorrect OpenAI shape. Mitigation: pure builders/parsers tested against representative payloads; manual verification against a real endpoint.
- Risk: misleading "streaming" for openai. Mitigation: openai emits a single chunk + done (honest non-streaming), with real SSE deferred and noted.
- Risk: breaking the Ollama default. Mitigation: default kind `ollama`; existing tests/eval cover it.
- Dependency: FS-028 merged (configurable provider state) and the generation/status/streaming commands.

## 11. Open questions

None. This slice adds the OpenAI-compatible non-streaming shape and status; SSE streaming, secret storage, and roles are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-028 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-029-openai-compatible`
- Expected implementation PR title: `feat(FS-029): OpenAI-compatible provider shape`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-029/amendments/`.
