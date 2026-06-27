# CODEOWNERS setup

**Status:** Initial setup
**Owner source of truth:** [`.github/CODEOWNERS`](../../.github/CODEOWNERS)
**Companion documents:** [`../PRD.md`](../PRD.md), [`ERD.md`](ERD.md)

---

## Purpose

Synth treats code ownership as part of the planning substrate. Ownership rules identify who reviews product documents, engineering decisions, feature specs, amendments, runtime code, and repository governance changes.

For this repository, ownership is intentionally simple while Synth is single-maintainer and pre-v1: all paths resolve to `@BrendanShields`. The rules are still split by product area so future teams can replace individual owners without redesigning the ownership model.

---

## Detection order

Synth v1 detects existing ownership files in this order:

```text
.github/CODEOWNERS
CODEOWNERS
/docs/CODEOWNERS
```

If a file exists, Synth must parse and respect it. Synth must not silently overwrite ownership rules.

---

## Review policy

Ownership applies to planning and implementation work:

- **Planning PRs** require reviewers for the PRD, ERD/HLSA, ADRs, templates, and ownership rules touched by the PR.
- **Feature spec PRs** require reviewers for the spec/amendment documents and the code paths touched by the implementation.
- **Governance changes** to CODEOWNERS, templates, ADR policy, or planning gates require the repository governance owner.

The initial setup PR that introduces ownership can be approved by the project initiator. Subsequent PRs follow CODEOWNERS.

---

## Maintenance rules

- Keep the actual GitHub-enforced file at `.github/CODEOWNERS` unless the repository has a reason to move it.
- Use specific rules for planning artifacts even when the current owner is the same person as the default owner.
- Put broad fallback rules before specific rules.
- Prefer team handles for shared ownership once Synth moves beyond a single maintainer.
- Update this document whenever ownership semantics change.

---

## Future team template

Use [`../templates/CODEOWNERS.template`](../templates/CODEOWNERS.template) when bootstrapping a new repository or migrating Synth to team ownership.
