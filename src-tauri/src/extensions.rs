use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub command: String,
}

pub fn is_valid_extension_kind(kind: &str) -> bool {
    matches!(kind, "tool" | "mcp" | "skill")
}

pub fn load_registry(path: &Path) -> Vec<Extension> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_registry(path: &Path, extensions: &[Extension]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    }
    let content = serde_json::to_string_pretty(extensions)
        .map_err(|error| format!("Cannot serialize registry: {error}"))?;
    std::fs::write(path, content).map_err(|error| format!("Cannot write registry: {error}"))
}

pub fn registry_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("extensions.json"))
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))
}

#[tauri::command]
pub fn register_extension(
    app: tauri::AppHandle,
    name: String,
    kind: String,
    command: String,
) -> Result<Extension, String> {
    let name = name.trim();
    let command = command.trim();
    if name.is_empty() || name.len() > 200 {
        return Err("Invalid extension name.".to_string());
    }
    if !is_valid_extension_kind(&kind) {
        return Err("Extension kind must be tool, mcp, or skill.".to_string());
    }
    if command.is_empty() || command.len() > 2000 {
        return Err("Invalid extension command.".to_string());
    }

    let path = registry_path(&app)?;
    let mut extensions = load_registry(&path);
    let id = extensions.iter().map(|e| e.id).max().map_or(0, |max| max + 1);
    let extension = Extension {
        id,
        name: name.to_string(),
        kind,
        command: command.to_string(),
    };
    extensions.push(extension.clone());
    save_registry(&path, &extensions)?;
    Ok(extension)
}

#[tauri::command]
pub fn list_extensions(app: tauri::AppHandle) -> Vec<Extension> {
    match registry_path(&app) {
        Ok(path) => load_registry(&path),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub fn remove_extension(app: tauri::AppHandle, id: u64) -> Result<(), String> {
    let path = registry_path(&app)?;
    let mut extensions = load_registry(&path);
    let before = extensions.len();
    extensions.retain(|e| e.id != id);
    if extensions.len() == before {
        return Err("Unknown extension.".to_string());
    }
    save_registry(&path, &extensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("synth-fs037-{tag}-{}.json", std::process::id()))
    }

    #[test]
    fn validates_kind() {
        assert!(is_valid_extension_kind("tool"));
        assert!(is_valid_extension_kind("mcp"));
        assert!(is_valid_extension_kind("skill"));
        assert!(!is_valid_extension_kind("plugin"));
        assert!(!is_valid_extension_kind(""));
    }

    #[test]
    fn missing_or_malformed_registry_loads_empty() {
        assert!(load_registry(Path::new("/no/such/synth/extensions.json")).is_empty());
        let path = temp_path("malformed");
        std::fs::write(&path, "not json").unwrap();
        assert!(load_registry(&path).is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn round_trips_register_and_remove() {
        let path = temp_path("crud");
        let _ = std::fs::remove_file(&path);

        let mut registry = load_registry(&path);
        assert!(registry.is_empty());

        registry.push(Extension {
            id: 0,
            name: "ripgrep".to_string(),
            kind: "tool".to_string(),
            command: "rg --version".to_string(),
        });
        save_registry(&path, &registry).unwrap();

        let loaded = load_registry(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "ripgrep");

        let kept: Vec<Extension> = loaded.into_iter().filter(|e| e.id != 0).collect();
        save_registry(&path, &kept).unwrap();
        assert!(load_registry(&path).is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn serializes_extension_in_camel_case() {
        let serialized = serde_json::to_value(Extension {
            id: 2,
            name: "fmt".to_string(),
            kind: "tool".to_string(),
            command: "cargo fmt".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["id"], 2);
        assert_eq!(serialized["kind"], "tool");
        assert_eq!(serialized["command"], "cargo fmt");
    }
}
