---
spec_id: FS-009
title: Ask the active spec artifact
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

# FS-009: Ask the active spec artifact

## 1. Problem statement

FS-008 makes `?` ask the model a free-floating question. But the PRD's command grammar defines `?` as "ask the current artifact" (PRD §19.3), and FS-006 already gives the shell a single active artifact. The next slice connects them: when a spec is the active artifact, `?` should ask the model *about that spec*, grounded in its known content.

The grounding prompt is built in the trusted Rust core from the static spec detail it already owns (FS-005), so the renderer never assembles prompts and the model answer stays anchored to committed spec content. When no artifact is active, `?` falls back to the FS-008 free-form ask. This keeps the model genuinely useful for the documents-first workflow without introducing document filesystem reading, multi-turn chat, or tools.

## 2. Requirements

- R1. The Rust core must expose an async Tauri command `ask_spec(specId: String, question: String) -> Result<ModelAnswer, String>` that grounds the question in the named static spec detail and returns the model's answer.
- R2. `ask_spec` must look up the spec detail via the existing FS-005 catalog. An unknown `specId` must return a readable `Err` and must not call the model.
- R3. An empty or whitespace-only `question` must return a readable `Err` and must not call the model.
- R4. Building the grounded prompt must be a pure, unit-testable function that incorporates the spec's `specId`, `title`, `summary`, `scope`, and `limitations` as context plus the user's question, and instructs the model to answer using that context.
- R5. The returned `ModelAnswer` `prompt` field must record the user's question (not the full grounded prompt), so the renderer can show what the user asked; the grounded context need not be surfaced verbatim.
- R6. On any transport, status, or parse failure, `ask_spec` must return `Err(String)` without panicking, reusing the FS-008 generation path.
- R7. The renderer must, on a handled `answer` route, call `ask_spec(activeSpecId, question)` when a spec artifact is active, and fall back to `ask_model(question)` when none is active. Routing of `?` (FS-008) is unchanged.
- R8. The answer surface must indicate, quietly, when an answer is grounded in an active spec (for example, the active spec id), and must show no grounding indicator for an ungrounded answer.
- R9. This story must not add streaming, multi-turn history, tools, document filesystem reading, credentials, remote endpoints, persistence, policy gating, or new Tauri capability permissions.
- R10. This story must not change the FS-001 runtime status contract and must not remove or break `ask_model` (FS-008) or any earlier command; `ask_spec` is additive.

## 3. Acceptance criteria

- AC1. `build_spec_prompt(detail, question)` includes the spec id, title, summary, scope, and limitations and the user question, and instructs the model to answer from that context.
- AC2. `ask_spec("FS-006", "what does this spec add?")` against a reachable provider returns a non-empty answer whose `prompt` field equals the user question. (Verified live in the eval.)
- AC3. `ask_spec("FS-999", "x")` returns `Err` (unknown spec) with no model call.
- AC4. `ask_spec("FS-006", "")` returns `Err` (empty question) with no model call.
- AC5. With a spec active, submitting `? ...` calls `ask_spec` for that spec; with none active, submitting `? ...` calls `ask_model` (FS-008 behavior).
- AC6. The answer surface shows the active spec id as a quiet grounding indicator for a grounded answer and shows none for an ungrounded answer.
- AC7. Rust unit coverage verifies grounded-prompt construction, unknown-spec rejection, and empty-question rejection.
- AC8. No code in this story adds streaming, persistence, document reading, credentials, new capabilities, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `cargo test` must not perform network I/O. Only the pure prompt-building and validation logic is unit-tested; live grounded generation is verified by the eval below.

Eval (required, attached to the PR): with local Ollama running `gemma4:e4b`, call `ask_spec` (or the equivalent grounded prompt) for at least two specs with questions answerable only from spec context (for example, "what is out of scope for this spec?"). The eval passes if answers are non-empty and consistent with the spec content.

Manual checks:

