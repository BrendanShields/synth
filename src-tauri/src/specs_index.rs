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

pub fn static_specs_index() -> SpecsIndex {
    SpecsIndex {
        artifact_type: "specs-index".to_string(),
        generated_from: "static-rust-catalog".to_string(),
        specs: vec![
            spec_entry(
                "FS-001",
                "Runtime event bridge and Synth shell",
                "Draft for review",
                "docs/specs/FS-001/spec.md",
                "synth/fs-001-runtime-event-bridge",
            ),
            spec_entry(
                "FS-002",
                "Command dock parsing and intent routing",
                "Draft for review",
                "docs/specs/FS-002/spec.md",
                "synth/fs-002-command-dock-parsing",
            ),
            spec_entry(
                "FS-003",
                "Slash command navigation routing",
                "Draft for review",
                "docs/specs/FS-003/spec.md",
                "synth/fs-003-slash-command-navigation",
            ),
            spec_entry(
                "FS-004",
                "Specs index reader shell",
                "Draft for review",
                "docs/specs/FS-004/spec.md",
                "synth/fs-004-specs-index-view",
            ),
        ],
        summary: "Static specs index generated from the Rust catalog. Workspace document reading arrives in a later spec.".to_string(),
    }
}

fn spec_entry(
    spec_id: &str,
    title: &str,
    status: &str,
    path: &str,
    implementation_branch: &str,
) -> SpecIndexEntry {
    SpecIndexEntry {
        spec_id: spec_id.to_string(),
        title: title.to_string(),
        status: status.to_string(),
        path: path.to_string(),
        implementation_branch: implementation_branch.to_string(),
        route: format!("/specs/{spec_id}"),
    }
}

#[tauri::command]
pub fn list_specs_index() -> SpecsIndex {
    static_specs_index()
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

        assert_eq!(ids, vec!["FS-001", "FS-002", "FS-003", "FS-004"]);
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
}
