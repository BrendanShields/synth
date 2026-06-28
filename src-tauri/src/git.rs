use serde::Serialize;

use crate::workspace::WorkspaceState;

const MAX_CHANGES: usize = 200;
const MAX_LOG: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: String,
    pub clean: bool,
    pub changes: Vec<String>,
}

fn parse_branch_header(rest: &str) -> String {
    if rest.contains("(no branch)") {
        return String::new();
    }
    if let Some(stripped) = rest.strip_prefix("No commits yet on ") {
        return stripped.split_whitespace().next().unwrap_or("").to_string();
    }
    rest.split("...")
        .next()
        .unwrap_or("")
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

pub fn parse_status(porcelain: &str) -> GitStatus {
    let mut branch = String::new();
    let mut changes = Vec::new();

    for line in porcelain.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            branch = parse_branch_header(rest);
        } else if !line.trim_end().is_empty() && changes.len() < MAX_CHANGES {
            changes.push(line.to_string());
        }
    }

    GitStatus {
        is_repo: true,
        clean: changes.is_empty(),
        branch,
        changes,
    }
}

fn not_a_repo() -> GitStatus {
    GitStatus {
        is_repo: false,
        branch: String::new(),
        clean: true,
        changes: Vec::new(),
    }
}

#[tauri::command]
pub fn git_status(state: tauri::State<'_, WorkspaceState>) -> Result<GitStatus, String> {
    let root = {
        let guard = state.0.lock().expect("workspace state lock poisoned");
        guard.as_ref().ok_or("No workspace is open.")?.root.clone()
    };

    let output = std::process::Command::new("git")
        .current_dir(&root)
        .args(["status", "--porcelain=v1", "--branch"])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;

    if output.status.success() {
        return Ok(parse_status(&String::from_utf8_lossy(&output.stdout)));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.to_lowercase().contains("not a git repository") {
        Ok(not_a_repo())
    } else {
        Err(format!("git failed: {}", stderr.trim()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommit {
    pub short: String,
    pub subject: String,
}

pub fn parse_log(output: &str) -> Vec<GitCommit> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(MAX_LOG)
        .filter_map(|line| {
            line.split_once(' ').map(|(short, subject)| GitCommit {
                short: short.to_string(),
                subject: subject.to_string(),
            })
        })
        .collect()
}

#[tauri::command]
pub fn git_log(state: tauri::State<'_, WorkspaceState>) -> Result<Vec<GitCommit>, String> {
    let root = {
        let guard = state.0.lock().expect("workspace state lock poisoned");
        guard.as_ref().ok_or("No workspace is open.")?.root.clone()
    };

    let output = std::process::Command::new("git")
        .current_dir(&root)
        .args([
            "log",
            &format!("--max-count={MAX_LOG}"),
            "--pretty=format:%h %s",
        ])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;

    if output.status.success() {
        return Ok(parse_log(&String::from_utf8_lossy(&output.stdout)));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("not a git repository")
        || stderr.contains("does not have any commits")
        || stderr.contains("bad default revision")
    {
        Ok(Vec::new())
    } else {
        Err(format!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_repository() {
        let status = parse_status("## main...origin/main\n");
        assert!(status.is_repo);
        assert_eq!(status.branch, "main");
        assert!(status.clean);
        assert!(status.changes.is_empty());
    }

    #[test]
    fn parses_dirty_repository() {
        let status = parse_status("## feature/x\n M src/a.rs\n?? b.txt\n");
        assert_eq!(status.branch, "feature/x");
        assert!(!status.clean);
        assert_eq!(status.changes, vec![" M src/a.rs".to_string(), "?? b.txt".to_string()]);
    }

    #[test]
    fn parses_detached_head_and_no_commits() {
        assert_eq!(parse_status("## HEAD (no branch)\n").branch, "");
        assert_eq!(parse_status("## No commits yet on main\n").branch, "main");
    }

    #[test]
    fn caps_the_change_list() {
        let mut input = String::from("## main\n");
        for index in 0..(MAX_CHANGES + 50) {
            input.push_str(&format!("?? file{index}.txt\n"));
        }
        assert_eq!(parse_status(&input).changes.len(), MAX_CHANGES);
    }

    #[test]
    fn parses_log_lines_and_skips_malformed() {
        let commits = parse_log("abc1234 first commit\ndef5678 fix: bug in parser\n\nbadline\n");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].short, "abc1234");
        assert_eq!(commits[0].subject, "first commit");
        assert_eq!(commits[1].subject, "fix: bug in parser");
    }

    #[test]
    fn caps_the_log() {
        let mut input = String::new();
        for index in 0..(MAX_LOG + 10) {
            input.push_str(&format!("hash{index} subject {index}\n"));
        }
        assert_eq!(parse_log(&input).len(), MAX_LOG);
    }

    #[test]
    fn serializes_commit_in_camel_case() {
        let serialized = serde_json::to_value(GitCommit {
            short: "abc1234".to_string(),
            subject: "hello".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["short"], "abc1234");
        assert_eq!(serialized["subject"], "hello");
    }

    #[test]
    fn serializes_status_in_camel_case() {
        let serialized = serde_json::to_value(GitStatus {
            is_repo: true,
            branch: "main".to_string(),
            clean: false,
            changes: vec!["?? x".to_string()],
        })
        .unwrap();
        assert_eq!(serialized["isRepo"], true);
        assert_eq!(serialized["branch"], "main");
        assert_eq!(serialized["clean"], false);
        assert!(serialized["changes"].is_array());
    }
}
