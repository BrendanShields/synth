# Synth PRD

**Version:** 0.1.0  
**Status:** Draft for review  
**Product:** Synth  
**Document type:** Product Requirements Document  
**Primary audience:** Product, engineering, design, and technical reviewers  

---

## 1. Summary

Synth is a local-first, cross-platform desktop application for agentic software development. It is built on one conviction: **the harness is the product**.

Models will improve, change, and compete. Synth's durable value is the scaffold around the model: how work is specified, reviewed, executed, observed, secured, corrected, committed, and learned from. Synth exists to reduce ambiguity before development begins, keep humans in the loop where judgment matters, and let agents execute approved work safely and inspectably.

The first shippable product is a **documents-first, spec-to-PR local harness** for existing repositories. A user opens a repo, Synth helps create or validate the project baseline documents, those documents are reviewed and merged through a planning PR, and only then does implementation work begin. Larger initiatives are decomposed into epics or releases, then into story-sized feature specs. Each approved spec is implemented on its own branch and produces its own PR.

Synth v1 is not a generic chat assistant, not a full IDE, and not a visual workflow platform. It is a disciplined planning and implementation harness for senior engineers and technical teams that want AI acceleration without silent scope drift.

---

## 2. Product premise

AI coding tools fail most often before the first line of code is changed: requirements are vague, assumptions are hidden, acceptance criteria are incomplete, and review happens after the model has already made hundreds of implicit decisions.

Synth addresses this by making planning artifacts first-class deliverables.

The core workflow is:

```text
Project request
→ classify request
→ create or validate PRD + ERD/HLSA
→ define success criteria and metrics
→ record material decisions as ADRs
→ create planning branch
→ commit planning documents
→ open planning PR
→ merge planning PR
→ scope epic/release or story-sized feature specs
→ approve each spec
→ implement each spec on its own branch
→ verify against tests, acceptance criteria, and metrics
→ adversarial review
→ human review
→ open PR per completed spec
```

Planning is not a side activity. Planning is the first product output.

Synth's north-star behavior is to ask the questions a strong staff engineer, architect, security reviewer, and product lead would ask before work becomes code. Once ambiguity is reduced and the plan is approved, Synth can execute with increasing autonomy inside clear boundaries.

---

## 3. Target users

### 3.1 Primary user: senior engineer / tech lead

The primary v1 user is a senior engineer or technical lead who values correctness, reviewability, architecture clarity, and controlled autonomy.

They want Synth to:

- turn vague intent into clear product and technical documents;
- prevent premature implementation;
- preserve design decisions as durable artifacts;
- enforce story-sized implementation specs;
- produce reviewable diffs and PRs;
- make agent work observable and auditable;
- support high autonomy only after the contract is clear.

### 3.2 Co-primary advanced user: workflow power user

A second priority user is the power user who wants to compose workflows with MCP, skills, subagents, model roles, and deterministic execution graphs.

For v1, Synth must expose the architectural seams that make this future possible. The full visual workflow builder is deferred, but the product should not be designed in a way that blocks it.

### 3.3 Fast-follow user: security-conscious professional

A fast-follow user is the security-conscious professional who cares about local-first operation, explicit approvals, audit logs, workspace boundaries, credential handling, and policy.

Security is not deferred. The heavier security-product experience can mature over time, but the v1 harness must be safe by default.

---

## 4. Product principles

### 4.1 Documents first

Project-level work begins with a PRD and ERD/HLSA. Implementation is blocked until the planning PR containing those documents is merged.

### 4.2 Reduce ambiguity before code

Synth should ask clarifying questions before writing code. It should identify missing requirements, inconsistent requirements, ambiguous language, wrong levels of detail, missing acceptance criteria, and unclear success metrics before implementation begins.

### 4.3 Human judgment at planning and review

Humans are most valuable when defining intent, approving trade-offs, judging risk, and reviewing final outcomes. Synth should optimize for human-in-the-loop planning and human-in-the-loop review, not constant manual steering during routine implementation.

### 4.4 Story-sized implementation specs

