use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecsIndex {
    pub artifact_type: String,
    pub generated_from: String,
    pub specs: Vec<SpecIndexEntry>,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecIndexEntry {
    pub spec_id: String,
    pub title: String,
    pub status: String,
    pub path: String,
    pub implementation_branch: String,
    pub route: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticSpecDetail {
    pub spec_id: String,
    pub title: String,
    pub status: String,
    pub path: String,
    pub implementation_branch: String,
    pub route: String,
    pub summary: String,
    pub scope: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn static_spec_details() -> Vec<StaticSpecDetail> {
    vec![
        spec_detail(
            "FS-001",
            "Runtime event bridge and Synth shell",
            "Draft for review",
            "docs/specs/FS-001/spec.md",
            "synth/fs-001-runtime-event-bridge",
            "Establishes the Rust-owned runtime status snapshot and event bridge, rendered by the thin React shell.",
            &[
                "Rust runtime status snapshot and event emission",
                "Thin React shell renders the status",
                "Typed IPC contract serialized in camelCase",
            ],
            &[
                "No workspace, provider, or persistence access",
                "Status is a static bootstrap snapshot",
            ],
        ),
        spec_detail(
            "FS-002",
            "Command dock parsing and intent routing",
            "Draft for review",
            "docs/specs/FS-002/spec.md",
            "synth/fs-002-command-dock-parsing",
            "Teaches the Rust core to classify raw command-dock input into typed command intents.",
            &[
                "parse_command classifies / ? @ # ! > and natural input",
                "Interactive command dock with a transient log",
            ],
            &[
                "Parsing only; intents do not route anywhere yet",
                "Shell commands require approval and never execute",
            ],
        ),
        spec_detail(
            "FS-003",
            "Slash command navigation routing",
            "Draft for review",
            "docs/specs/FS-003/spec.md",
            "synth/fs-003-slash-command-navigation",
            "Routes parsed slash-navigation commands to existing in-memory shell sections.",
            &[
                "route_command returns a typed routing disposition",
                "Handled slash navigation scrolls to shell sections",
            ],
            &[
                "In-memory UI navigation only",
                "No documents, providers, persistence, or shell execution",
            ],
        ),
        spec_detail(
            "FS-004",
            "Specs index reader shell",
            "Draft for review",
            "docs/specs/FS-004/spec.md",
            "synth/fs-004-specs-index-view",
            "Adds a static, Rust-owned specs index and routes /specs to it as a focused artifact section.",
            &[
                "list_specs_index exposes the static catalog",
                "/specs routes to the specs index section",
            ],
            &[
                "Static catalog, not a workspace document reader",
                "No directory scanning or markdown parsing",
            ],
        ),
        spec_detail(
            "FS-005",
            "Static spec detail selection",
            "Draft for review",
            "docs/specs/FS-005/spec.md",
            "synth/fs-005-static-spec-detail-selection",
            "Lets a known feature spec become the active visible detail artifact from the index or a /specs/<id> command.",
            &[
                "get_static_spec_detail returns a static detail snapshot",
                "/specs/<spec-id> routes to the spec-detail section",
                "Accessible select control per specs-index entry",
            ],
            &[
                "Static detail summaries, not full markdown bodies",
                "No runtime document reading, editing, or persistence",
            ],
        ),
    ]
}

pub fn static_specs_index() -> SpecsIndex {
    SpecsIndex {
        artifact_type: "specs-index".to_string(),
        generated_from: "static-rust-catalog".to_string(),
        specs: static_spec_details()
            .into_iter()
            .map(|detail| SpecIndexEntry {
                spec_id: detail.spec_id,
                title: detail.title,
                status: detail.status,
                path: detail.path,
                implementation_branch: detail.implementation_branch,
                route: detail.route,
            })
            .collect(),
        summary: "Static specs index generated from the Rust catalog. Workspace document reading arrives in a later spec.".to_string(),
    }
}

pub fn lookup_static_spec_detail(spec_id: &str) -> Result<StaticSpecDetail, String> {
    let normalized = spec_id.trim().to_ascii_uppercase();

    static_spec_details()
        .into_iter()
        .find(|detail| detail.spec_id == normalized)
        .ok_or_else(|| {
            format!(
                "No static spec detail for '{}'. Known specs are FS-001 through FS-005.",
                spec_id.trim()
            )
        })
}

fn spec_detail(
    spec_id: &str,
    title: &str,
    status: &str,
    path: &str,
    implementation_branch: &str,
    summary: &str,
    scope: &[&str],
    limitations: &[&str],
) -> StaticSpecDetail {
    StaticSpecDetail {
        spec_id: spec_id.to_string(),
        title: title.to_string(),
        status: status.to_string(),
        path: path.to_string(),
        implementation_branch: implementation_branch.to_string(),
        route: format!("/specs/{spec_id}"),
        summary: summary.to_string(),
        scope: scope.iter().map(|item| item.to_string()).collect(),
        limitations: limitations.iter().map(|item| item.to_string()).collect(),
    }
}

#[tauri::command]
pub fn list_specs_index() -> SpecsIndex {
    static_specs_index()
}

#[tauri::command]
pub fn get_static_spec_detail(spec_id: String) -> Result<StaticSpecDetail, String> {
    lookup_static_spec_detail(&spec_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn static_specs_index_contains_known_specs_in_order() {
        let index = static_specs_index();
        let ids: Vec<&str> = index
            .specs
            .iter()
            .map(|spec| spec.spec_id.as_str())
            .collect();

        assert_eq!(ids, vec!["FS-001", "FS-002", "FS-003", "FS-004", "FS-005"]);
    }

    #[test]
    fn static_specs_index_uses_static_catalog_metadata() {
        let index = static_specs_index();

        assert_eq!(index.artifact_type, "specs-index");
        assert_eq!(index.generated_from, "static-rust-catalog");
        assert!(index.summary.contains("Static specs index"));
        assert!(index
            .summary
            .contains("Workspace document reading arrives in a later spec"));
    }

    #[test]
    fn static_specs_index_entries_include_required_metadata() {
        let index = static_specs_index();
        let fs_004 = index
            .specs
            .iter()
            .find(|spec| spec.spec_id == "FS-004")
            .expect("FS-004 should be present in the static specs index");

        assert_eq!(fs_004.title, "Specs index reader shell");
        assert_eq!(fs_004.status, "Draft for review");
        assert_eq!(fs_004.path, "docs/specs/FS-004/spec.md");
        assert_eq!(
            fs_004.implementation_branch,
            "synth/fs-004-specs-index-view"
        );
        assert_eq!(fs_004.route, "/specs/FS-004");
    }

    #[test]
    fn index_entries_match_their_detail_source() {
        let index = static_specs_index();

        for entry in &index.specs {
            let detail = lookup_static_spec_detail(&entry.spec_id)
                .expect("every index entry must have a backing detail");

            assert_eq!(entry.spec_id, detail.spec_id);
            assert_eq!(entry.title, detail.title);
            assert_eq!(entry.status, detail.status);
            assert_eq!(entry.path, detail.path);
            assert_eq!(entry.implementation_branch, detail.implementation_branch);
            assert_eq!(entry.route, detail.route);
        }
    }

    #[test]
    fn looks_up_known_spec_detail_with_required_fields() {
        let detail = lookup_static_spec_detail("FS-001").expect("FS-001 is a known spec");

        assert_eq!(detail.spec_id, "FS-001");
        assert_eq!(detail.title, "Runtime event bridge and Synth shell");
        assert_eq!(detail.status, "Draft for review");
        assert_eq!(detail.path, "docs/specs/FS-001/spec.md");
        assert_eq!(
            detail.implementation_branch,
            "synth/fs-001-runtime-event-bridge"
        );
        assert_eq!(detail.route, "/specs/FS-001");
        assert!(!detail.summary.is_empty());
        assert!(!detail.scope.is_empty());
        assert!(!detail.limitations.is_empty());
    }

    #[test]
    fn detail_lookup_supports_every_catalog_spec() {
        for id in ["FS-001", "FS-002", "FS-003", "FS-004", "FS-005"] {
            let detail = lookup_static_spec_detail(id)
                .unwrap_or_else(|_| panic!("{id} should be a known static spec detail"));
            assert_eq!(detail.spec_id, id);
        }
    }

    #[test]
    fn detail_lookup_is_case_insensitive_and_normalizes_to_canonical_id() {
        let detail = lookup_static_spec_detail("  fs-002  ").expect("fs-002 is a known spec");

        assert_eq!(detail.spec_id, "FS-002");
    }

    #[test]
    fn detail_lookup_returns_readable_error_for_unknown_spec() {
        let error = lookup_static_spec_detail("FS-999").expect_err("FS-999 is unknown");

        assert!(error.contains("FS-999"));
        assert!(error.contains("Known specs"));
    }

    #[test]
    fn serializes_specs_index_for_the_react_ipc_contract_in_camel_case() {
        let serialized = serde_json::to_value(static_specs_index()).unwrap();

        assert_eq!(serialized["artifactType"], json!("specs-index"));
        assert_eq!(serialized["generatedFrom"], json!("static-rust-catalog"));
        assert_eq!(serialized["specs"][0]["specId"], json!("FS-001"));
        assert_eq!(
            serialized["specs"][0]["implementationBranch"],
            json!("synth/fs-001-runtime-event-bridge")
        );
        assert_eq!(serialized["specs"][0]["route"], json!("/specs/FS-001"));
    }

    #[test]
    fn serializes_spec_detail_for_the_react_ipc_contract_in_camel_case() {
        let serialized =
            serde_json::to_value(lookup_static_spec_detail("FS-005").unwrap()).unwrap();

        assert_eq!(serialized["specId"], json!("FS-005"));
        assert_eq!(serialized["title"], json!("Static spec detail selection"));
        assert_eq!(serialized["status"], json!("Draft for review"));
        assert_eq!(serialized["path"], json!("docs/specs/FS-005/spec.md"));
        assert_eq!(
            serialized["implementationBranch"],
            json!("synth/fs-005-static-spec-detail-selection")
        );
        assert_eq!(serialized["route"], json!("/specs/FS-005"));
        assert!(serialized["summary"].is_string());
        assert!(serialized["scope"].is_array());
        assert!(serialized["limitations"].is_array());
    }
}
