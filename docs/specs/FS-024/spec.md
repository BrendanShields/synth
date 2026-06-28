---
spec_id: FS-024
title: Model-assisted spec drafting
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
  - docs/adrs/ADR-0006-story-sized-immutable-feature-specs.md
---

# FS-024: Model-assisted spec drafting

## 1. Problem statement

FS-023 classifies a request and says whether a feature spec is required, but Synth cannot yet help write one. The PRD's core workflow is requirements-first: a request becomes a feature spec with the six non-negotiable sections before code (PRD §9.2, §9.4). This story connects the classifier to the model: for a request that needs a spec, Synth drafts a feature spec with the model (gemma via Ollama, FS-008 path) so the user has a starting point to review.

The draft is a *proposal for review*, not an approved spec: it is generated text, not written to disk, and it follows the repository's feature-spec section structure. This is the first model-authored planning artifact; approval, immutability, and saving remain later specs.

## 2. Requirements

- R1. The Rust core must expose an async Tauri command `draft_spec(request: String) -> Result<SpecDraft, String>` that builds a spec-drafting prompt from the request and returns the model's draft.
- R2. `SpecDraft` must serialize in camelCase with at least: `request` (the originating request) and `draft` (the generated spec text).
- R3. Building the spec-drafting prompt must be a pure, unit-testable function that incorporates the request and instructs the model to produce a story-sized feature spec containing the six required sections (problem statement, requirements, acceptance criteria, tests/verification plan, success criteria, metrics).
- R4. An empty/whitespace request must return a readable `Err` and must not call the model.
- R5. `draft_spec` must reuse the FS-008 generation path (the configured Ollama provider) and return a readable `Err` on any transport/parse failure without panicking.
- R6. The draft must be returned as text only; this story must not write the draft to the workspace or anywhere on disk, must not create a branch/commit/PR, and must not mark anything approved.
- R7. The renderer must let the user draft a spec from a request (for example, a control shown when a classification indicates a spec is required, or a dedicated input), call `draft_spec`, and render the returned draft in a calm reader surface with a pending state while generating and a readable error state on failure. Draft state must be transient renderer state only.
- R8. This story must not add new Tauri capability permissions, must not change the FS-001 runtime status contract, and must not alter any existing command. Network access is the existing localhost Ollama generation path.

## 3. Acceptance criteria

- AC1. `build_spec_prompt_for_request("Add a loading state to the command dock")` includes the request and instructs the model to produce the six required sections.
- AC2. `draft_spec(request)` against a reachable provider returns a `SpecDraft` whose `request` equals the input and whose `draft` is non-empty. (Verified live in the eval.)
- AC3. `draft_spec("")` returns `Err` and performs no model call.
- AC4. With the provider unreachable, `draft_spec` returns a readable `Err` with no panic.
- AC5. The renderer drafts a spec from a request and renders it in a calm reader surface with pending and error states; the draft is not saved anywhere.
- AC6. Rust unit coverage verifies prompt construction (request + six-section instruction) and empty-request rejection; no test requires a live model.
- AC7. No code in this story writes the draft to disk, creates git artifacts, marks approval, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `cargo test` must not perform network I/O. Only the pure prompt builder and empty-request validation are unit-tested; live drafting is verified by the eval.

Eval (required, attached to the PR): with local Ollama running `gemma4:e4b`, draft a spec for at least one realistic request and confirm the draft is non-empty and contains the six required section headings.

Manual checks:

- Classify a component request, draft a spec, and confirm a readable draft renders with the required sections.
- Draft with an empty request and confirm a calm validation error.
- Stop Ollama and confirm a calm error.
- Confirm `src-tauri/capabilities/*.json` is unchanged and nothing is written to disk.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- The eval draft (or its section headings) for a sample request.
- Short note confirming the draft is not saved, no git artifacts are created, and capabilities are unchanged.

## 5. Success criteria

- SC1. Synth drafts a feature spec from a request using the model.
- SC2. Drafts follow the six-section structure and are returned for review, not saved.
- SC3. Empty requests and provider failures degrade calmly.
- SC4. No disk writes, git artifacts, approvals, or new capability are introduced.
- SC5. The slice stays story-sized and does not save, approve, or implement the spec.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Draft quality | non-empty draft with the six section headings | Eval output | @BrendanShields |
| Prompt correctness | builder includes request + six-section instruction | Rust tests | @BrendanShields |
| Graceful failure | empty request and unreachable provider return errors | Rust tests / manual | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 disk writes, 0 git artifacts, 0 approvals, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-023 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An async `draft_spec` command reusing the FS-008 generation path.
- A pure spec-drafting prompt builder with unit tests.
- A `SpecDraft` camelCase shape.
- A renderer draft control and a calm draft reader surface.
- An eval drafting a spec against `gemma4:e4b`.

### Out of scope

- Writing the draft to `docs/specs/` or anywhere on disk (a later spec).
- Approving, marking immutable, or versioning the spec.
- Interactive multi-step refinement, requirement-quality critique, or clarifying-question loops (later specs).
- Creating branches/commits/PRs from the draft.
- Streaming the draft (may reuse FS-010 later); a single-shot draft is sufficient here.

## 8. Technical design

### Rust/Tauri core

Add a spec-drafting helper and command (in the `provider` or a new `specs` module):

```text
SpecDraft { request, draft }   // serde camelCase
build_spec_prompt_for_request(request: &str) -> String   // pure
draft_spec(request: String) -> Result<SpecDraft, String>   // async #[tauri::command]
```

`build_spec_prompt_for_request` instructs the model to write a concise, story-sized feature spec for the request, with the six required sections as markdown headings, solution-free where appropriate. `draft_spec` trims and rejects empty input, then reuses the FS-008 generation helper against the configured provider, returning `SpecDraft { request, draft }`. Failures map to `Err`.

### React renderer

Add a draft control (reuse the request text, e.g. surfaced when a classification requires a spec, or a dedicated input) that calls `draft_spec`, shows a pending state while generating, and renders the returned draft in a calm preformatted reader surface, with an error state. The draft is transient and clearly a proposal (not saved).

### Styling

Reuse the FS-014 reader surface styling for the draft; add nothing beyond a calm pending indicator.

## 9. Impact notes

- Data model impact: introduces a `SpecDraft` IPC shape; no persisted entities; the draft is transient.
- Security/privacy impact: reuses the existing localhost Ollama generation path; no new network surface, no disk writes, no capability.
- Observability impact: drafting can be noted in the FS-011 session log.
- Performance impact: one generation request per draft (a longer generation than a short answer); bounded by the provider timeout.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: drafts treated as approved specs. Mitigation: the draft is transient text, not saved or marked approved; saving/approval are later specs.
- Risk: incomplete drafts (missing sections). Mitigation: the prompt explicitly requires the six sections; the eval checks for the headings; the user reviews and refines.
- Risk: latency on a long draft. Mitigation: a calm pending state and the provider timeout; streaming can be added later.
- Dependency: FS-023 merged (classification) and FS-008 generation path; local Ollama running `gemma4:e4b` for the eval.

## 11. Open questions

None. This slice drafts a feature spec for review using the model, without saving, approving, or implementing it.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-023 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-024-draft-spec`
- Expected implementation PR title: `feat(FS-024): Model-assisted spec drafting`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-024/amendments/`.
