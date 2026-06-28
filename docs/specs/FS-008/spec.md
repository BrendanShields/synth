---
spec_id: FS-008
title: Ask-the-model single-shot completion
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

# FS-008: Ask-the-model single-shot completion

## 1. Problem statement

FS-007 proves the trusted core can see the configured Ollama provider. The next slice closes the loop: actually send a prompt to the model and render its answer. The PRD command grammar reserves `?` for "ask" (PRD §19.3); today `?` routes as `unsupported`. This story makes `?` ask the model a single-shot question and shows the answer in a calm surface.

Generation runs entirely in the Rust core (ADR-0001): the renderer submits a `?` command, the core sends a single non-streaming request to Ollama, and the answer is returned over typed IPC. Streaming, multi-turn sessions, the agent loop, tool calls, and grounding the question in the active artifact are explicitly deferred to later specs. This slice is the minimum that demonstrates a real model round-trip.

## 2. Requirements

- R1. The Rust core must expose an async Tauri command `ask_model(prompt: String) -> Result<ModelAnswer, String>` that sends a single non-streaming completion request to the configured Ollama provider's `/api/generate` endpoint and returns the model's answer.
- R2. `ModelAnswer` must serialize in camelCase with at least: `model`, `prompt`, and `answer`.
- R3. Building the Ollama request body must be a pure, unit-testable function producing `{ model, prompt, stream: false }` for the configured model, with `stream` false.
- R4. Parsing the Ollama `/api/generate` non-streaming response body into the answer text must be a pure, unit-testable function. A malformed or empty body must produce a typed error, not a panic.
- R5. `ask_model` must reject an empty or whitespace-only prompt with a readable error and must not call the provider in that case.
- R6. On any transport, status, or parse failure, `ask_model` must return `Err(String)` with a readable message; it must never panic.
- R7. The command router must route a non-empty `?` command (`CommandKind::Ask`) to `disposition: handled` with a new `target: answer`, carrying the question text (resource or the existing parsed argument) so the renderer does not re-parse the raw input. An empty `?` must remain a renderer no-op (consistent with FS-003 empty handling) or a non-handled route, and must not call the model.
- R8. On a handled answer route, the renderer must call `ask_model`, render the returned answer in an `id="answer"` surface, scroll it into view, and show a calm loading state while the request is in flight.
- R9. If `ask_model` fails, the renderer must show a calm, readable inline error in the answer surface without crashing and without discarding the command log.
- R10. The answer surface and pending/answer state must be transient renderer state only: no persistence to disk, app-local storage, browser storage, repo files, or URL/history.
- R11. This story must not add streaming, multi-turn history, system prompts, tool calls, credentials, remote endpoints, model selection, persistence, policy gating, or new Tauri capability permissions. Network access remains a localhost request to the static Ollama base URL, now including a POST to `/api/generate`.
- R12. This story must not change the FS-001 runtime status contract and must not remove or break `parse_command`, `route_command` (existing targets), `list_specs_index`, `get_static_spec_detail`, or `get_provider_status`. Adding the `answer` target and Ask handling must be additive.

## 3. Acceptance criteria

- AC1. `build_generate_body` produces `{ model: "gemma4:e4b", prompt: <text>, stream: false }`.
- AC2. `parse_generate_answer` extracts the answer text from a representative `/api/generate` non-streaming JSON body and returns a typed error for malformed/empty bodies.
- AC3. `ask_model("")` (or whitespace) returns `Err` and performs no provider call.
- AC4. Given a reachable Ollama with the model, submitting `? what is 2 + 2?` results in a handled `answer` route, a model request, and a rendered non-empty answer surface. (Verified live in the eval.)
- AC5. Given the provider is unreachable, submitting a `?` question shows a calm inline error in the answer surface with no crash and no unhandled promise rejection.
- AC6. Submitting an empty `?` performs no model call and adds no answer.
- AC7. Existing routes — `/summary`, `/specs`, `/specs/<id>`, `/runtime`, blocked `!`, and other unsupported kinds — behave exactly as before; only `?` changes from unsupported to handled.
- AC8. Rust unit coverage verifies request-body building, answer parsing (success + malformed), empty-prompt rejection, and Ask routing to the `answer` target with camelCase serialization.
- AC9. No code in this story adds streaming, persistence, credentials, remote endpoints, new Tauri capabilities, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `cargo test` must not perform network I/O. Only pure body-building and answer-parsing logic and routing are unit-tested; live generation is verified by the eval below.

Eval (required, attached to the PR): with local Ollama running `gemma4:e4b`, send at least three single-shot prompts via the provider path (for example a factual question, a short instruction, and a one-line summary request) and record the prompts, answers, and round-trip latency. The eval passes if each prompt returns a non-empty, on-topic answer with no error.

