# Architecture Decision Records

Synth uses ADRs for material product, architecture, security, workflow, data model, storage, autonomy, extension, and release-impacting decisions.

## Initial decisions

- [ADR-0001: Use a Rust-native trusted runtime](ADR-0001-rust-native-runtime.md)
- [ADR-0002: Use process-based extensibility](ADR-0002-process-based-extensibility.md)
- [ADR-0003: Split project truth from private operational truth](ADR-0003-hybrid-repo-and-app-local-storage.md)
- [ADR-0004: Gate project-level implementation on a merged planning baseline](ADR-0004-planning-baseline-gate.md)
- [ADR-0005: Establish CODEOWNERS during project bootstrap](ADR-0005-codeowners-during-bootstrap.md)
- [ADR-0006: Use story-sized immutable feature specs](ADR-0006-story-sized-immutable-feature-specs.md)
- [ADR-0007: Always pause for amendment approval](ADR-0007-amendments-always-pause-work.md)
- [ADR-0008: Use a bring-your-own-key provider strategy for v1](ADR-0008-byok-provider-strategy.md)
- [ADR-0009: Build a minimal command-native frontend](ADR-0009-minimal-command-native-frontend.md)

## ADR rules

- Use ADRs only for material decisions; keep small clarifications inline in the relevant spec.
- Link each ADR to the PRD, ERD/HLSA, feature spec, amendment, or release that depends on it.
- Do not rewrite accepted ADRs to change history. Create a superseding ADR when a material decision changes.
