---
spec_id: FS-010
title: Streaming model answers
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

# FS-010: Streaming model answers

## 1. Problem statement

FS-008 and FS-009 answer with a single blocking request: the user submits a `?` question and waits with a quiet "Thinking…" until the whole answer arrives, which can take several seconds. The PRD makes provider streaming and observable, event-driven output first-class (PRD §15, §21), and the FS-001 runtime event bridge already exists for exactly this. This story streams the answer token-by-token from the Rust core to the renderer over Tauri events, so the answer appears as it is generated.

Streaming runs in the trusted core (ADR-0001): the core opens a streaming request to Ollama, parses the newline-delimited chunks, and emits typed events; the renderer accumulates them into the live answer. This also lays the groundwork for the agent loop and observability. It keeps the same boundary as FS-008/FS-009 — localhost only, no credentials, no persistence — and reuses the FS-009 grounding so a streamed answer can still be grounded in the active spec.

## 2. Requirements

- R1. The Rust core must expose an async Tauri command `ask_stream(app, requestId: u64, specId: Option<String>, question: String) -> Result<(), String>` that streams a model answer for the question, grounding it in the named spec when `specId` is provided (reusing FS-009), and emitting events as chunks arrive.
- R2. An empty/whitespace question must return `Err` and emit no events. An unknown `specId` must return `Err` and emit no events. Neither calls the model.
- R3. While streaming, the core must emit a chunk event (event name `synth-answer-chunk`) for each non-empty token chunk, with a camelCase payload containing at least `requestId` and `token`.
- R4. On normal completion, the core must emit a final event (event name `synth-answer-done`) with a camelCase payload containing at least `requestId`, `model`, and the full `answer` text. On a transport/parse failure mid-stream, it must emit an error event (event name `synth-answer-error`) with `requestId` and `message`, and the command must resolve without panicking.
- R5. Parsing a single Ollama streaming line (NDJSON) into an optional token chunk and a done flag must be a pure, unit-testable function. Malformed lines must be skipped without error.
- R6. Every emitted event must carry the `requestId` passed to `ask_stream` so the renderer can ignore events from superseded requests.
- R7. The renderer must, on a handled `answer` route, start a streaming request with a fresh monotonically increasing `requestId`, subscribe to the three answer events, accumulate `token`s into the visible answer text live, and finalize on the done event. It must ignore events whose `requestId` is not the current one.
- R8. The renderer must show streamed text as it arrives (not only after completion) and must show a calm error state if an error event arrives. Grounding indication from FS-009 must still display for grounded streams.
- R9. Streaming state must be transient renderer state only: no persistence to disk, app-local storage, browser storage, repo files, or URL/history.
- R10. This story must not add multi-turn history, tools, document filesystem reading, credentials, remote endpoints, persistence, policy gating, or new Tauri capability permissions. Network access remains a localhost streaming request to `/api/generate`.
- R11. This story must not change the FS-001 runtime status contract. It may supersede the FS-008/FS-009 blocking `ask_model`/`ask_spec` for the renderer's `?` path, but those commands must remain available and unbroken for compatibility and tests.

## 3. Acceptance criteria

- AC1. `parse_stream_line` returns the token for a chunk line (`{"response":"4","done":false}` → `Some("4")`, not done), reports done for `{"response":"","done":true}`, and skips a malformed line without error.
- AC2. Submitting `? ...` streams the answer: chunk events arrive and the answer text grows incrementally before the done event. (Verified live in the eval.)
- AC3. On the done event, the final `answer` equals the concatenation of streamed tokens and the answer surface shows the complete text.
- AC4. With a spec active, a streamed `?` is grounded in that spec and shows the grounding indicator; with none active, it is ungrounded.
- AC5. `ask_stream` with an empty question or unknown `specId` returns `Err` and emits no events.
- AC6. If a second `?` is submitted while one is streaming, events from the superseded `requestId` are ignored and only the newest answer renders.
- AC7. On a provider error mid-stream, the answer surface shows a calm error, with no crash or unhandled rejection.
- AC8. Rust unit coverage verifies stream-line parsing (chunk, done, malformed) and event payload camelCase serialization. Existing `ask_model`/`ask_spec` and all prior contracts remain intact.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `cargo test` must not perform network I/O. Only pure stream-line parsing and event payload serialization are unit-tested; live streaming is verified by the eval below.

Eval (required, attached to the PR): with local Ollama running `gemma4:e4b`, stream at least one answer and record that multiple chunks were received (chunk count > 1) and that the concatenated chunks equal the model's full response.

