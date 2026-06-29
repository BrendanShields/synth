use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subagent {
    pub id: u64,
    pub name: String,
    pub role: String,
    pub instructions: String,
}

pub fn validate_subagent(name: &str, role: &str, instructions: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 200 {
        return Err("Invalid subagent name.".to_string());
    }
    if !crate::roles::is_valid_role(role) {
        return Err("Unknown role.".to_string());
    }
    if instructions.is_empty() || instructions.len() > 8000 {
        return Err("Invalid subagent instructions.".to_string());
    }
    Ok(())
}

pub fn build_subagent_prompt(instructions: &str, input: &str) -> String {
    format!("{instructions}\n\nInput:\n{input}\n\nResponse:")
}

pub fn load_store(path: &Path) -> Vec<Subagent> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_store(path: &Path, subagents: &[Subagent]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    }
    let content = serde_json::to_string_pretty(subagents)
        .map_err(|error| format!("Cannot serialize subagents: {error}"))?;
    std::fs::write(path, content).map_err(|error| format!("Cannot write subagents: {error}"))
}

pub fn store_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("subagents.json"))
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))
}

#[tauri::command]
pub fn save_subagent(
    app: tauri::AppHandle,
    name: String,
    role: String,
    instructions: String,
) -> Result<Subagent, String> {
    let name = name.trim().to_string();
    let instructions = instructions.trim().to_string();
    validate_subagent(&name, &role, &instructions)?;

    let path = store_path(&app)?;
    let mut subagents = load_store(&path);
    let id = subagents.iter().map(|s| s.id).max().map_or(0, |max| max + 1);
    let subagent = Subagent {
        id,
        name,
        role,
        instructions,
    };
    subagents.push(subagent.clone());
    save_store(&path, &subagents)?;
    Ok(subagent)
}

#[tauri::command]
pub fn list_subagents(app: tauri::AppHandle) -> Vec<Subagent> {
    match store_path(&app) {
        Ok(path) => load_store(&path),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub fn remove_subagent(app: tauri::AppHandle, id: u64) -> Result<(), String> {
    let path = store_path(&app)?;
    let mut subagents = load_store(&path);
    let before = subagents.len();
    subagents.retain(|s| s.id != id);
    if subagents.len() == before {
        return Err("Unknown subagent.".to_string());
    }
    save_store(&path, &subagents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("synth-fs049-{tag}-{}.json", std::process::id()))
    }

    #[test]
    fn validates_fields_including_role() {
        assert!(validate_subagent("Reviewer", "adversary", "Critique it.").is_ok());
        assert!(validate_subagent("", "adversary", "x").is_err());
        assert!(validate_subagent("R", "nonsense-role", "x").is_err());
        assert!(validate_subagent("R", "adversary", "").is_err());
    }

    #[test]
    fn prompt_includes_instructions_and_input() {
        let prompt = build_subagent_prompt("Be terse.", "summarize this");
        assert!(prompt.contains("Be terse."));
        assert!(prompt.contains("summarize this"));
    }

    #[test]
    fn round_trips_save_and_remove() {
        let path = temp_path("crud");
        let _ = std::fs::remove_file(&path);

        save_store(
            &path,
            &[Subagent {
                id: 0,
                name: "Reviewer".to_string(),
                role: "adversary".to_string(),
                instructions: "Critique it.".to_string(),
            }],
        )
        .unwrap();
        assert_eq!(load_store(&path).len(), 1);

        let kept: Vec<Subagent> = load_store(&path).into_iter().filter(|s| s.id != 0).collect();
        save_store(&path, &kept).unwrap();
        assert!(load_store(&path).is_empty());
        assert!(load_store(Path::new("/no/such/synth/subagents.json")).is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn serializes_subagent_in_camel_case() {
        let value = serde_json::to_value(Subagent {
            id: 1,
            name: "Reviewer".to_string(),
            role: "adversary".to_string(),
            instructions: "Critique it.".to_string(),
        })
        .unwrap();
        assert_eq!(value["id"], 1);
        assert_eq!(value["role"], "adversary");
    }
}
