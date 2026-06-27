# ADR-0003: Split project truth from private operational truth

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md)

## Context

Synth needs durable project artifacts that can be reviewed in git, while also recording private operational data such as transcripts, tool calls, approvals, credentials, and replay data. Committing all operational state would expose sensitive information and create noisy diffs.

## Decision

Synth will use hybrid storage. Repo-versioned project truth lives under `docs/`. Private operational truth lives outside the repository in app-local storage by default.

## Consequences

- PRDs, ERD/HLSA documents, ADRs, specs, amendments, releases, and knowledge docs are reviewable through normal PR workflows.
- Raw transcripts, audit logs, credentials, provider metadata, tool-call logs, and replay data are not committed by default.
- Export flows must redact sensitive data before sharing operational bundles.
- Future storage feature specs must preserve the repo/app-local boundary defined in the ERD.
