---
spec_id: FS-051
title: Adversarial requirements review
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
  - docs/adrs/ADR-0006-story-sized-immutable-feature-specs.md
  - docs/adrs/ADR-0008-byok-provider-strategy.md
---

# FS-051: Adversarial requirements review

## 1. Problem statement

The PRD makes adversarial review a first-class part of the product model (PRD §22): "Every meaningful workflow should include adversarial review," and it explicitly lists *requirements review* as one of the points at which that review should happen — before code exists. Today Synth only has one adversarial pass: post-implementation diff review (FS-034), which runs after the work is built. There is no pass that critiques a drafted spec's requirements *before* implementation, when fixing a missing or untestable requirement is cheapest.

The substrate for this already exists. FS-031 defines a `requirements_critic` model role, but that role is currently dormant — it is never resolved into a generation by any command. FS-024 drafts a spec from a request, and the FS-008/FS-029 provider path generates answers. This story wires the dormant `requirements_critic` role into a single, read-only adversarial pass over a drafted spec, surfacing concrete requirement risks (ambiguity, untestable criteria, missing edge cases, hidden scope) so they can be fixed before the spec is saved.

This slice mirrors FS-034 exactly in shape (a pure prompt builder plus one provider-backed command, surfaced calmly in the renderer), only the input is spec text and the role is `requirements_critic` rather than `adversary` over a diff.

## 2. Requirements