Feature specs are the unit of implementation. A feature spec should be approximately the size of a user story. Larger initiatives become epics or releases that decompose into multiple specs.

### 4.5 Immutable specs with explicit amendments

Approved feature specs are immutable. If the work changes in flight, Synth pauses and creates an amendment. Amendments record the deviation and become product-quality telemetry.

### 4.6 Security is a runtime boundary

The model proposes. The trusted runtime enforces. File access, command execution, network access, credential access, and out-of-workspace operations must route through policy and approval boundaries.

### 4.7 Observability is a first-class product surface

Synth records conversations, tool calls, approvals, errors, interruptions, loops, negative sentiment, amendments, test failures, and review findings as structured data. These records support replay, audit, evaluation, and harness improvement.

### 4.8 The interface should disappear until needed

Synth should feel quiet, minimal, editorial, and keyboard-native. Most of the time the UI should show one focused artifact, a small contextual status, and a command/input dock.

---

## 5. V1 product promise

Synth v1 promises:

> Open an existing repository, create or validate the project planning baseline, merge that baseline through a planning PR, then scope and implement story-sized specs on their own branches, with verification, review, and PRs generated from the approved specs.

V1 includes:

- local-first desktop app for macOS and Windows;
- single-user workflow;
- bring-your-own-key model configuration;
- OpenAI-compatible endpoint support;
- Ollama support;
- Rust-native trusted runtime;
- documents-first planning workflow;
- PRD + ERD/HLSA project baseline;
- setup/code ownership workflow;
- story-sized feature specs;
- immutable specs and amendments;
- ADR capture for material decisions;
- supervised and high-autonomy execution modes;
- safe file/tool/command boundaries;
- session and tool-call observability;
- diff review;
- logical commits;
- PR preparation/creation workflow according to available repo/provider integration.

V1 does not need the full visual workflow builder, mature MCP marketplace, complex multi-agent orchestration, team/cloud sync, or deep knowledge graph experience. Those remain part of the product roadmap.

---

## 6. Request classification

Synth classifies every request before deciding what gates apply.

### 6.1 Question or explanation request

Examples:

- “How does this module work?”
- “What does this error mean?”
- “Explain the repo structure.”
- “Compare these approaches.”

No spec is required. Synth may inspect files and answer directly. If the answer produces durable project knowledge, Synth may suggest saving it as a knowledge document, ADR, or note.

### 6.2 Component-level change

Examples:

- “Refactor this component.”
- “Add loading states to this page.”
- “Fix this hook.”
- “Change this API route.”

A plan/spec is required before edits. The spec can be scoped to the component, but it must still include the non-negotiable feature-spec sections.

### 6.3 Project-level work

Examples:

- “Build this product.”
- “Add authentication.”
- “Redesign the app architecture.”
- “Introduce a workflow engine.”
- “Create the spec-to-PR system.”

Project-level work requires the project baseline:

```text
PRD
ERD / HLSA
Relevant feature spec or epic/release plan
```

Implementation is blocked until the PRD and ERD/HLSA planning PR is merged.

### 6.4 Borderline requests

If Synth is unsure, it should explain its classification:

```text
I’m treating this as component-level work because it changes behavior in an existing module. A plan/spec is required before edits.
```

or:

```text
I’m treating this as a question because you asked for explanation only. No spec is required unless you ask me to change code.
```

---

## 7. Project baseline documents

### 7.1 Required documents for project-level implementation

For project-level work, the minimum required project baseline is:

```text
PRD
ERD / HLSA
```

The model may recommend additional documents, but the user can decline optional documents. Declined documentation recommendations are saved with scope, reason, and revisit conditions.

### 7.2 Optional recommended documents

Synth may recommend:

- `Security.md`
- `Observability.md`
- `Coding-Patterns.md`
- `Runtime.md`
- `Workflow-System.md`
- `Testing-and-Evals.md`
- `Git-Automation.md`
- `Threat-Model.md`
- `API-Contracts.md`
- ADRs

A declined recommendation should not become repeated noise. Synth can resurface it only when context materially changes.

### 7.3 Planning PR gate

For project-level work:

