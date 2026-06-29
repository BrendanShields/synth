use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub steps: Vec<String>,
}

pub fn validate_workflow(name: &str, steps: &[String]) -> Result<(), String> {
    if name.is_empty() || name.len() > 200 {
        return Err("Invalid workflow name.".to_string());
    }
    if steps.is_empty() || steps.len() > 50 {
        return Err("A workflow needs between 1 and 50 steps.".to_string());
    }
    if steps.iter().any(|s| s.trim().is_empty() || s.len() > 2000) {
        return Err("Every step must be a non-empty command.".to_string());
    }
    Ok(())
}

pub fn load_store(path: &Path) -> Vec<Workflow> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_store(path: &Path, workflows: &[Workflow]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    }
    let content = serde_json::to_string_pretty(workflows)
        .map_err(|error| format!("Cannot serialize workflows: {error}"))?;
    std::fs::write(path, content).map_err(|error| format!("Cannot write workflows: {error}"))
}

fn store_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("workflows.json"))
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))
}

#[tauri::command]
pub fn save_workflow(
    app: tauri::AppHandle,
    name: String,
    steps: Vec<String>,
) -> Result<Workflow, String> {
    let name = name.trim().to_string();
    let steps: Vec<String> = steps.into_iter().map(|s| s.trim().to_string()).collect();
    validate_workflow(&name, &steps)?;

    let path = store_path(&app)?;
    let mut workflows = load_store(&path);
    let id = workflows.iter().map(|w| w.id).max().map_or(0, |max| max + 1);
    let workflow = Workflow { id, name, steps };
    workflows.push(workflow.clone());
    save_store(&path, &workflows)?;
    Ok(workflow)
}

#[tauri::command]
pub fn list_workflows(app: tauri::AppHandle) -> Vec<Workflow> {
    match store_path(&app) {
        Ok(path) => load_store(&path),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub fn remove_workflow(app: tauri::AppHandle, id: u64) -> Result<(), String> {
    let path = store_path(&app)?;
    let mut workflows = load_store(&path);
    let before = workflows.len();
    workflows.retain(|w| w.id != id);
    if workflows.len() == before {
        return Err("Unknown workflow.".to_string());
    }
    save_store(&path, &workflows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("synth-fs038-{tag}-{}.json", std::process::id()))
    }

    fn steps(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn validates_workflow() {
        assert!(validate_workflow("verify", &steps(&["bun run test"])).is_ok());
        assert!(validate_workflow("", &steps(&["x"])).is_err());
        assert!(validate_workflow("v", &[]).is_err());
        assert!(validate_workflow("v", &steps(&["ok", "   "])).is_err());
    }

    #[test]
    fn missing_or_malformed_store_loads_empty() {
        assert!(load_store(Path::new("/no/such/synth/workflows.json")).is_empty());
        let path = temp_path("malformed");
        std::fs::write(&path, "not json").unwrap();
        assert!(load_store(&path).is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn round_trips_save_and_remove_preserving_step_order() {
        let path = temp_path("crud");
        let _ = std::fs::remove_file(&path);

        let workflow = Workflow {
            id: 0,
            name: "verify".to_string(),
            steps: steps(&["bun run test", "cargo test"]),
        };
        save_store(&path, &[workflow]).unwrap();

        let loaded = load_store(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].steps, steps(&["bun run test", "cargo test"]));

        let kept: Vec<Workflow> = loaded.into_iter().filter(|w| w.id != 0).collect();
        save_store(&path, &kept).unwrap();
        assert!(load_store(&path).is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn serializes_workflow_in_camel_case() {
        let serialized = serde_json::to_value(Workflow {
            id: 1,
            name: "verify".to_string(),
            steps: steps(&["cargo test"]),
        })
        .unwrap();
        assert_eq!(serialized["id"], 1);
        assert_eq!(serialized["steps"][0], "cargo test");
    }
}
