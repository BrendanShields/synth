# ADR-0005: Establish CODEOWNERS during project bootstrap

**Status:** Accepted
**Created:** 2026-06-27
**Linked documents:** [`docs/PRD.md`](../PRD.md), [`docs/engineering/ERD.md`](../engineering/ERD.md), [`docs/engineering/CODEOWNERS.md`](../engineering/CODEOWNERS.md)

## Context

Synth routes planning and implementation through PRs. Review quality depends on consistently identifying who owns product requirements, engineering decisions, specs, amendments, runtime code, and repository governance.

## Decision

Synth will detect existing CODEOWNERS files during setup and respect them. If no ownership file exists, Synth will generate one in the setup/planning branch. The preferred location is `.github/CODEOWNERS`.

## Consequences

- CODEOWNERS becomes the reviewer source of truth for planning PRs, feature spec PRs, and implementation PRs.
- Synth must not silently overwrite existing ownership rules.
- The first ownership rules may be simple for single-maintainer repositories, but they must be structured so teams can refine them later.
- Ownership changes are governance changes and require review.
