---
spec_id: FS-040
title: App identity and version surfacing
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
---

# FS-040: App identity and version surfacing

## 1. Problem statement

The PRD's Phase 8 covers production readiness — release, update, and support all start from the app knowing and showing its own identity and version (PRD §19). Today nothing surfaces the running version, so a build in the field cannot be identified. This story surfaces the app's name and version (the single source already in `tauri.conf.json` / the crate) through the trusted core, and shows it quietly in the UI — the foundation any later about/update/diagnostics surface builds on.

This is read-only identity. It does not add an updater, check for updates, sign, or notarize — those require release credentials and infrastructure outside this story and are explicitly deferred.

## 2. Requirements

- R1. The Rust core must expose `app_identity() -> AppIdentity` returning the running app's `name` and `version` from the Tauri package info (the existing single source), not a duplicated constant.
- R2. `AppIdentity` must serialize in camelCase with at least `name` and `version` (version as a display string).
- R3. The renderer must show the version quietly (e.g. in the footer), consistent with the zen/minimal surface — no extra chrome or text.
- R4. This story must not add an updater or update check, must not perform any network/filesystem/signing operation, must not add new Tauri capability permissions, and must not change the FS-001 runtime status contract.

## 3. Acceptance criteria

- AC1. `app_identity` returns the name and version from the Tauri package info (matching `tauri.conf.json` / the crate version).
- AC2. `AppIdentity` serializes in camelCase (`name`, `version`).
- AC3. The renderer shows the version quietly in the footer.
- AC4. No updater, update check, network/signing operation, new capability, or FS-001 contract change is introduced.

## 4. Tests / verification plan

Automated checks required before the implementation PR is ready:

- `bun run build`
- `bun run test`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Note: `app_identity` reads Tauri package info (no pure logic to unit-test beyond serialization); a serialization-shape test covers the camelCase contract.

Manual checks:

- Launch the app and confirm the version shows in the footer, matching `tauri.conf.json`.
- Confirm `src-tauri/capabilities/*.json` is unchanged.

Evidence to attach to the implementation PR:

- Automated command output summary for the three required checks.
- Screenshot of the version in the footer.
- Short note confirming read-only identity (no updater/network) and unchanged capabilities.

## 5. Success criteria

- SC1. A field build can be identified by name and version from the UI.
- SC2. The version comes from the single existing source, not a duplicate.
- SC3. No updater, network, signing, or new capability is introduced.
- SC4. The surface stays zen/minimal.
- SC5. The slice stays story-sized — identity only, update/about deferred.

## 6. Metrics used to evaluate success

| Metric | Target | Evidence source | Review owner |
| --- | --- | --- | --- |
| Identity fidelity | name/version match the single source | Manual / source | @BrendanShields |
| Contract shape | camelCase AppIdentity | Rust test | @BrendanShields |
| Verification pass rate | 100% of required automated checks pass | PR check output | @BrendanShields |
| Scope containment | 0 updater, 0 network, 0 new capabilities | Capability/source review | @BrendanShields |
| Contract stability | FS-001..FS-039 commands/contracts intact | PR diff review | @BrendanShields |

## 7. Scope boundaries

### In scope

- An `app_identity` command returning name + version from Tauri package info.
- An `AppIdentity` camelCase shape.
- A quiet version display in the footer.

### Out of scope

- An auto-updater, update checks, or release channels.
- Code signing, notarization, or packaging changes.
- Build metadata beyond name/version (commit hash, build date), diagnostics, or an about dialog.
- Telemetry or crash reporting.

## 8. Technical design

### Rust/Tauri core

Add to `runtime_status` (or a small module):

```text
AppIdentity { name, version }            // serde camelCase
app_identity(app) -> AppIdentity          // #[tauri::command]; reads app.package_info()
```

`app_identity` reads `app.package_info()` (name + version) and returns them; the version is rendered as a string. Register the command.

### React renderer

On load, call `app_identity` and render the version quietly in the footer. Transient state.

### Styling

Reuse the footer styles; a muted, small version label.

## 9. Impact notes

- Data model impact: introduces an `AppIdentity` IPC shape; no persisted entities.
- Security/privacy impact: read-only local identity; no network, signing, or capability.
- Observability impact: the running version becomes visible — the basis for support and updates.
- Performance impact: negligible.
- Migration/backward compatibility impact: additive; all prior contracts unchanged.

## 10. Risks and dependencies

- Risk: duplicating the version. Mitigation: read from the single Tauri package-info source.
- Risk: implying an updater exists. Mitigation: identity only; updater is explicitly out of scope (and requires release credentials/infrastructure).
- Dependency: Tauri package info (existing).

## 11. Open questions

None. This slice surfaces app identity/version; the updater, signing, and notarization are deferred and require release credentials and infrastructure outside this repo's autonomous scope.

## 12. Approval and immutability

This spec is ready for PR review. Implementation may begin only after this spec PR is reviewed and merged, and after the FS-039 implementation PR has merged.

- Scope approval: Draft for review by @BrendanShields on 2026-06-29
- Spec approval status: Draft for review
- Implementation branch: `synth/fs-040-app-identity`
- Expected implementation PR title: `feat(FS-040): App identity and version surfacing`

After this spec is approved and merged, material changes must be captured as amendments in `docs/specs/FS-040/amendments/`.
