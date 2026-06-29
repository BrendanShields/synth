---
spec_id: FS-034
title: Model-assisted diff review
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
  - docs/adrs/ADR-0011-git-via-cli-readonly-first.md
---

# FS-034: Model-assisted diff review

## 1. Problem statement

FS-027 shows the working-tree diff; the PRD's Phase 4 makes review a first-class surface, including adversarial/model review of changes (PRD §10.5, §21). This story adds the first review capability: the model reviews the current diff and returns findings (risks, bugs, omissions). It builds on the read-only git diff (FS-027, ADR-0011) and the provider generation path (FS-008/FS-029), and uses the `adversary` role model (FS-031) so review can run on a different model than drafting.

This is read-only and advisory: it reads the diff, asks the model, and shows the review. It applies nothing, blocks nothing, and writes nothing.

## 2. Requirements

- R1. The Rust core must expose an async Tauri command `review_diff() -> Result<DiffReview, String>` that reads the open workspace's working-tree diff against `HEAD` and returns the model's review.
- R2. Reading the diff must use the read-only, jailed git invocation discipline (`git diff HEAD`, fixed args, no shell), with the diff text bounded by a fixed maximum (truncated, never unbounded) before being sent to the model.
- R3. `DiffReview` must serialize in camelCase with at least: `empty` (true when there is no diff) and `review` (the model's review text; empty when `empty`).
- R4. Building the review prompt from the diff text must be a pure, unit-testable function instructing the model to review the diff for correctness, risks, and omissions and to be concise.
- R5. `review_diff` must use the resolved `adversary` role model (FS-031: override or default) for generation, reusing the provider generation path; transport/parse failures return a readable `Err` without panicking.
- R6. If no workspace is open, `review_diff` returns a readable `Err`. If the workspace is not a git repository or has no changes, it returns `empty: true` with an empty review and no error.
- R7. `review_diff` must be read-only: it must not stage, apply, commit, write, or perform any mutating git or network operation other than the localhost model request.
- R8. The renderer must provide a control to review the current diff, call `review_diff`, and render the review in a calm surface with a pending state while generating and an empty/error state otherwise. Review state must be transient renderer state only.
- R9. This story must not add new Tauri capability permissions, must not change the FS-001 runtime status contract, and must not change existing commands.

## 3. Acceptance criteria

- AC1. `build_review_prompt(diff)` includes the diff text and instructs the model to review for correctness, risks, and omissions concisely.
- AC2. With a git workspace that has uncommitted changes, `review_diff` returns `empty: false` and a non-empty `review`. (Verified live in the eval.)
- AC3. With a clean working tree or a non-git folder, `review_diff` returns `empty: true` and an empty `review`, no error.
- AC4. With no workspace open, `review_diff` returns `Err`.
- AC5. A diff larger than the cap is truncated before being sent to the model, never read/sent unbounded.
- AC6. `review_diff` uses the resolved `adversary` model (override or default).
- AC7. The renderer reviews the diff and renders the review with pending/empty/error states.
- AC8. Rust unit coverage verifies the pure review-prompt builder and diff truncation; the live review path is covered by the eval. Existing tests remain intact.
- AC9. No code in this story stages/applies/commits/writes, performs mutating git or non-model network operations, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `cargo test` must not perform network I/O; only the pure prompt builder and truncation are unit-tested. Live review is verified by the eval.

Eval (required, attached to the PR): with local Ollama running `gemma4:e4b` and a representative diff, run the review and confirm a non-empty, on-topic review (mentions the changed code).

Manual checks:

- Make an uncommitted change, run review, and confirm a readable review appears.
- Revert the change and confirm a calm empty state.
- Stop Ollama and confirm a calm error.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- The eval review output for a sample diff.
- Short note confirming read-only behaviour, the diff cap, adversary-role usage, and unchanged capabilities.

## 5. Success criteria

- SC1. The model reviews the current diff and returns concise findings.
- SC2. Review is read-only, jailed, bounded, and uses the adversary role.
- SC3. Non-repository, clean, and failure cases degrade calmly.
- SC4. No mutation, non-model network, or new capability is introduced.
- SC5. The slice stays story-sized and does not apply changes, post comments, or block.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Review quality | non-empty, on-topic review for a real diff | Eval output | @BrendanShields |
| Prompt correctness | builder includes diff + review instruction | Rust tests | @BrendanShields |
| Bounded input | over-cap diff truncated | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 mutations, 0 non-model network, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-033 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An async `review_diff` command: read-only jailed `git diff HEAD`, truncate, review via the adversary-role model.
- A `DiffReview` camelCase shape and a pure review-prompt builder + truncation.
- A renderer review control and calm review surface.
- An eval reviewing a diff against `gemma4:e4b`.

### Out of scope

- Applying changes, staging, or committing from the review.
- Posting the review as a PR/commit comment or persisting it.
- Structured/located findings (line-anchored), multi-pass or multi-model panels.
- Reviewing a specific commit/range beyond the working tree vs `HEAD`.
- Blocking or gating on the review.

## 8. Technical design

### Rust/Tauri core

Add a read-only `git::diff_text(root) -> Result<(bool, String), String>` returning `(is_repo, diff)` for `git diff HEAD` (jailed, fixed args), or extend the existing diff path. Add to the `provider` (or a `review`) module:

```text
DiffReview { empty, review }                 // serde camelCase
build_review_prompt(diff: &str) -> String     // pure
review_diff(workspace, provider, roles) -> Result<DiffReview, String>   // async #[tauri::command]
```

`review_diff` reads the workspace root (Err if none), gets the diff text, returns `empty: true` if not a repo / no changes, truncates the diff to a fixed cap, resolves the `adversary` role model, and generates the review via the shared provider generation path. Register the command.

### React renderer

Add a "Review diff" control (shown when a workspace has a diff) that calls `review_diff`, shows a pending state, and renders the review in a calm surface, with empty and error states.

### Styling

Reuse the reader/answer surface styles.

## 9. Impact notes

- Data model impact: introduces a `DiffReview` IPC shape; no persisted entities.
- Security/privacy impact: read-only git per ADR-0011 plus one localhost model request; the diff (which may contain code) is sent to the configured provider — for the default local Ollama this stays on-device. No mutation, no non-model network, no capability added.
- Observability impact: a review can be noted in the FS-011/FS-032 event log.
- Performance impact: one bounded git diff + one generation per review.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: sending a large diff to the model. Mitigation: a fixed truncation cap before the request.
- Risk: sending code to a remote provider. Mitigation: the default provider is local Ollama (on-device); remote providers are the user's explicit BYOK choice.
- Risk: implying the review gates merges. Mitigation: review is advisory and read-only; gating is out of scope.
- Dependency: FS-027 (diff), FS-029 (provider generation), FS-031 (adversary role).

## 11. Open questions

None. This slice produces an advisory, read-only model review of the working-tree diff; located findings and panels are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-033 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-034-model-diff-review`
- Expected implementation PR title: `feat(FS-034): Model-assisted diff review`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-034/amendments/`.