1. Synth creates or validates PRD and ERD/HLSA.
2. Synth defines project success criteria and metrics.
3. Synth records material decisions as ADRs.
4. Synth creates a planning branch.
5. Synth commits the planning documents.
6. Synth opens a planning PR.
7. The planning PR must be reviewed and merged.
8. Only then can implementation begin.

This ensures the right people review the product and architecture baseline before code changes start.

---

## 8. Code ownership and reviewers

During project setup, Synth establishes code ownership.

If a CODEOWNERS file already exists, Synth parses and respects it. It must not silently overwrite existing ownership rules.

Detection order for v1:

```text
.github/CODEOWNERS
CODEOWNERS
/docs/CODEOWNERS
```

If no CODEOWNERS file exists, Synth interactively defines owners and generates one in the setup/planning branch.

The generated ownership should cover both code and planning artifacts:

```text
*                       @default-owner
/docs/PRD.md            @product-owner
/docs/engineering/**    @tech-lead
/docs/specs/**          @product-owner @tech-lead
/docs/adrs/**           @tech-lead
/src/<area>/**          @area-owner
```

CODEOWNERS becomes the reviewer source of truth:

```text
Planning PR      → owners of PRD and engineering docs
Feature spec PR  → owners of spec docs and touched code paths
```

The initial setup PR that introduces CODEOWNERS is approved by the project initiator. Every PR after that follows CODEOWNERS.

---

## 9. Feature specs

### 9.1 Feature specs are full but story-sized

A feature spec is the implementation contract for one story-sized behavior change.

A feature spec should not contain a whole system, module, or epic. If the requested work is too large, Synth decomposes it into an epic or release with multiple specs.

### 9.2 Non-negotiable sections

Every feature spec must include:

1. Problem statement
2. Requirements
3. Acceptance criteria
4. Tests / verification plan
5. Success criteria
6. Metrics used to evaluate success

If any of these are missing, implementation cannot begin.

### 9.3 Optional sections

Synth may include or omit optional sections based on risk and relevance:

- user story;
- background/context;
- out of scope;
- UX notes;
- technical design;
- data model impact;
- security/privacy impact;
- observability impact;
- performance impact;
- migration/backward compatibility;
- rollout plan;
- risks;
- open questions;
- dependencies;
- release notes.

If the model omits an optional section where omission may matter, it should say why. For example:

```text
Data model impact omitted because this feature does not change persisted entities.
```

Rollout can be handled at the release/epic level when multiple specs ship together.

### 9.4 Interactive generation

Feature specs are generated interactively.

The flow is:

```text
classify request
→ draft problem statement
→ confirm/correct with user
→ draft requirements
→ analyze requirement quality
→ ask focused clarification questions
→ draft acceptance criteria
→ define success criteria and metrics
→ draft tests / verification plan
→ decide optional sections
→ save material decisions as ADRs
→ ask for final approval
→ create immutable approved spec
```

This follows a requirements-first approach: prompt → requirements → design → tasks → code.

### 9.5 Requirement quality gate

Synth checks requirements for:

- wrong level of detail;
- ambiguity;
- inconsistency;
- incompleteness;
- implementation leakage;
- missing error paths;
- unclear acceptance criteria;
- missing success metrics.

Good requirements should be:

- testable;
- solution-free where appropriate;
- unambiguous;
- consistent;
- complete for the story scope.

When intent cannot be inferred safely, Synth asks the user concrete questions rather than choosing silently.

---

## 10. Epics and releases

Large initiatives are not feature specs. They are epics or releases that group story-sized specs.

An epic/release may define:

- release goal;
- included feature specs;
- sequencing;
- rollout plan;
- migration plan;
- release-level risks;
- release-level observability;
- release-level QA;
- release notes;
- rollback strategy.

Example:

```text
Epic: Spec-to-PR Workflow
  FS-001 Project document gate
  FS-002 Feature spec generator
  FS-003 Immutable spec approval
  FS-004 Amendment flow
  FS-005 Branch and commit automation
  FS-006 PR generation
```

Each included spec still has its own branch, implementation, verification, review, and PR.

---