Manual checks:

- Submit `? what is 2 + 2?` in the running app and confirm a calm answer surface renders the response.
- Submit an empty `?` and confirm nothing happens.
- Stop Ollama and confirm a `?` question renders a calm error, not a crash.
- Confirm `src-tauri/capabilities/*.json` gains no new permissions.

## 5. Success criteria

- SC1. A user can ask the configured model a single-shot question from the command dock and read a coherent answer.
- SC2. Generation is owned by the Rust core and exposed through typed IPC.
- SC3. Failures (empty prompt, unreachable provider, malformed response) degrade calmly with no panic.
- SC4. The answer experience stays quiet and minimal, consistent with the shell aesthetic.
- SC5. The slice stays story-sized and does not become streaming, chat history, tool use, or an agent loop.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Round-trip success | 3/3 eval prompts return non-empty, on-topic answers | Eval log attached to PR | @BrendanShields |
| Graceful failure | Empty prompt and unreachable provider return typed errors, no panic | Rust review / manual | @BrendanShields |
| Pure-logic coverage | Body build + answer parse + routing covered by tests | `cargo test` output | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 streaming, 0 persistence, 0 credentials, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-007 commands/contracts intact; only `?` becomes handled | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An async `ask_model` command doing a single non-streaming `/api/generate` POST.
- Pure request-body and response-parse helpers with unit tests.
- Routing `?` to a new `answer` target carrying the question.
- A calm renderer answer surface with loading and error states.
- An eval exercising real generation against `gemma4:e4b`.

### Out of scope

- Streaming token output (later spec).
- Multi-turn conversation, history, or sessions.
- System prompts, tool calls, structured output, or thinking-trace rendering.
- Grounding the question in the active artifact or any document content.
- Model/role selection, remote providers, credentials, or a settings UI.
- Persistence of prompts/answers, audit/event logging, or policy gating.

## 8. Technical design

### Rust/Tauri core

Extend the `provider` module:

```text
build_generate_body(config, prompt) -> serde_json::Value   // { model, prompt, stream: false }, pure
parse_generate_answer(body: &str) -> Result<String, String>  // pure
ask_model(prompt: String) -> Result<ModelAnswer, String>     // async #[tauri::command]
```

`ask_model` trims and rejects empty prompts, then POSTs the built body to `{base_url}/api/generate` with a generous timeout (generation is slow). Transport/status/parse failures map to `Err(String)`. Register the command in `tauri::generate_handler!`.

`ModelAnswer { model, prompt, answer }` serializes camelCase.

### Command router

Add `RouteTarget::Answer` (`answer`). In `route_navigation`/route handling, a non-empty `Ask` command returns `handled` + `answer`, carrying the question (reuse the route `resource` field added in FS-005 or the parsed argument). Empty `?` is not handled. All other kinds/targets are unchanged.

### React renderer

On a handled `answer` route, read the question, call `ask_model`, and render the result in an `id="answer"` section with three calm states: pending ("thinking…"-style quiet line), answer (the text), and error (a quiet inline message). Scroll to the section like other handled routes. Keep all state transient.

### Styling

Reuse prose/muted styles; add minimal styling for the answer surface. No chat bubbles, avatars, or noisy chrome — quiet and spacious.

## 9. Impact notes

- Data model impact: adds `ModelAnswer` IPC shape and an `answer` route target; no persisted entities.
- Security/privacy impact: adds a localhost POST to `/api/generate` with a prompt body; still no credentials, remote hosts, or persistence. Policy gating remains Phase 2.
- Observability impact: answers are in-session only; no event store or audit log yet.
- Performance impact: one blocking (non-streaming) generation request per `?`; latency depends on the model. A generous timeout prevents indefinite hangs.
- Migration/backward compatibility impact: additive; only `?` changes from unsupported to handled.

## 10. Risks and dependencies

- Risk: generation latency could feel like a hang. Mitigation: a calm pending state and a generous-but-bounded timeout; streaming arrives in a later spec.
- Risk: the answer surface could grow into a chat UI. Mitigation: single-shot only, transient state, no history; scope boundaries forbid conversation.
- Risk: changing `?` routing could disturb existing routes. Mitigation: the change is additive (new target), with tests asserting all prior routes are unchanged.
- Dependency: FS-007 merged; a local Ollama running `gemma4:e4b` for the eval and manual checks.

## 11. Open questions

None. This slice intentionally implements a single-shot, non-streaming ask against the static local provider.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-007 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-008-ollama-completion`
- Expected implementation PR title: `feat(FS-008): Ask-the-model single-shot completion`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-008/amendments/`.
