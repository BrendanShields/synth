---
spec_id: FS-023
title: Request classification
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
  - docs/adrs/ADR-0004-planning-baseline-gate.md
  - docs/adrs/ADR-0009-minimal-command-native-frontend.md
---

# FS-023: Request classification

## 1. Problem statement

The git-write chain is complete, but Synth has no front end to the workflow: it does not yet decide *what kind of work* a request is. The PRD makes this the first step of every request (PRD §6): classify a request as a question, a component-level change, or project-level work, and apply the right gate — no spec for questions, a spec for component changes, and the planning baseline for project-level work. Until this exists, the workflow has no entry point.

This story adds a request classifier in the trusted core. It is a deterministic, rule-based first version (model-assisted classification can refine it later): given a natural-language request it returns a typed classification, whether a spec is required, and a short rationale (PRD §6.4 — Synth should explain its classification). The dock's natural-language input routes to it, so typing a request shows how Synth would treat it.

## 2. Requirements

- R1. The Rust core must expose `classify_request(input: String) -> RequestClassification` returning a typed classification of the request.
- R2. `RequestClassification` must serialize in camelCase with at least: `kind` (`question`, `component`, or `project`), `specRequired` (boolean), `baselineRequired` (boolean), and `rationale` (a short human-readable explanation).
- R3. Classification must be a pure, deterministic, unit-testable function with no network, filesystem, or model dependency.
- R4. The rules must classify, at minimum: explanation/question phrasing (interrogatives, or a trailing `?`, without change intent) as `question` (`specRequired: false`, `baselineRequired: false`); project-level intent (e.g. build a product, add a subsystem like authentication, redesign architecture, introduce an engine) as `project` (`specRequired: true`, `baselineRequired: true`); and component-level change intent (e.g. refactor/fix/add/update a specific component) as `component` (`specRequired: true`, `baselineRequired: false`).
- R5. Empty or whitespace-only input must classify as `question` with `specRequired: false` and a rationale noting there is nothing to classify (it must not be treated as a change).
- R6. The `rationale` must reference the basis for the decision (e.g. that it asks for explanation, or that it changes a component, or that it is project-scoped) so the user understands the classification.
- R7. The command router must route a non-empty natural-language command (`CommandKind::Natural`) to `disposition: handled` with a new `target: classification`, carrying the request text via the parsed argument. Empty input remains a no-op as in FS-003. All other route kinds/targets are unchanged.
- R8. On a handled classification route, the renderer must call `classify_request`, render the classification (kind, whether a spec/baseline is required, rationale) in a calm surface, and scroll it into view. Classification state must be transient renderer state only.
- R9. This story must not add a model/provider call, filesystem access, persistence, policy enforcement that blocks actions, or new Tauri capability permissions. It classifies and informs; it does not yet block or start work.
- R10. This story must not change the FS-001 runtime status contract and must not remove or break any existing command; the `classify_request` command and `classification` target are additive.

## 3. Acceptance criteria

- AC1. `classify_request("How does routing work?")` returns `kind: question`, `specRequired: false`, `baselineRequired: false`, with a rationale about explanation.
- AC2. `classify_request("Refactor the command dock component")` returns `kind: component`, `specRequired: true`, `baselineRequired: false`.
- AC3. `classify_request("Build a new authentication system for the app")` returns `kind: project`, `specRequired: true`, `baselineRequired: true`.
- AC4. `classify_request("")` returns `kind: question`, `specRequired: false`, with a nothing-to-classify rationale.
- AC5. Submitting natural-language text in the dock produces a handled `classification` route and renders the classification with its rationale.
- AC6. Existing routes — slash navigation, `/specs`, `/specs/<id>`, `?` ask, blocked `!`, and other kinds — behave exactly as before; only natural-language input changes from unsupported to handled.
- AC7. Rust unit coverage verifies each classification class (question, component, project, empty) and camelCase serialization.
- AC8. No code in this story calls a model/provider, accesses the filesystem, persists data, blocks an action, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Manual checks:

- Type "explain the specs index" and confirm a `question` classification (no spec required).
- Type "add a loading state to the dock" and confirm a `component` classification (spec required).
- Type "add billing and subscriptions to the product" and confirm a `project` classification (baseline required).
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of a classification for each of the three kinds.
- Short note confirming classification is deterministic and local (no model/provider call), informs without blocking, and capabilities are unchanged.

## 5. Success criteria

- SC1. Synth classifies a request into question / component / project and states which gate applies.
- SC2. Classification is deterministic, local, and explained by a rationale.
- SC3. Natural-language dock input shows the classification.
- SC4. No model call, persistence, blocking, or new capability is introduced.
- SC5. The slice stays story-sized and does not start work or enforce the gate.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Classification coverage | question/component/project/empty classified correctly | Rust tests | @BrendanShields |
| Rationale quality | every classification carries an explanatory rationale | Rust tests / manual | @BrendanShields |
| Routing stability | only natural-language becomes handled; other routes unchanged | Rust tests | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 model calls, 0 persistence, 0 blocking, 0 new capabilities | Source review | @BrendanShields |
| Contract stability | FS-001..FS-022 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- A `RequestClassification` camelCase shape and a pure, deterministic `classify_request` command.
- Routing non-empty natural-language dock input to a `classification` target.
- A calm renderer classification surface (kind, gate, rationale).
- Rust unit tests for each classification class.

### Out of scope

- Model/provider-assisted classification (a later refinement).
- Enforcing the gate (blocking edits, starting specs, or opening the planning baseline).
- Generating a spec, ADR, or plan from the request.
- Persisting classifications or learning from corrections.
- Reading the workspace to inform classification.

## 8. Technical design

### Rust/Tauri core

Add a `classify` module:

```text
RequestClassification { kind, spec_required, baseline_required, rationale }   // serde camelCase
classify_request(input: String) -> RequestClassification   // pure, #[tauri::command]
```

The classifier lowercases and trims the input, then applies ordered rules: empty → question; project-intent keywords (build, product, architecture, authentication, subsystem, platform, redesign, introduce … engine/system) → project; change-intent verbs (refactor, fix, add, update, change, rename, remove, implement) → component; interrogative/`?` without change intent → question; otherwise default to question with a rationale that it reads as a query. Each branch sets `spec_required`/`baseline_required` and a rationale. All logic is pure and unit-tested.

### Command router

Add `RouteTarget::Classification` (`classification`). A non-empty `Natural` command returns `handled` + `classification` (carrying the text via the parsed argument); empty stays a no-op. All other routing is unchanged.

### React renderer

On a handled `classification` route, call `classify_request(argument)` and render the result in an `id="classification"` surface: the kind, whether a spec/baseline is required, and the rationale, presented calmly. Scroll to it like other handled routes. Transient state; the FS-011 session log may note it.

### Styling

Reuse prose/muted styles; add a calm classification block. No alarming colors.

## 9. Impact notes

- Data model impact: introduces a `RequestClassification` IPC shape and a `classification` route target; no persisted entities.
- Security/privacy impact: none; deterministic local logic, no model/provider/filesystem access, no capability.
- Observability impact: classification can be noted in the FS-011 session log.
- Performance impact: negligible; pure string analysis.
- Migration/backward compatibility impact: additive; only natural-language routing changes from unsupported to handled.

## 10. Risks and dependencies

- Risk: misclassification by heuristics. Mitigation: the rationale makes the basis visible, the classification informs rather than blocks, and a model-assisted refinement is a later spec.
- Risk: implying the gate is enforced. Mitigation: this slice only informs; enforcement/spec-start is out of scope.
- Risk: routing change disturbing other routes. Mitigation: additive new target with tests asserting prior routes unchanged.
- Dependency: FS-022 merged (router and dock).

## 11. Open questions

None. This slice provides deterministic request classification and surfaces it; enforcement and model assistance are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-022 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-023-request-classification`
- Expected implementation PR title: `feat(FS-023): Request classification`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-023/amendments/`.