## 11. Immutable specs and amendments

Approved specs are immutable. Synth does not silently edit approved specs.

If implementation reveals that a spec is incomplete, wrong, too broad, ambiguous, or blocked by an unexpected constraint, Synth pauses and creates an amendment.

Amendments are always approval-gated. This applies in supervised mode and high-autonomy mode.

The amendment flow is:

```text
implementation detects deviation
→ Synth pauses work
→ Synth drafts amendment
→ user reviews amendment
→ amendment is approved, rejected, or revised
→ tasks/checks are updated
→ implementation resumes only after approval
```

An amendment records:

- linked feature spec;
- trigger;
- original clause affected;
- proposed change;
- reason for change;
- impact on requirements;
- impact on acceptance criteria;
- impact on tests;
- impact on success metrics;
- impact on tasks;
- impact on release scope, if any;
- phase detected;
- source of detection;
- approval status.

Amendments also create deviation telemetry, including:

```text
missing_requirement
ambiguous_requirement
incorrect_acceptance_criteria
underestimated_scope
hidden_dependency
test_gap
design_conflict
user_preference_change
model_misclassification
implementation_constraint
security_policy_gap
```

Synth uses this telemetry to improve future spec generation and adversarial review.

---

## 12. ADR policy

Synth creates ADRs for material decisions.

Small clarifications stay inline in the spec. Material decisions become ADRs.

Material decisions include:

- architecture decisions;
- product behavior decisions;
- security decisions;
- workflow decisions;
- data model decisions;
- storage decisions;
- autonomy decisions;
- extension decisions;
- release-impacting decisions.

An ADR includes:

```text
title
status
context
decision
consequences
linked PRD/HLSA/spec/release
created date
```

Initial ADRs expected for Synth include:

- Rust-native runtime;
- process-based extensibility;
- hybrid repo/app-local storage;
- PRD + ERD/HLSA planning gate;
- CODEOWNERS setup during project bootstrap;
- story-sized immutable feature specs;
- always-pause amendment approval;
- BYOK provider strategy;
- minimal command-native frontend.

---

## 13. Autonomy model

Synth supports two execution modes:

```text
Supervised
High autonomy
```

The user can toggle between them.

### 13.1 Supervised mode

After spec/plan approval, Synth can propose edits, run checks, and prepare commits, but it asks for approval at key steps and for risky actions.

### 13.2 High-autonomy mode

After spec/plan approval, Synth can execute approved tasks with fewer interruptions. It may edit, run checks, and create logical commits within the approved scope.

High autonomy is not a security bypass. It does not bypass:

- out-of-workspace access;
- destructive actions;
- credential access;
- network access;
- dependency installs above risk threshold;
- PR creation when policy requires review;
- amendment approval.

### 13.3 Autonomy visibility

The active autonomy mode must be visible at all times. The user should never wonder whether Synth is acting in supervised or autonomous mode.

Autonomy changes are policy state, recorded in events, and applied at safe boundaries.

---

## 14. Model provider strategy

Synth v1 is BYOK.

The first provider families are:

1. OpenAI-compatible endpoints;
2. Ollama.

Synth should implement a provider abstraction from the start, but not attempt to support every provider in v1.

Provider configuration includes:

- provider type;
- base URL;
- authentication method;
- model list strategy;
- context window;
- streaming support;
- tool-call support;
- structured-output support;
- cost metadata when available.

### 14.1 Model roles

Synth stores a default model plus optional role overrides.

```text
default_model
planner_model override
builder_model override
adversary_model override
summarizer_model override
requirements_critic_model override
```

If a role override is not configured, Synth uses the default model.

### 14.2 Roadmap to auto-selection

The long-term direction is auto-select with user overrides.

Synth should evolve through:

1. manual default + optional role overrides;
2. capability-aware suggestions;
3. policy-based auto-select by role, task, context, cost, privacy, and latency;
4. outcome-informed recommendations using amendments, failed tests, review findings, loops, and user corrections.

---

## 15. Runtime and architecture principles

Synth uses a Rust-native runtime.

The React renderer is a thin visual surface. The Tauri/Rust backend is the trusted product kernel.

