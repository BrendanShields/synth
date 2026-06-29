---
spec_id: FS-044
title: Knowledge retrieval into context
status: Draft for review
type: feature-spec
created: 2026-06-29
owner: '@BrendanShields'
reviewers:
  - '@BrendanShields'
linked_prd: docs/PRD.md
linked_erd_hlsa: docs/engineering/ERD.md
linked_adrs:
  - docs/adrs/ADR-0008-byok-provider-strategy.md
  - docs/adrs/ADR-0010-workspace-jail.md
---

# FS-044: Knowledge retrieval into context

## 1. Problem statement

FS-039 captures knowledge as committed markdown, but nothing uses it. The PRD's Phase 7 calls for retrieval into context (PRD §18): pulling the most relevant knowledge into the model's prompt so answers are grounded in the project's own captured knowledge. This story adds deterministic retrieval (rank captured notes against a query) and an answer path that grounds the model with the top notes — closing the loop from capture (FS-039) to use.

Ranking is deterministic and local (term overlap); grounding reuses the existing provider path. It reads knowledge from the jailed workspace and changes nothing.

## 2. Requirements

- R1. Ranking captured notes against a query must be a pure, deterministic, unit-testable function `rank_knowledge(docs, query, limit) -> Vec<KnowledgeHit>` scoring by query-term overlap over each note's title and content, returning the top `limit` hits ordered by score then slug, excluding zero-score notes.
- R2. `KnowledgeHit` must serialize in camelCase with at least: `slug`, `title`, `path`, `score`, and `snippet` (a short excerpt).
- R3. The core must expose `retrieve_knowledge(query: String, limit: u32) -> Result<Vec<KnowledgeHit>, String>` that reads the workspace's `docs/knowledge/` notes (with content, confined to the jail) and returns the ranked hits; no workspace open returns a readable `Err`.
- R4. The core must expose `ask_with_context(question: String) -> Result<String, String>` that retrieves the top knowledge hits for the question, builds a prompt that includes them as grounding context plus the question, and returns the model's answer via the existing provider generation path. Building the grounded prompt must be a pure, unit-testable function.
- R5. Retrieval and grounding must be read-only over the workspace (no writes, no git/network beyond the one localhost model request in `ask_with_context`), confined to the jail, and bounded (note count/size capped).
- R6. The renderer must let the user enter a query, show the retrieved hits (title · path · snippet), and optionally ask the model grounded in the retrieved knowledge, showing the answer. Retrieval/answer display must be transient renderer state.
- R7. This story must not modify knowledge notes, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `rank_knowledge` ranks a note containing the query terms above one that does not, excludes zero-overlap notes, respects `limit`, and is deterministic for a given input.
- AC2. `KnowledgeHit` serializes in camelCase with `slug`, `title`, `path`, `score`, `snippet`.
- AC3. `retrieve_knowledge("routing", 5)` over a workspace with knowledge notes returns the matching notes as hits; no workspace open returns `Err`.
- AC4. The grounded-prompt builder includes the retrieved notes and the question.
- AC5. `ask_with_context` returns a model answer grounded in the retrieved notes (verified live in the eval).
- AC6. Retrieval is read-only and confined; notes are unchanged after retrieval/asking.
- AC7. Rust unit coverage verifies `rank_knowledge` (ordering, zero-exclusion, limit, determinism), the grounded-prompt builder, and camelCase serialization.
- AC8. No code in this story modifies notes, adds a Tauri capability, or changes the FS-001 runtime status contract.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: ranking and prompt-building are pure and unit-tested; `cargo test` performs no network. The grounded answer is verified by the eval.

Eval (required, attached to the PR): with local Ollama running `gemma4:e4b`, capture a knowledge note, then `ask_with_context` a question it answers and confirm the answer reflects the note's content.

Manual checks:

- Capture a note, query a term in it, and confirm it appears as a hit; query an unrelated term and confirm no hit.
- Ask with context and confirm the answer is grounded in the note.
- Confirm notes are unchanged and `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- The eval: the captured note, the question, and the grounded answer.
- Short note confirming read-only retrieval, jail confinement, and unchanged capabilities.

## 5. Success criteria

- SC1. Captured knowledge can be ranked against a query and surfaced.
- SC2. The model can be grounded in the project's own captured knowledge.
- SC3. Retrieval is deterministic, read-only, and confined.
- SC4. No note modification or new capability is introduced.
- SC5. The slice stays story-sized — term-overlap retrieval and grounding; embeddings/graph are deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Ranking quality | relevant notes ranked above irrelevant; zero excluded | Rust tests | @BrendanShields |
| Grounding | answer reflects retrieved note | Eval | @BrendanShields |
| Read-only | notes unchanged after retrieval/ask | Manual / source | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 note writes, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-043 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- Pure `rank_knowledge` term-overlap ranking with hits + snippets.
- `retrieve_knowledge` and `ask_with_context` commands (the latter grounding the model).
- A pure grounded-prompt builder.
- A renderer query → hits → grounded-answer surface.

### Out of scope

- Embeddings/vector search, a knowledge graph, or backlinks.
- Reranking models, citations, or chunking strategies beyond a simple snippet.
- Retrieving from sources other than `docs/knowledge/` (e.g. specs/ADRs).
- Persisting retrieval results or feedback.

## 8. Technical design

### Rust/Tauri core

Add a `knowledge` module (or extend `workspace`/`provider`):

```text
KnowledgeHit { slug, title, path, score, snippet }         // serde camelCase
rank_knowledge(docs, query, limit) -> Vec<KnowledgeHit>      // pure, deterministic
build_grounded_prompt(hits, question) -> String              // pure
retrieve_knowledge(workspace, query, limit) -> Result<Vec<KnowledgeHit>, String>   // command
ask_with_context(workspace, provider, roles, question) -> Result<String, String>   // command
```

Retrieval reads `docs/knowledge/*.md` (confined; reusing the FS-039 listing plus content reads, bounded), tokenizes lower-cased terms, scores by overlap, and snippets the first matching region. `ask_with_context` retrieves the top hits, builds the grounded prompt, and generates via the shared provider path.

### React renderer

Add a knowledge-query input that calls `retrieve_knowledge` and shows hits, and an "Ask grounded" action calling `ask_with_context`, showing the answer. Transient state.

### Styling

Reuse list/answer styles.

## 9. Impact notes

- Data model impact: introduces a `KnowledgeHit` IPC shape; no persisted entities.
- Security/privacy impact: read-only, jail-confined retrieval; one localhost model request for grounding (default Ollama on-device). No note writes, no capability.
- Observability impact: retrieval/grounded-ask can be noted in the event log.
- Performance impact: a bounded scan + term scoring; one generation for grounded ask.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: weak term-overlap ranking. Mitigation: deterministic and transparent; embeddings are a later upgrade.
- Risk: oversized context. Mitigation: bounded hit count and snippet length.
- Risk: sending notes to a remote provider. Mitigation: default provider is local Ollama (on-device); remote is the user's BYOK choice.
- Dependency: FS-039 knowledge capture, FS-008/FS-029 provider generation.

## 11. Open questions

None. This slice retrieves captured knowledge by term overlap and grounds the model; embeddings, graph, and citations are deferred.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-043 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-044-knowledge-retrieval`
- Expected implementation PR title: `feat(FS-044): Knowledge retrieval into context`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-044/amendments/`.