- R1. The Rust core must expose a pure `build_requirements_review_prompt(spec)` function that frames the model as a requirements critic and asks for concrete, concise findings (ambiguity, untestable acceptance criteria, missing requirements/edge cases, scope creep), instructing it to say so plainly when the requirements look sound.
- R2. The prompt builder must embed the provided spec text and must bound the embedded text with a defined character cap, reusing the existing truncation helper used by FS-034.
- R3. The Rust core must expose `review_requirements(provider, roles, spec) -> Result<RequirementsReview, String>` as a `#[tauri::command]` that resolves the model via the `requirements_critic` role (FS-031), builds the prompt, and generates a single answer via the shared provider path.
- R4. `RequirementsReview` must serialize camelCase with at least an `empty` flag and a `review` string; when the spec input is empty/whitespace the command must return `empty: true` with no model call.
- R5. The review must be strictly read-only: it must not write any file, must not run any command, must not require an approval, and must not add any new Tauri capability.
- R6. The `requirements_critic` role must resolve to a configured override when set and otherwise fall back to the default model, exactly like the other roles (FS-031), with no change to the role set or the `RoleAssignment` contract.
- R7. The renderer must add a "Review requirements" action wherever a drafted spec is shown (the classification/draft surface), call `review_requirements` with the current draft text, and display the findings with a calm pending state and a calm error state.
- R8. This story must not change any existing command's contract (FS-024 draft, FS-034 diff review, FS-025 spec save) and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `build_requirements_review_prompt` returns a prompt that contains the spec text and an instruction to list concrete requirement findings (and to say so if the requirements look sound).
- AC2. The prompt builder truncates spec text longer than the defined cap and never panics on very long input.
- AC3. `review_requirements` returns `empty: true` and performs no generation when given empty or whitespace-only spec text.
- AC4. `review_requirements` resolves the `requirements_critic` role: with an override configured it uses the override model; with none it uses the provider's default model.
- AC5. A successful review returns `empty: false` and the model's findings text via the shared provider path.
- AC6. The renderer shows a "Review requirements" control on the drafted-spec surface, renders the returned findings, shows a pending state while in flight, and shows a readable error state on failure.
- AC7. Rust unit coverage verifies the prompt builder (spec inclusion + instruction), the truncation cap, the empty-input short-circuit, the `requirements_critic` role resolution (override vs default), and camelCase serialization of `RequirementsReview`.
- AC8. No new Tauri capabilities are added; FS-024, FS-034, FS-025, FS-031, and the FS-001 runtime contract remain stable.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Rust unit tests must cover the pure prompt builder, the truncation cap, the empty-input short-circuit, role resolution for `requirements_critic` (override and default), and `RequirementsReview` camelCase serialization. Tests must not depend on a reachable model; the network generation path is exercised only manually (consistent with FS-034's test approach).

Manual checks:

- Classify a project request so a spec draft is produced (FS-023/FS-024), then click "Review requirements" and confirm findings render.
- Confirm an empty/blank draft yields the calm empty state with no model call.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- A short eval note: a deliberately weak requirement (e.g. "make it fast") is flagged by the critic as untestable.
- A note confirming capabilities are unchanged and no approval/write path was added.

## 5. Success criteria

- SC1. A drafted spec can be adversarially critiqued for requirement quality before implementation.
- SC2. The previously dormant `requirements_critic` role is now active and selectable per FS-031.
- SC3. The pass is read-only: no writes, no command execution, no approval, no new capability.
- SC4. The feature is story-sized and mirrors FS-034: one pure prompt builder, one provider-backed command, one calm renderer surface.
- SC5. Existing draft, diff-review, and save contracts are unchanged.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Role activation | `requirements_critic` resolves to override/default correctly | Rust tests | @BrendanShields |
| Prompt grounding | Spec text + critic instruction present; bounded by cap | Rust tests | @BrendanShields |
| Empty-input safety | empty/whitespace returns `empty: true`, no generation | Rust tests | @BrendanShields |
| Read-only containment | 0 new capabilities; no write/exec/approval path | Capability/source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Contract stability | FS-024/FS-034/FS-025/FS-031/FS-001 contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A pure `build_requirements_review_prompt(spec)` helper.
- A `review_requirements` command resolving the `requirements_critic` role and generating one review.
- A `RequirementsReview { empty, review }` IPC shape.
- A "Review requirements" renderer action on the drafted-spec surface with pending/empty/error states.

### Out of scope

- Persisting review findings or attaching them to the saved spec/PR body (deferred; the PRD's "findings attach to the spec/PR" is a later slice).
- Design review, pre-implementation plan review, test/check review, and PR-readiness review (separate adversarial points).
- Multi-model orchestration or automatically choosing a different model from the builder.
- Auto-applying or auto-editing the spec based on findings.
- Any change to the diff-review (FS-034) or draft (FS-024) flows beyond adding the new, separate action.

## 8. Technical design

### Rust/Tauri core

In `provider`, add:

```text
RequirementsReview { empty, review }                         // serde camelCase
build_requirements_review_prompt(spec) -> String             // pure, bounded by truncate_diff cap
review_requirements(provider, roles, spec) -> Result<RequirementsReview, String>  // #[tauri::command]
```

`build_requirements_review_prompt` reuses `truncate_diff` (the existing char-cap helper) against a `MAX_REQUIREMENTS_REVIEW_CHARS` cap and embeds the (truncated) spec inside a critic instruction analogous to `build_review_prompt`. `review_requirements` trims the spec; if empty it returns `RequirementsReview { empty: true, review: String::new() }` without a model call. Otherwise it clones the current provider config, resolves the model via `crate::roles::resolve_model_for_role("requirements_critic", &overrides, &config.model)`, generates via the shared `generate` path, and returns `RequirementsReview { empty: false, review }`. Register the command in `lib.rs`.

### React renderer

On the classification/draft surface (where `specDraft` is shown), add a "Review requirements" button that calls `review_requirements` with the current draft text, storing the result in transient state. Render the findings in a calm surface, with a pending label while in flight and a readable error state on failure, reusing the existing review/answer styles used by the diff-review surface.

### Styling

Reuse the existing review/answer/notice styles; no new visual system.

## 9. Impact notes

- Data model impact: introduces a `RequirementsReview` IPC shape; no persisted entities.
- Security/privacy impact: one read-only generation (default Ollama on-device; remote is the user's BYOK choice per ADR-0008); no writes, no command execution, no approval, no new capability.
- Observability impact: a requirements-review pass can be noted in the event log, like diff review.
- Performance impact: one bounded generation per invocation.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: sending spec text to a remote provider. Mitigation: default provider is local Ollama (on-device); remote is the user's explicit BYOK choice (ADR-0008).
- Risk: oversized spec input. Mitigation: bounded by the truncation cap before the prompt is built.
- Risk: implying the critique edits the spec. Mitigation: the pass is read-only and only displays findings; auto-fix is out of scope.
- Risk: implying findings are persisted to the PR. Mitigation: persistence/attachment is explicitly deferred to a later slice.
- Dependency: FS-031 roles (`requirements_critic`), FS-024 draft, FS-008/FS-029 provider generation, FS-034 prompt/truncation pattern.

## 11. Open questions

None. This slice activates an adversarial requirements-review pass over a drafted spec; persisting findings, other review points, and auto-fix are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-050 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-051-adversarial-requirements-review`
- Expected implementation PR title: `feat(FS-051): Adversarial requirements review`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-051/amendments/`.