The Rust backend owns:

- provider streaming;
- agent loop;
- tool-call lifecycle;
- workspace jail;
- approval/policy engine;
- command execution;
- file operations;
- session tree;
- event store;
- compaction;
- spec workflow;
- task tracking;
- git automation;
- audit logging.

PI remains a design reference, not a runtime dependency. Synth can borrow concepts such as provider abstraction, session harness, event streaming, compaction, tool lifecycle, and hooks, but the product kernel is native.

---

## 16. Extensibility

Synth uses process-based extensibility.

MCP servers, skills, custom tools, and subagents run out-of-process over typed protocols. The Rust core remains the trusted broker.

Extension rules:

- extensions declare capabilities;
- extensions do not receive unrestricted file/shell/network access;
- extension actions route through Synth policy;
- extension calls are logged with identity;
- extension errors and interruptions are observable;
- workspace-level allowlists control extension access.

Full visual workflow composition is deferred beyond v1, but the architecture should support it.

---

## 17. Storage model

Synth uses hybrid storage.

### 17.1 Repo-versioned artifacts

Project truth lives in the repository:

```text
docs/PRD.md
docs/engineering/HLSA.md
docs/engineering/<technical-doc>.md
docs/specs/<spec-id>/spec.md
docs/specs/<spec-id>/amendments/*.md
docs/adrs/ADR-*.md
docs/releases/<release-id>.md
docs/knowledge/*.md
```

### 17.2 App-local private data

Private operational records live in app-local storage:

```text
sessions
raw transcripts
tool-call logs
provider metadata
approvals
audit logs
credentials
session replay data
loop/stall/sentiment metadata
local eval runs
```

These are not committed by default. Users may export redacted session bundles explicitly.

### 17.3 Knowledge system

Synth's knowledge system should follow an OKF-style approach: markdown files with YAML frontmatter, human-readable, agent-readable, diffable, and portable.

The v1 priority is the document substrate. Graph retrieval and rich knowledge visualization are roadmap items.

---

## 18. Git and PR workflow

Synth treats branches, commits, and PRs as product artifacts.

### 18.1 Planning branch and PR

For project-level work:

```text
branch: synth/docs/bootstrap-product-architecture
commits:
  docs: add PRD v0.1
  docs: add HLSA v0.1
  docs: add initial ADRs
PR:
  title: Add initial product and architecture baseline
```

This PR must be merged before implementation begins.

### 18.2 Spec branches and PRs

Each feature spec gets its own branch and PR.

```text
branch: synth/fs-006-command-palette-navigation
commits:
  docs: add FS-006 command palette navigation
  feat: add command palette shell
  test: cover command navigation behavior
PR:
  title: FS-006 Command palette navigation
```

The PR body is generated from:

- immutable spec;
- amendments;
- ADRs;
- tasks completed;
- acceptance criteria;
- tests/checks run;
- metrics evidence;
- adversarial review findings;
- risk notes.

### 18.3 Logical commits

Synth creates commits in logical steps. Commits should be understandable to humans and trace back to the spec/task that produced them.

Synth must avoid mixing unrelated work into a spec PR.

---

## 19. Product experience and interface direction

Synth is a minimal command-native workspace.

The interface principle is:

```text
One artifact at a time.
Input controls the artifact.
Command palette navigates.
Panels appear only when context demands them.
```

Synth should not look or feel like a traditional IDE cockpit. It should be quiet, editorial, pale, precise, frosted, keyboard-native, and low-noise.

### 19.1 App shell

The shell contains:

```text
tiny contextual status
one central artifact
bottom command/input dock
tiny keyboard hints
```

The center stage changes by mode.

### 19.2 Core views

Primary views:

- Reader view for PRD, HLSA, specs, ADRs, amendments, releases, and knowledge docs;
- Session/chat view for live or replayed workflow execution;
- Slash command overlay for navigation and app control;
- Diff review view for focused code review and comment-to-fix;
- Inline render view for charts, previews, generated figures, and PR cards;
- Approval/amendment overlays for blocking trust gates.