Manual checks:

- Submit `? explain Synth in two sentences` and confirm the text appears progressively, not all at once.
- Select a spec and confirm a streamed grounded answer shows the grounding indicator.
- Submit a second question mid-stream and confirm only the latest answer renders.
- Stop Ollama and confirm a calm error state.
- Confirm `src-tauri/capabilities/*.json` gains no new permissions.

## 5. Success criteria

- SC1. Answers stream into the shell token-by-token over the event bridge.
- SC2. Streaming is owned by the Rust core and exposed via typed events tagged by request id.
- SC3. Grounded and ungrounded asks both stream.
- SC4. Superseded and failed streams are handled calmly.
- SC5. The slice stays story-sized and does not become chat history, tools, or an agent loop.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Incremental delivery | Chunk count > 1 and concatenation equals full response | Eval log attached to PR | @BrendanShields |
| Request isolation | Superseded request events ignored | Manual review / source | @BrendanShields |
| Pure-logic coverage | Stream-line parsing + payload serialization covered | `cargo test` output | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 persistence, 0 credentials, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-009 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An async `ask_stream` command emitting chunk/done/error events tagged by request id.
- A pure NDJSON stream-line parser with unit tests.
- Reusing FS-009 grounding for streamed answers.
- Renderer event subscription, live accumulation, request-id supersession, and error handling.
- An eval confirming incremental streaming against `gemma4:e4b`.

### Out of scope

- Multi-turn conversation or history.
- Tools, structured output, or thinking-trace streaming.
- Document filesystem reading or non-spec grounding.
- Cancellation/stop control mid-stream (later spec).
- Persistence, audit/event-store integration, or policy gating.

## 8. Technical design

### Rust/Tauri core

Extend the `provider` module. Add the `stream` feature to `reqwest` and `futures-util` for stream consumption.

```text
parse_stream_line(line: &str) -> StreamChunk   // pure: { token: Option<String>, done: bool }
ask_stream(app, request_id, spec_id, question) -> Result<(), String>   // async #[tauri::command]
```

`ask_stream` validates the question and (if `spec_id` is set) looks up the spec and builds the grounded prompt via FS-009's `build_spec_prompt`; otherwise it uses the raw question. It POSTs `{ model, prompt, stream: true }`, reads the byte stream line-by-line, and for each parsed chunk emits `synth-answer-chunk { requestId, token }`. On the done line it emits `synth-answer-done { requestId, model, answer }` (answer = accumulated text). Transport/parse failures emit `synth-answer-error { requestId, message }`. Event payload structs serialize camelCase. Register the command; keep `ask_model`/`ask_spec` for compatibility.

### React renderer

Maintain a `requestId` counter and the current request id in transient state. On a handled `answer` route, increment the id, reset the streamed answer, set pending, and call `ask_stream(requestId, activeSpecId, question)`. Subscribe (once, via the existing `@tauri-apps/api/event` `listen`) to the three events; handlers ignore events whose `requestId` is not current, append tokens to the live answer, finalize on done, and show the error on error. Reuse the FS-009 grounding indicator.

### Styling

Reuse the FS-008/FS-009 answer styling. No new chrome; the streaming text simply fills in.

## 9. Impact notes

- Data model impact: adds streaming event payload shapes; no persisted entities. Reuses `ModelAnswer` semantics for the final text.
- Security/privacy impact: same localhost generation path as FS-008/FS-009, now with `stream: true`; no credentials, remote hosts, or persistence.
- Observability impact: answer chunks are emitted as runtime events in-session; this is the first token-level event stream, but no event store/persistence is added yet.
- Performance impact: many small events per answer; negligible locally, and far better perceived latency than blocking.
- Migration/backward compatibility impact: additive; `ask_model`/`ask_spec` remain. The renderer's `?` path switches to streaming.

## 10. Risks and dependencies

- Risk: event cross-talk between rapid asks. Mitigation: every event carries `requestId`; the renderer ignores superseded ids.
- Risk: partial/malformed stream lines. Mitigation: a pure tested parser that skips malformed lines and accumulates safely.
- Risk: a hung stream. Mitigation: the request uses a bounded client timeout; an error event ends the pending state.
- Dependency: FS-009 merged; local Ollama running `gemma4:e4b` for the eval.

## 11. Open questions

None. This slice streams single-shot answers (grounded or not) with request-id isolation; cancellation and history are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-009 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-010-streaming-answers`
- Expected implementation PR title: `feat(FS-010): Streaming model answers`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-010/amendments/`.
