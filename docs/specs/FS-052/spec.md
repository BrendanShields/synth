---
spec_id: FS-052
title: Review findings capture
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
  - docs/adrs/ADR-0003-hybrid-repo-and-app-local-storage.md
  - docs/adrs/ADR-0008-byok-provider-strategy.md
---

# FS-052: Review findings capture

## 1. Problem statement

The PRD makes a clear commitment about adversarial review (PRD §22): "Review findings attach to the feature spec and PR." Synth now has two adversarial-review producers — post-implementation diff review (FS-034) and pre-implementation requirements review (FS-051) — but both are ephemeral: the findings are generated, displayed once, and then discarded. There is no durable record of what was reviewed, when, against which subject, or what the critic said. That makes the findings impossible to attach to a PR body, impossible to revisit across sessions, and impossible to use as observability/improvement signal (PRD §21 calls out "adversarial review findings" as a recorded event type).

This story adds a narrow, durable review-findings store. When a diff review or a requirements review completes, Synth captures the finding (subject, source review type, and the critic's text) into an app-local append-only JSONL store, and exposes a bounded listing so recent findings can be surfaced and later attached to a PR. It mirrors the FS-032 event store and the FS-050 extension-run store exactly in shape. It does not change how reviews are generated; it only persists their output.

## 2. Requirements

- R1. The Rust core must define a `ReviewFinding` shape that serializes camelCase with at least `id`, `kind` (the review type, e.g. `diff` or `requirements`), `subject` (a short human label for what was reviewed), and `finding` (the critic's text).
- R2. Review findings must be persisted app-locally in an append-only JSONL store, separate from the FS-032 event log and all other stores, and must tolerate a missing or malformed file by returning the valid records that can be parsed.
- R3. The Rust core must expose `capture_review_finding(kind, subject, finding) -> Result<ReviewFinding, String>` as a `#[tauri::command]` that validates `kind` against a known set (`diff`, `requirements`), rejects an empty `finding`, assigns an id, and appends one record.
- R4. The Rust core must expose `list_review_findings(limit) -> Vec<ReviewFinding>` returning recent findings newest-first, bounded by the requested limit, skipping malformed lines without panicking.
- R5. The renderer must capture a finding after a successful diff review (kind `diff`) and after a successful requirements review (kind `requirements`), passing a short subject, and must not capture for the empty/`No changes`/`No requirements` outcomes.
- R6. The renderer must surface recent review findings (kind, subject, finding) with a calm empty state, and refresh the list after a capture.
- R7. Capture must be read-only with respect to the repository and the model: it performs no model call, no command execution, no repo write, and no approval; it writes only to the app-local findings store and must not add any new Tauri capability.
- R8. This story must not change the generation behavior or contracts of FS-034 (diff review) or FS-051 (requirements review), and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `capture_review_finding("diff", "working tree", "...")` appends a `ReviewFinding` with a stable id and returns it.
- AC2. `capture_review_finding` rejects an unknown `kind` and an empty/whitespace `finding` with a readable error and writes nothing.
- AC3. `list_review_findings(10)` returns at most ten valid records, newest-first, and skips malformed JSONL lines without panicking.
- AC4. A missing findings file yields an empty list (no error, no panic).
- AC5. After a successful diff review the renderer captures a `diff` finding; after a successful requirements review it captures a `requirements` finding; neither captures on the empty/no-op outcomes.
- AC6. The renderer lists recent findings and refreshes after a capture, with a calm empty state when none exist.
- AC7. Rust unit coverage verifies record construction, JSONL parse/load ordering, malformed-line tolerance, `kind` validation, empty-finding rejection, and camelCase serialization.
- AC8. No new Tauri capabilities are added; FS-034, FS-051, FS-032, and the FS-001 runtime contract remain stable.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Rust unit tests must cover the pure/file-backed helpers using temporary files (construction, ordering, malformed tolerance, kind validation, empty rejection, serialization). Tests must not depend on a reachable model.

Manual checks:

- Run a diff review (FS-034) on a dirty working tree, then confirm a `diff` finding appears in the findings list.
- Draft a spec and run a requirements review (FS-051), then confirm a `requirements` finding appears.
- Confirm the empty outcomes (no changes / no requirements) do not create findings.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- A note showing diff and requirements findings captured, and empty outcomes skipped.
- A note confirming capabilities are unchanged and no model/exec/approval path was added by capture.

## 5. Success criteria

- SC1. Adversarial review findings are durable and survive across sessions.
- SC2. Each finding records its review kind, subject, and the critic's text.
- SC3. Findings are the substrate the PRD requires for attaching review output to a PR (PRD §22) and for the "adversarial review findings" observability event (PRD §21).
- SC4. Capture is read-only (no model/exec/approval/repo write) and adds no capability.
- SC5. The slice stays story-sized: it persists and lists findings; PR-body templating and improvement analytics over findings are deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Record completeness | 100% of findings include id/kind/subject/finding | Rust tests / source review | @BrendanShields |
| Kind coverage | both `diff` and `requirements` captured from their flows | Rust tests + manual | @BrendanShields |
| Ordering and resilience | newest-first bounded listing; malformed lines skipped | Rust tests | @BrendanShields |
| Containment | 0 new capabilities; no model/exec/approval path in capture | Capability/source review | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Contract stability | FS-034/FS-051/FS-032/FS-001 contracts intact except additive commands | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An app-local `review-findings.jsonl` store.
- A `ReviewFinding` shape plus parse/load/append helpers.
- `capture_review_finding(kind, subject, finding)` and `list_review_findings(limit)` commands.
- Renderer capture after diff/requirements reviews and a recent-findings list.

### Out of scope

- Generating PR bodies from findings or auto-inserting findings into PR text (later slice).
- Improvement analytics/insights over findings (PRD §21 insight surfacing is a separate slice).
- Capturing findings from review points not yet built (design/plan/test/PR-readiness).
- Editing, deleting, filtering, or exporting findings beyond a simple recent bounded list.
- Any change to how diff or requirements reviews are generated.

## 8. Technical design

### Rust/Tauri core

Add a small `review_findings` module mirroring `events` (FS-032):

```text
ReviewFinding { id, kind, subject, finding }                 // serde camelCase
is_valid_review_kind(kind) -> bool                           // "diff" | "requirements"
serialize_record / parse_record                             // pure JSONL helpers
append_record(path, record) / load_records(path, limit)     // file I/O, malformed-tolerant, newest-first
review_findings_path(app) -> Result<PathBuf, String>        // {app_data_dir}/review-findings.jsonl
capture_review_finding(app, kind, subject, finding) -> Result<ReviewFinding, String>  // #[tauri::command]
list_review_findings(app, limit) -> Vec<ReviewFinding>      // #[tauri::command]
```

`capture_review_finding` validates `kind` (Err on unknown), trims and rejects an empty `finding`, assigns `id = load_records(path, MAX).len()`, and appends. `list_review_findings` returns the recent tail newest-first. Register both commands in `lib.rs`. This reuses the exact append/load/parse pattern already proven by FS-032 events and FS-050 extension runs.

### React renderer

After a successful diff review (FS-034) the renderer calls `capture_review_finding("diff", "working tree", review)`; after a successful requirements review (FS-051) it calls `capture_review_finding("requirements", "drafted spec", review)`. It does not capture for the empty/no-op outcomes. Add a `reviewFindings` state plus `refreshReviewFindings()` using `list_review_findings({ limit: 20 })`, call it on load and after each capture, and render recent findings (kind, subject, finding) below the existing review surfaces with a calm empty state. Reuse existing list/answer styles.

### Styling

Reuse `doc-events`/answer/notice styles; no new visual system.

## 9. Impact notes

- Data model impact: introduces an app-local `review-findings.jsonl` store and `ReviewFinding` IPC shape; additive only.
- Security/privacy impact: stores review text already shown to the user; app-local and private; no model call, no exec, no repo write, no approval, no new capability.
- Observability impact: provides the durable "adversarial review findings" record the PRD calls for (§21/§22).
- Performance impact: bounded append/read of small JSONL records.
- Migration/backward compatibility impact: missing store loads empty; malformed lines skipped; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: double-capturing on repeated review clicks. Mitigation: each successful review is one explicit user action → one record; de-duplication/merge is out of scope and acceptable for an append-only audit trail.
- Risk: capturing empty/no-op review text. Mitigation: capture is skipped for the empty outcomes and the command rejects empty findings.
- Risk: leaking sensitive review text. Mitigation: the store is app-local/private and not exported in this slice.
- Dependency: FS-034 diff review, FS-051 requirements review, FS-032 app-local JSONL store pattern.

## 11. Open questions

None. This slice captures and lists adversarial review findings; PR-body templating and findings analytics are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-051 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-052-review-findings-capture`
- Expected implementation PR title: `feat(FS-052): Review findings capture`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-052/amendments/`.