### 19.3 Command/input dock

The bottom input is the control center.

It supports:

- natural-language input;
- slash commands;
- search;
- current-artifact questions;
- steering active turns;
- shell commands through policy;
- references to specs/docs/files.

Command grammar:

```text
/   navigate or run command
?   ask current artifact
@   reference file/spec/session/workflow/doc
#   reference issue/spec/release/tag
!   shell command, policy-gated
>   steer current agent turn
```

Examples:

```text
/sessions
/specs
/approve
? what does this PRD imply for v1?
@src/runtime/loop.rs explain this change
! cargo test
> stop and create an amendment
```

### 19.4 Visual aesthetic

Use:

- off-white paper backgrounds;
- serif typography for prose and transcripts;
- monospace typography for metadata, commands, timestamps, file paths, and code;
- frosted-glass command dock;
- large whitespace;
- soft shadows;
- low-contrast borders;
- minimal state colors.

Avoid:

- busy dashboards;
- permanent sidebars;
- permanent terminal panes;
- colorful AI gradients;
- chat bubbles;
- IDE chrome unless the current task demands it.

---

## 20. Security and trust

Synth is local-first, but local-first is not enough. The app executes in the user's real environment and must enforce safe boundaries.

Security requirements:

- workspace jailing by default;
- approval for out-of-workspace access;
- approval for destructive actions;
- approval for network access when policy requires;
- approval for credential access;
- clear command risk classification;
- redaction of secrets in logs and model context;
- explicit extension permissions;
- local audit log;
- user-visible autonomy mode;
- no silent escalation.

High autonomy can reduce routine prompts, but it cannot bypass trust boundaries.

---

## 21. Observability and improvement loop

Synth records structured events for:

- user messages;
- model messages;
- provider requests;
- tool calls;
- tool results;
- approvals;
- denials;
- command execution;
- file reads/writes/edits;
- diffs;
- test/check runs;
- errors;
- interruptions;
- loops/stalls;
- negative sentiment;
- amendments;
- adversarial review findings;
- commits and PRs.

Observability serves four purposes:

1. user trust and replay;
2. debugging failed runs;
3. audit and compliance;
4. improving Synth's planning and execution quality.

Synth should surface insights such as:

- specs with high amendment rates;
- recurring missing requirements;
- models that fail particular roles;
- tools that produce repeated errors;
- workflows with loops or stalls;
- tests that often expose spec gaps.

---

## 22. Adversarial review

Every meaningful workflow should include adversarial review.

Adversarial review should use fresh context where possible and preferably a different model from the builder. The adversary's role is not to generate more code; it is to find concrete risks, requirement mismatches, security concerns, test gaps, and implementation deviations.

Review findings attach to the feature spec and PR.

Adversarial review can happen at multiple points:

- requirements review;
- design review;
- pre-implementation plan review;
- post-implementation diff review;
- test/check review;
- PR readiness review.

Full multi-agent orchestration is deferred, but a single adversarial review pass should be part of the product model early.

---

## 23. Context management and long-running work

Synth must support long-running tasks without losing context.

Requirements:

- persistent task tracking;
- session tree;
- resumable sessions;
- event replay;
- auto-compaction;
- structured summaries;
- file-read and file-modified tracking;
- recovery after interruption;
- visible blocked state;
- continuation from last safe point.

Compaction summaries should preserve exact file paths, function names, error messages, decisions, next steps, and modified files.

---

## 24. Success criteria and metrics

Every project and feature must define explicit success criteria and metrics.

### 24.1 Project-level success criteria

Synth succeeds at the project level when:

- project-level implementation cannot begin until PRD and ERD/HLSA are reviewed and merged;
- code ownership is established or respected;
- feature work decomposes into story-sized specs;
- each spec includes requirements, acceptance criteria, tests, success criteria, and metrics;
- deviations become amendments rather than silent scope changes;
- implementation work produces reviewable branches, commits, and PRs;
- humans review planning and final outputs;
- security-sensitive actions are gated;
- observability data supports replay and improvement.

### 24.2 Product metrics

Initial product metrics:

