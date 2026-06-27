# Synth

Synth is a local-first desktop harness for agentic software development. It is built on one conviction:

> The harness is the product.

Models are hot-swappable. Synth's value is the scaffold around them: documents, specs, approvals, tool boundaries, observability, commits, review, and PRs.

## Status

Synth is in the planning-baseline phase. The repository currently contains the initialized Tauri + Bun + React scaffold and the first product/architecture documents.

## Product direction

Synth's first shippable product is a **documents-first, spec-to-PR local harness** for existing repositories.

The intended workflow is:

```text
Open an existing repo
→ create or validate PRD + ERD/HLSA
→ define success criteria and metrics
→ record material decisions as ADRs
→ open and merge the planning PR
→ scope epics/releases or story-sized feature specs
→ approve each spec
→ implement each spec on its own branch
→ verify, review, commit, and open a PR per spec
```

Project-level implementation is blocked until the planning baseline has been reviewed and merged. This is intentional: Synth exists to reduce ambiguity before code changes begin.

## Core principles

- **Documents first:** PRD and ERD/HLSA are required for project-level work.
- **Human in the loop where it matters:** humans review planning and final outputs; agents execute approved work inside boundaries.
- **Story-sized specs:** large features are decomposed into epics/releases and then into small feature specs.
- **Immutable specs:** approved specs are not edited in place; deviations create explicit amendments.
- **Success is explicit:** every project and feature defines success criteria and metrics.
- **Security-first runtime:** the model proposes, the trusted runtime enforces.
- **Observable by default:** sessions, tool calls, approvals, errors, amendments, and review findings are structured events.
- **Minimal command-native UI:** one focused artifact at a time, controlled by the bottom command input.

## Architecture direction

Synth is designed as a Rust-native Tauri app with a thin React renderer.

```text
React renderer = visual surface
Rust/Tauri core = trusted product kernel
External processes = supervised extension/tool boundary
Repo docs = committed project truth
App-local store = private operational truth
```

Initial provider targets are:

- OpenAI-compatible endpoints
- Ollama

Future extension points include MCP servers, skills, subagents, deterministic workflow graphs, and an OKF-style project knowledge system.

## Documents

- [PRD](docs/PRD.md)
- [ERD / HLSA](docs/engineering/ERD.md)

## Development

Prerequisites:

- [Bun](https://bun.sh/)
- [Rust](https://www.rust-lang.org/tools/install)
- Tauri platform prerequisites for your OS

Install dependencies:

```bash
bun install
```

Run the web dev server:

```bash
bun run dev
```

Run the Tauri app:

```bash
bun run tauri dev
```

Build the frontend:

```bash
bun run build
```

Build the desktop bundle:

```bash
bun run tauri build
```

## Repository layout

```text
docs/
  PRD.md
  engineering/
    ERD.md
src/
  React renderer scaffold
src-tauri/
  Rust/Tauri application scaffold
```

Operational data such as transcripts, audit logs, approvals, tool logs, and local session replay data should remain app-local and outside the repository by default.

## Roadmap snapshot

1. Planning substrate: PRD, ERD/HLSA, CODEOWNERS, ADRs, feature spec templates.
2. Rust-native walking skeleton: Tauri shell, typed IPC, event stream, provider abstraction.
3. Security and workspace kernel: jail, policy, approvals, credential handling, audit log.
4. Spec-to-PR loop: request classification, specs, amendments, tasks, commits, PR generation.
5. Review and observability: diff review, replay, adversarial review, improvement signals.
6. Extensibility: MCP, skills, subagents, extension permissions.
7. Workflow graph and knowledge system.
8. Production hardening: signing, notarization, auto-update, performance.

## License

No license has been selected yet.
