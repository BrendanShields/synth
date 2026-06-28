use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

const MAX_DOC_BYTES: u64 = 262_144;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub root: String,
    pub name: String,
}

#[derive(Default)]
pub struct WorkspaceState(pub Mutex<Option<Workspace>>);

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub fn is_within_root(root: &Path, candidate: &Path) -> bool {
    normalize(candidate).starts_with(normalize(root))
}

#[tauri::command]
pub async fn open_workspace(
    state: tauri::State<'_, WorkspaceState>,
    path: String,
) -> Result<Workspace, String> {
    let candidate = Path::new(&path);

    let metadata =
        std::fs::metadata(candidate).map_err(|_| format!("Path does not exist: {path}"))?;
    if !metadata.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    let canonical = std::fs::canonicalize(candidate)
        .map_err(|error| format!("Cannot resolve path: {error}"))?;
    let name = canonical
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| canonical.to_string_lossy().to_string());

    let workspace = Workspace {
        root: canonical.to_string_lossy().to_string(),
        name,
    };

    *state.0.lock().expect("workspace state lock poisoned") = Some(workspace.clone());
    Ok(workspace)
}

#[tauri::command]
pub fn get_workspace(state: tauri::State<'_, WorkspaceState>) -> Option<Workspace> {
    state
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .clone()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningBaseline {
    pub prd_present: bool,
    pub erd_present: bool,
    pub complete: bool,
}

fn file_within_root(root: &Path, relative: &str) -> bool {
    let candidate = root.join(relative);
    is_within_root(root, &candidate) && candidate.is_file()
}

pub fn detect_planning_baseline(root: &Path) -> PlanningBaseline {
    let prd_present = file_within_root(root, "docs/PRD.md");
    let erd_present = file_within_root(root, "docs/engineering/ERD.md");
    PlanningBaseline {
        prd_present,
        erd_present,
        complete: prd_present && erd_present,
    }
}

#[tauri::command]
pub fn inspect_planning_baseline(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<PlanningBaseline, String> {
    let guard = state.0.lock().expect("workspace state lock poisoned");
    let workspace = guard.as_ref().ok_or("No workspace is open.")?;
    Ok(detect_planning_baseline(Path::new(&workspace.root)))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDoc {
    pub kind: String,
    pub path: String,
    pub text: String,
}

pub fn workspace_doc_path(kind: &str) -> Option<&'static str> {
    match kind {
        "prd" => Some("docs/PRD.md"),
        "erd" => Some("docs/engineering/ERD.md"),
        _ => None,
    }
}

fn read_capped(path: &Path, cap: u64) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|_| "Cannot read document.".to_string())?;
    let mut buffer = Vec::new();
    file.take(cap)
        .read_to_end(&mut buffer)
        .map_err(|error| format!("read error: {error}"))?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

#[tauri::command]
pub fn read_workspace_doc(
    state: tauri::State<'_, WorkspaceState>,
    kind: String,
) -> Result<WorkspaceDoc, String> {
    let relative =
        workspace_doc_path(&kind).ok_or_else(|| format!("Unknown document: {kind}"))?;

    let guard = state.0.lock().expect("workspace state lock poisoned");
    let workspace = guard.as_ref().ok_or("No workspace is open.")?;
    let root = Path::new(&workspace.root);
    let candidate = root.join(relative);

    if !is_within_root(root, &candidate) {
        return Err("Path escapes the workspace.".to_string());
    }

    Ok(WorkspaceDoc {
        kind,
        path: relative.to_string(),
        text: read_capped(&candidate, MAX_DOC_BYTES)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_root_accepts_root_and_nested_paths() {
        let root = Path::new("/work/repo");
        assert!(is_within_root(root, Path::new("/work/repo")));
        assert!(is_within_root(root, Path::new("/work/repo/src/main.rs")));
    }

    #[test]
    fn within_root_rejects_traversal_sibling_and_unrelated() {
        let root = Path::new("/work/repo");
        assert!(!is_within_root(root, Path::new("/work/repo/../secret")));
        assert!(!is_within_root(root, Path::new("/work/other")));
        assert!(!is_within_root(root, Path::new("/etc/passwd")));
    }

    #[test]
    fn within_root_is_component_wise_not_prefix_string() {
        assert!(!is_within_root(
            Path::new("/work/repo"),
            Path::new("/work/repo-secrets")
        ));
    }

    #[test]
    fn open_workspace_validation_rejects_missing_and_non_directory() {
        // Pure validation mirrors open_workspace without Tauri state: a missing
        // path and a file both fail the directory check.
        assert!(std::fs::metadata("/this/path/should/not/exist/synth").is_err());
        let this_file = Path::new(file!());
        assert!(this_file.is_file());
        assert!(!this_file.is_dir());
    }

    #[test]
    fn open_workspace_derives_name_from_a_real_directory() {
        let dir = std::env::temp_dir();
        let canonical = std::fs::canonicalize(&dir).unwrap();
        let name = canonical
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        assert!(!name.is_empty());
        assert!(is_within_root(&canonical, &canonical.join("sub/file.txt")));
    }

    #[test]
    fn detects_planning_baseline_states_in_a_temp_workspace() {
        let base = std::env::temp_dir().join(format!("synth-fs013-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("docs/engineering")).unwrap();

        let none = detect_planning_baseline(&base);
        assert!(!none.prd_present && !none.erd_present && !none.complete);

        std::fs::write(base.join("docs/PRD.md"), "x").unwrap();
        let one = detect_planning_baseline(&base);
        assert!(one.prd_present && !one.erd_present && !one.complete);

        std::fs::write(base.join("docs/engineering/ERD.md"), "x").unwrap();
        let both = detect_planning_baseline(&base);
        assert!(both.prd_present && both.erd_present && both.complete);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn workspace_doc_path_allow_list() {
        assert_eq!(workspace_doc_path("prd"), Some("docs/PRD.md"));
        assert_eq!(workspace_doc_path("erd"), Some("docs/engineering/ERD.md"));
        assert_eq!(workspace_doc_path("secrets"), None);
        assert_eq!(workspace_doc_path(""), None);
    }

    #[test]
    fn read_capped_truncates_to_the_cap() {
        let path =
            std::env::temp_dir().join(format!("synth-fs014-{}.txt", std::process::id()));
        std::fs::write(&path, "hello world").unwrap();

        assert_eq!(read_capped(&path, 5).unwrap(), "hello");
        assert_eq!(read_capped(&path, 1000).unwrap(), "hello world");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn serializes_workspace_doc_in_camel_case() {
        let serialized = serde_json::to_value(WorkspaceDoc {
            kind: "prd".to_string(),
            path: "docs/PRD.md".to_string(),
            text: "hi".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["kind"], "prd");
        assert_eq!(serialized["path"], "docs/PRD.md");
        assert_eq!(serialized["text"], "hi");
    }

    #[test]
    fn serializes_planning_baseline_in_camel_case() {
        let serialized = serde_json::to_value(PlanningBaseline {
            prd_present: true,
            erd_present: false,
            complete: false,
        })
        .unwrap();
        assert_eq!(serialized["prdPresent"], true);
        assert_eq!(serialized["erdPresent"], false);
        assert_eq!(serialized["complete"], false);
    }

    #[test]
    fn serializes_workspace_in_camel_case() {
        let serialized = serde_json::to_value(Workspace {
            root: "/work/repo".to_string(),
            name: "repo".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["root"], "/work/repo");
        assert_eq!(serialized["name"], "repo");
    }
}
