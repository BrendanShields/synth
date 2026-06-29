use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub kind: String,
    pub label: String,
    pub detail: String,
}

pub fn serialize_node(node: &SessionNode) -> String {
    serde_json::to_string(node).unwrap_or_default()
}

pub fn parse_node(line: &str) -> Option<SessionNode> {
    serde_json::from_str(line.trim()).ok()
}

pub fn append_node(path: &Path, node: &SessionNode) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Cannot open session tree file: {error}"))?;
    writeln!(file, "{}", serialize_node(node))
        .map_err(|error| format!("Cannot write session node: {error}"))?;
    Ok(())
}

pub fn load_nodes(path: &Path) -> Vec<SessionNode> {
    match std::fs::read_to_string(path) {
        Ok(content) => content.lines().filter_map(parse_node).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn path_to_root(nodes: &[SessionNode], id: u64) -> Vec<SessionNode> {
    let mut chain = Vec::new();
    let mut current = nodes.iter().find(|node| node.id == id);
    let limit = nodes.len();

    while let Some(node) = current {
        chain.push(node.clone());
        if chain.len() > limit {
            break; // guard against a cycle in a broken store
        }
        current = match node.parent_id {
            Some(parent) => nodes.iter().find(|candidate| candidate.id == parent),
            None => None,
        };
    }

    chain.reverse();
    chain
}

fn tree_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("session-tree.jsonl"))
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))
}

#[tauri::command]
pub fn append_session_node(
    app: tauri::AppHandle,
    parent_id: Option<u64>,
    kind: String,
    label: String,
    detail: String,
) -> Result<SessionNode, String> {
    let path = tree_path(&app)?;
    let nodes = load_nodes(&path);

    if let Some(parent) = parent_id {
        if !nodes.iter().any(|node| node.id == parent) {
            return Err("Unknown parent node.".to_string());
        }
    }

    let node = SessionNode {
        id: nodes.len() as u64,
        parent_id,
        kind,
        label,
        detail,
    };
    append_node(&path, &node)?;
    Ok(node)
}

#[tauri::command]
pub fn load_session_tree(app: tauri::AppHandle) -> Vec<SessionNode> {
    match tree_path(&app) {
        Ok(path) => load_nodes(&path),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub fn replay_path(app: tauri::AppHandle, node_id: u64) -> Vec<SessionNode> {
    let nodes = match tree_path(&app) {
        Ok(path) => load_nodes(&path),
        Err(_) => return Vec::new(),
    };
    path_to_root(&nodes, node_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u64, parent: Option<u64>) -> SessionNode {
        SessionNode {
            id,
            parent_id: parent,
            kind: "step".to_string(),
            label: "x".to_string(),
            detail: format!("n{id}"),
        }
    }

    #[test]
    fn serializes_parent_id_in_camel_case() {
        let value = serde_json::to_value(node(1, Some(0))).unwrap();
        assert_eq!(value["id"], 1);
        assert_eq!(value["parentId"], 0);
        let root = serde_json::to_value(node(0, None)).unwrap();
        assert!(root["parentId"].is_null());
    }

    #[test]
    fn appends_and_loads_in_order_and_skips_malformed() {
        let path = std::env::temp_dir().join(format!("synth-fs042-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);

        append_node(&path, &node(0, None)).unwrap();
        append_node(&path, &node(1, Some(0))).unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"garbage\n")
            .unwrap();

        let loaded = load_nodes(&path);
        assert_eq!(loaded.iter().map(|n| n.id).collect::<Vec<_>>(), vec![0, 1]);
        assert!(load_nodes(Path::new("/no/such/synth/session-tree.jsonl")).is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn path_to_root_returns_root_to_leaf_and_is_robust() {
        let nodes = vec![node(0, None), node(1, Some(0)), node(2, Some(1))];
        let path = path_to_root(&nodes, 2);
        assert_eq!(path.iter().map(|n| n.id).collect::<Vec<_>>(), vec![0, 1, 2]);

        assert!(path_to_root(&nodes, 99).is_empty());

        // a broken parent link terminates without looping
        let broken = vec![node(5, Some(404))];
        assert_eq!(
            path_to_root(&broken, 5).iter().map(|n| n.id).collect::<Vec<_>>(),
            vec![5]
        );
    }
}
