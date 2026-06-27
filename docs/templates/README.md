# Planning templates

These templates define the repo-versioned planning artifacts used by Synth's documents-first workflow.

## Templates

- [`CODEOWNERS.template`](CODEOWNERS.template) — starting point for repository ownership setup.
- [`feature-spec.md`](feature-spec.md) — story-sized implementation contract template.
- [`amendment.md`](amendment.md) — approval-gated change record for immutable specs.

## Usage rules

- Copy templates into their target location before filling them out.
- Feature specs live at `docs/specs/<spec-id>/spec.md`.
- Amendments live at `docs/specs/<spec-id>/amendments/<amendment-id>.md`.
- Approved specs are immutable; material changes require an amendment.
- Remove template instructional text before approval.