- time from project prompt to merged planning PR;
- number of ambiguity questions resolved before implementation;
- percentage of implementation work backed by approved specs;
- percentage of specs with complete success criteria and metrics;
- amendment rate per spec;
- defects caught by adversarial review before PR;
- percentage of specs completed without unapproved scope drift;
- test/check pass rate before PR;
- user approval rate for generated planning docs;
- number of prevented unsafe actions;
- session replay completeness;
- time from approved spec to PR-ready branch.

### 24.3 Feature-level success criteria

Every feature spec defines its own success criteria and metrics. A feature is not complete because code was written; it is complete when the approved acceptance criteria, verification plan, and success metrics have evidence.

---

## 25. Roadmap

### Phase 0 — Planning substrate

- PRD and HLSA document model;
- CODEOWNERS setup;
- ADR model;
- feature spec template;
- amendment model;
- docs-as-code layout.

### Phase 1 — Rust-native walking skeleton

- Tauri shell;
- thin React renderer;
- Rust runtime event stream;
- provider abstraction;
- OpenAI-compatible provider;
- Ollama provider;
- command/input dock prototype;
- reader/session modes.

### Phase 2 — Security and workspace kernel

- project open/create;
- workspace jail;
- policy engine;
- approval prompts;
- credential storage;
- audit events;
- safe command execution.

### Phase 3 — Spec-to-PR loop

- request classification;
- PRD/HLSA planning gate;
- feature spec generation;
- requirements quality gate;
- tasks;
- immutable approvals;
- amendments;
- logical commits;
- PR generation.

### Phase 4 — Review and observability

- live diff view;
- terminal/tool output view;
- session replay;
- adversarial review;
- loop/stall/error metadata;
- improvement insights.

### Phase 5 — Extensibility

- MCP process integration;
- skill registry;
- subagent definitions;
- extension permissions;
- extension observability.

### Phase 6 — Workflow graph

- deterministic workflow schema;
- nodes and edges;
- approval gates;
- visual workflow builder;
- workflow run timeline;
- workflow versioning.

### Phase 7 — Knowledge system

- OKF-style knowledge bundles;
- project knowledge graph;
- retrieval into context;
- code/doc drift detection;
- citation-aware updates.

### Phase 8 — Production hardening

- macOS signing and notarization;
- Windows signing;
- auto-update;
- backup/export;
- performance on large repos;
- installer and release channels.

---

## 26. Non-goals for v1

V1 does not attempt to provide:

- hosted team collaboration;
- cloud sync;
- full visual workflow builder;
- full MCP marketplace;
- full multi-agent orchestration;
- enterprise policy administration;
- every model provider;
- a full IDE replacement;
- automatic implementation without planning review;
- implementation before merged planning baseline for project-level work.

---

## 27. Reference influence

This PRD adopts a requirements-first planning model inspired by spec-driven development workflows that move from prompt to requirements, design, tasks, and code. Synth extends that model into a desktop coding harness where planning documents, feature specs, ADRs, amendments, ownership, branches, commits, PRs, review, and observability are all first-class product concepts.

The product direction also incorporates the architecture principle that model capability is only one part of agentic coding quality. The runtime loop, execution boundary, tools, context management, session model, security policy, and review flow are equally central.

---

## 28. Glossary

**ADR:** Architecture Decision Record. A durable record of a material decision and its consequences.

**Amendment:** A formal change to an approved immutable feature spec. Required when implementation discovers a deviation.

**CODEOWNERS:** Repository file defining required owners/reviewers for paths.

**ERD/HLSA:** Engineering/system architecture documents defining high-level architecture, data relationships, boundaries, and technical decisions.

**Feature spec:** A story-sized implementation contract containing problem statement, requirements, acceptance criteria, tests, success criteria, and metrics.

**Planning PR:** The PR that introduces or updates the PRD, ERD/HLSA, ownership, and ADR baseline before implementation begins.

**Spec-to-PR:** Synth's workflow for turning approved specs into implementation branches, verification evidence, and pull requests.

**Workflow graph:** A future deterministic representation of steps, gates, tools, agents, and approvals.