- Select a spec, ask `? what does this spec add?`, and confirm the answer is about that spec and shows the grounding indicator.
- Clear the active artifact, ask `? hello`, and confirm the ungrounded FS-008 path with no grounding indicator.
- Confirm `src-tauri/capabilities/*.json` gains no new permissions.

## 5. Success criteria

- SC1. `?` answers about the active spec when one is selected, using core-built grounding.
- SC2. Prompt construction stays in the trusted core and is unit-tested.
- SC3. Ungrounded `?` still works when no artifact is active.
- SC4. The grounded answer is clearly, quietly attributed to its spec.
- SC5. The slice stays story-sized and does not become chat, tools, or document reading.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Grounded relevance | 2/2 eval answers consistent with spec content | Eval log attached to PR | @BrendanShields |
| Validation | Unknown spec and empty question rejected with no model call | Rust tests | @BrendanShields |
| Pure-logic coverage | Grounded-prompt build covered by tests | `cargo test` output | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 streaming, 0 document reading, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-008 commands/contracts intact; `ask_spec` additive | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An async `ask_spec` command grounding a question in a static spec detail.
- A pure grounded-prompt builder with unit tests.
- Renderer wiring: active spec → `ask_spec`, otherwise `ask_model`.
- A quiet grounding indicator in the answer surface.
- An eval exercising grounded generation against `gemma4:e4b`.

### Out of scope

- Streaming output.
- Multi-turn conversation or history.
- Reading spec markdown bodies or any document filesystem access (grounding uses the static detail only).
- Tools, structured output, or citations.
- Asking about non-spec artifacts.
- Persistence, audit/event logging, or policy gating.

## 8. Technical design

### Rust/Tauri core

Extend the `provider` module with a pure builder and a command:

```text
build_spec_prompt(detail: &StaticSpecDetail, question: &str) -> String   // pure
ask_spec(spec_id: String, question: String) -> Result<ModelAnswer, String>  // async #[tauri::command]
```

`ask_spec` trims and rejects an empty question, looks up the detail via `specs_index::lookup_static_spec_detail` (Err on unknown), builds the grounded prompt, and reuses the FS-008 generation path (`build_generate_body` + POST + `parse_generate_answer`). The returned `ModelAnswer.prompt` is the user's question; `ModelAnswer.answer` is the model output. Register the command.

Refactor the FS-008 generation core into a small internal helper (for example `generate(config, prompt) -> Result<String, String>`) shared by `ask_model` and `ask_spec` to avoid duplicating the request logic.

### React renderer

In the handled `answer` branch, choose the call based on the active artifact: if `specDetail` is set, call `ask_spec(specDetail.specId, question)` and remember the grounding spec id in transient state; otherwise call `ask_model(question)` and clear any grounding. Render the grounding spec id quietly in the answer surface when present.

### Styling

Reuse the FS-008 answer styling; add only a small, low-contrast grounding label. No new chrome.

## 9. Impact notes

- Data model impact: no new IPC shape (reuses `ModelAnswer`); adds one command and transient grounding state.
- Security/privacy impact: same localhost generation path as FS-008; grounding context is static committed spec content, no document filesystem reading, no credentials.
- Observability impact: in-session only; no event store yet.
- Performance impact: one generation request per grounded ask, same as FS-008.
- Migration/backward compatibility impact: additive; `ask_model` and all earlier contracts unchanged.

## 10. Risks and dependencies

- Risk: grounding could drift toward a full document reader. Mitigation: grounding uses only the static spec detail; document reading stays out of scope.
- Risk: the grounded prompt could leak as the visible question. Mitigation: `ModelAnswer.prompt` carries the user question only; tests assert this.
- Risk: changing the generation internals could regress FS-008. Mitigation: share a tested helper and keep `ask_model` behavior and tests intact.
- Dependency: FS-008 merged; a local Ollama running `gemma4:e4b` for the eval.

## 11. Open questions

None. This slice grounds `?` in the active static spec only, with an ungrounded fallback.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-008 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-28
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-009-ask-active-spec`
- Expected implementation PR title: `feat(FS-009): Ask the active spec artifact`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-009/amendments/`.
