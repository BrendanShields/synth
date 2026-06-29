use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

fn default_scope() -> String {
    "read".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub command: String,
    #[serde(default = "default_scope")]
    pub scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRunRecord {
    pub id: u64,
    pub extension_id: u64,
    pub name: String,
    pub kind: String,
    pub scope: String,
    pub command: String,
    pub status: String,
    pub detail: String,
}

pub fn is_valid_extension_kind(kind: &str) -> bool {
    matches!(kind, "tool" | "mcp" | "skill")
}

pub fn is_valid_extension_scope(scope: &str) -> bool {
    matches!(scope, "read" | "write" | "network" | "shell")
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

pub fn extension_runs_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("extension-runs.jsonl"))
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))
}

pub fn build_run_record(
    id: u64,
    extension: &Extension,
    status: &str,
    detail: &str,
) -> ExtensionRunRecord {
    ExtensionRunRecord {
        id,
        extension_id: extension.id,
        name: extension.name.clone(),
        kind: extension.kind.clone(),
        scope: extension.scope.clone(),
        command: extension.command.clone(),
        status: status.to_string(),
        detail: detail.to_string(),
    }
}

pub fn serialize_run_record(record: &ExtensionRunRecord) -> String {
    serde_json::to_string(record).unwrap_or_default()
}

pub fn parse_run_record(line: &str) -> Option<ExtensionRunRecord> {
    serde_json::from_str(line.trim()).ok()
}

pub fn load_run_records(path: &Path, limit: usize) -> Vec<ExtensionRunRecord> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    let mut records: Vec<ExtensionRunRecord> =
        content.lines().filter_map(parse_run_record).collect();
    records.reverse();
    records.truncate(limit);
    records
}

pub fn append_run_record(
    path: &Path,
    extension: &Extension,
    status: &str,
    detail: &str,
) -> Result<ExtensionRunRecord, String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    }

    let id = load_run_records(path, usize::MAX).len() as u64;
    let record = build_run_record(id, extension, status, detail);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Cannot open extension runs file: {error}"))?;
    writeln!(file, "{}", serialize_run_record(&record))
        .map_err(|error| format!("Cannot write extension run: {error}"))?;
    Ok(record)
}

#[tauri::command]
pub fn list_extension_runs(app: tauri::AppHandle, limit: u32) -> Vec<ExtensionRunRecord> {
    match extension_runs_path(&app) {
        Ok(path) => load_run_records(&path, limit as usize),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub fn register_extension(
    app: tauri::AppHandle,
    name: String,
    kind: String,
    command: String,
    scope: String,
) -> Result<Extension, String> {
    let name = name.trim();
    let command = command.trim();
    if name.is_empty() || name.len() > 200 {
        return Err("Invalid extension name.".to_string());
    }
    if !is_valid_extension_kind(&kind) {
        return Err("Extension kind must be tool, mcp, or skill.".to_string());
    }
    if !is_valid_extension_scope(&scope) {
        return Err("Extension scope must be read, write, network, or shell.".to_string());
    }
    if command.is_empty() || command.len() > 2000 {
        return Err("Invalid extension command.".to_string());
    }

    let path = registry_path(&app)?;
    let mut extensions = load_registry(&path);
    let id = extensions
        .iter()
        .map(|e| e.id)
        .max()
        .map_or(0, |max| max + 1);
    let extension = Extension {
        id,
        name: name.to_string(),
        kind,
        command: command.to_string(),
        scope,
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

    fn extension() -> Extension {
        Extension {
            id: 7,
            name: "ripgrep".to_string(),
            kind: "tool".to_string(),
            command: "rg --version".to_string(),
            scope: "shell".to_string(),
        }
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
    fn validates_scope() {
        for scope in ["read", "write", "network", "shell"] {
            assert!(is_valid_extension_scope(scope));
        }
        assert!(!is_valid_extension_scope("admin"));
        assert!(!is_valid_extension_scope(""));
    }

    #[test]
    fn scope_defaults_to_read_for_pre_scope_registries() {
        let path = temp_path("legacy");
        std::fs::write(
            &path,
            r#"[{"id":0,"name":"rg","kind":"tool","command":"rg"}]"#,
        )
        .unwrap();
        let loaded = load_registry(&path);
        assert_eq!(loaded[0].scope, "read");
        let _ = std::fs::remove_file(&path);
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
            scope: "read".to_string(),
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
            scope: "shell".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["id"], 2);
        assert_eq!(serialized["kind"], "tool");
        assert_eq!(serialized["command"], "cargo fmt");
        assert_eq!(serialized["scope"], "shell");
    }

    #[test]
    fn serializes_extension_run_record_in_camel_case() {
        let record = build_run_record(3, &extension(), "requested", "Approval requested.");
        let serialized = serde_json::to_value(record).unwrap();
        assert_eq!(serialized["id"], 3);
        assert_eq!(serialized["extensionId"], 7);
        assert_eq!(serialized["name"], "ripgrep");
        assert_eq!(serialized["scope"], "shell");
        assert_eq!(serialized["status"], "requested");
    }

    #[test]
    fn parses_and_loads_extension_runs_newest_first_skipping_malformed() {
        let path = temp_path("runs");
        let _ = std::fs::remove_file(&path);
        let ext = extension();

        append_run_record(&path, &ext, "requested", "Approval requested.").unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"not json\n")
            .unwrap();
        append_run_record(&path, &ext, "denied", "Denied.").unwrap();
        append_run_record(&path, &ext, "succeeded", "ok").unwrap();
        append_run_record(&path, &ext, "failed", "boom").unwrap();

        let records = load_run_records(&path, 10);
        assert_eq!(records.len(), 4);
        assert_eq!(
            records
                .iter()
                .map(|record| record.status.as_str())
                .collect::<Vec<_>>(),
            vec!["failed", "succeeded", "denied", "requested"]
        );
        assert_eq!(load_run_records(&path, 1).len(), 1);
        assert!(load_run_records(Path::new("/no/such/synth/extension-runs.jsonl"), 5).is_empty());

        let _ = std::fs::remove_file(&path);
    }
}
