use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::events::EventRecord;
use crate::extensions::Extension;
use crate::workflows::Workflow;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub events: Vec<EventRecord>,
    pub extensions: Vec<Extension>,
    pub workflows: Vec<Workflow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub extensions: usize,
    pub workflows: usize,
}

pub fn parse_bundle(text: &str) -> Result<ExportBundle, String> {
    serde_json::from_str(text).map_err(|error| format!("Invalid export bundle: {error}"))
}

pub fn build_bundle(
    events: Vec<EventRecord>,
    extensions: Vec<Extension>,
    workflows: Vec<Workflow>,
) -> ExportBundle {
    ExportBundle {
        events,
        extensions,
        workflows,
    }
}

#[tauri::command]
pub fn export_state(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))?;

    let bundle = build_bundle(
        crate::events::load_records(&crate::events::events_path(&app)?, usize::MAX),
        crate::extensions::load_registry(&crate::extensions::registry_path(&app)?),
        crate::workflows::load_store(&crate::workflows::store_path(&app)?),
    );

    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    let target = dir.join("synth-export.json");
    let content = serde_json::to_string_pretty(&bundle)
        .map_err(|error| format!("Cannot serialize export: {error}"))?;
    std::fs::write(&target, content).map_err(|error| format!("Cannot write export: {error}"))?;
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_state(app: tauri::AppHandle) -> Result<ImportSummary, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))?;
    let source = dir.join("synth-export.json");
    let text = std::fs::read_to_string(&source)
        .map_err(|_| "No export bundle to import.".to_string())?;
    let bundle = parse_bundle(&text)?;

    crate::extensions::save_registry(&crate::extensions::registry_path(&app)?, &bundle.extensions)?;
    crate::workflows::save_store(&crate::workflows::store_path(&app)?, &bundle.workflows)?;

    Ok(ImportSummary {
        extensions: bundle.extensions.len(),
        workflows: bundle.workflows.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_bundle_and_rejects_malformed() {
        let bundle = build_bundle(vec![], vec![], vec![]);
        let text = serde_json::to_string(&bundle).unwrap();
        assert_eq!(parse_bundle(&text).unwrap(), bundle);
        assert!(parse_bundle("not json").is_err());
        assert!(parse_bundle("{}").is_err());
    }

    #[test]
    fn builds_bundle_with_all_sections_in_camel_case() {
        let bundle = build_bundle(
            vec![EventRecord {
                id: 0,
                kind: "command".to_string(),
                label: "x".to_string(),
                detail: "y".to_string(),
            }],
            vec![Extension {
                id: 0,
                name: "rg".to_string(),
                kind: "tool".to_string(),
                command: "rg --version".to_string(),
                scope: "read".to_string(),
            }],
            vec![Workflow {
                id: 0,
                name: "verify".to_string(),
                steps: vec!["cargo test".to_string()],
            }],
        );

        let value = serde_json::to_value(&bundle).unwrap();
        assert_eq!(value["events"].as_array().unwrap().len(), 1);
        assert_eq!(value["extensions"].as_array().unwrap().len(), 1);
        assert_eq!(value["workflows"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn empty_inputs_yield_empty_sections() {
        let value = serde_json::to_value(build_bundle(vec![], vec![], vec![])).unwrap();
        assert!(value["events"].as_array().unwrap().is_empty());
        assert!(value["extensions"].as_array().unwrap().is_empty());
        assert!(value["workflows"].as_array().unwrap().is_empty());
    }
}
