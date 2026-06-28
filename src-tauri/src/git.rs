use std::path::Path;

use serde::Serialize;

use crate::workspace::WorkspaceState;

const MAX_CHANGES: usize = 200;
const MAX_LOG: usize = 20;
const MAX_DIFF_LINES: usize = 2000;

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

pub fn create_branch(root: &Path, name: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["branch", name])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git branch failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiff {
    pub is_repo: bool,
    pub empty: bool,
    pub lines: Vec<DiffLine>,
}

pub fn parse_diff(diff: &str) -> Vec<DiffLine> {
    diff.lines()
        .take(MAX_DIFF_LINES)
        .map(|line| {
            let kind = if line.starts_with("+++")
                || line.starts_with("---")
                || line.starts_with("diff ")
                || line.starts_with("@@")
                || line.starts_with("index ")
            {
                "meta"
            } else if line.starts_with('+') {
                "add"
            } else if line.starts_with('-') {
                "del"
            } else {
                "context"
            };
            DiffLine {
                kind: kind.to_string(),
                text: line.to_string(),
            }
        })
        .collect()
}

#[tauri::command]
pub fn git_diff(state: tauri::State<'_, WorkspaceState>) -> Result<GitDiff, String> {
    let root = {
        let guard = state.0.lock().expect("workspace state lock poisoned");
        guard.as_ref().ok_or("No workspace is open.")?.root.clone()
    };

    let output = std::process::Command::new("git")
        .current_dir(&root)
        .args(["diff", "HEAD"])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;

    if output.status.success() {
        let lines = parse_diff(&String::from_utf8_lossy(&output.stdout));
        return Ok(GitDiff {
            is_repo: true,
            empty: lines.is_empty(),
            lines,
        });
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("not a git repository") {
        Ok(GitDiff {
            is_repo: false,
            empty: true,
            lines: Vec::new(),
        })
    } else if stderr.contains("ambiguous argument")
        || stderr.contains("unknown revision")
        || stderr.contains("bad revision")
    {
        Ok(GitDiff {
            is_repo: true,
            empty: true,
            lines: Vec::new(),
        })
    } else {
        Err(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn create_pr(root: &Path, title: &str, body: &str) -> Result<String, String> {
    let output = std::process::Command::new("gh")
        .current_dir(root)
        .args(["pr", "create", "--title", title, "--body", body])
        .output()
        .map_err(|error| format!("Could not run gh: {error}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "gh pr create failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn push(root: &Path, remote: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["push", "-u", remote, "HEAD"])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git push failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn switch_branch(root: &Path, name: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["switch", name])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git switch failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn commit_all(root: &Path, message: &str) -> Result<(), String> {
    let add = std::process::Command::new("git")
        .current_dir(root)
        .args(["add", "-A"])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;
    if !add.status.success() {
        return Err(format!(
            "git add failed: {}",
            String::from_utf8_lossy(&add.stderr).trim()
        ));
    }

    let commit = std::process::Command::new("git")
        .current_dir(root)
        .args(["commit", "-m", message])
        .output()
        .map_err(|error| format!("Could not run git: {error}"))?;
    if commit.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr).trim()
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
    fn parses_unified_diff_into_classified_lines() {
        let diff = "diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n context\n-old\n+new\n";
        let lines = parse_diff(diff);
        let kinds: Vec<&str> = lines.iter().map(|l| l.kind.as_str()).collect();
        assert_eq!(
            kinds,
            vec!["meta", "meta", "meta", "meta", "meta", "context", "del", "add"]
        );
    }

    #[test]
    fn diff_parser_caps_lines() {
        let big = "+a\n".repeat(MAX_DIFF_LINES + 100);
        assert_eq!(parse_diff(&big).len(), MAX_DIFF_LINES);
    }

    #[test]
    fn serializes_diff_in_camel_case() {
        let serialized = serde_json::to_value(GitDiff {
            is_repo: true,
            empty: false,
            lines: vec![DiffLine {
                kind: "add".to_string(),
                text: "+x".to_string(),
            }],
        })
        .unwrap();
        assert_eq!(serialized["isRepo"], true);
        assert_eq!(serialized["lines"][0]["kind"], "add");
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
    fn create_branch_in_a_temp_repo() {
        let dir = std::env::temp_dir().join(format!("synth-fs018-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .current_dir(&dir)
                .args(args)
                .output()
                .unwrap()
        };
        assert!(git(&["init", "-q"]).status.success());
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        assert!(git(&["commit", "--allow-empty", "-q", "-m", "init"])
            .status
            .success());

        assert!(create_branch(&dir, "feature/x").is_ok());
        let listed = git(&["branch", "--list", "feature/x"]);
        assert!(String::from_utf8_lossy(&listed.stdout).contains("feature/x"));

        assert!(create_branch(&dir, "feature/x").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn push_lands_branch_on_a_local_bare_remote() {
        let base = std::env::temp_dir().join(format!("synth-fs021-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let work = base.join("work");
        let bare = base.join("bare.git");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::create_dir_all(&bare).unwrap();

        let run = |dir: &Path, args: &[&str]| {
            std::process::Command::new("git")
                .current_dir(dir)
                .args(args)
                .output()
                .unwrap()
        };
        run(&bare, &["init", "--bare", "-q"]);
        run(&work, &["init", "-q"]);
        run(&work, &["config", "user.email", "t@t"]);
        run(&work, &["config", "user.name", "t"]);
        run(&work, &["commit", "--allow-empty", "-q", "-m", "init"]);
        run(&work, &["remote", "add", "origin", &bare.to_string_lossy()]);

        assert!(push(&work, "origin").is_ok());
        let refs = run(&work, &["ls-remote", "origin"]);
        assert!(!String::from_utf8_lossy(&refs.stdout).trim().is_empty());

        assert!(push(&work, "no-such-remote").is_err());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn switch_branch_changes_current_branch_in_a_temp_repo() {
        let dir = std::env::temp_dir().join(format!("synth-fs020-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .current_dir(&dir)
                .args(args)
                .output()
                .unwrap()
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        git(&["commit", "--allow-empty", "-q", "-m", "init"]);
        git(&["branch", "feature/x"]);

        assert!(switch_branch(&dir, "feature/x").is_ok());
        let current = git(&["branch", "--show-current"]);
        assert_eq!(String::from_utf8_lossy(&current.stdout).trim(), "feature/x");

        assert!(switch_branch(&dir, "does-not-exist").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn commit_all_stages_and_commits_in_a_temp_repo() {
        let dir = std::env::temp_dir().join(format!("synth-fs019-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .current_dir(&dir)
                .args(args)
                .output()
                .unwrap()
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        git(&["commit", "--allow-empty", "-q", "-m", "init"]);

        std::fs::write(dir.join("note.txt"), "hello").unwrap();
        assert!(commit_all(&dir, "add note").is_ok());

        let status = git(&["status", "--porcelain"]);
        assert!(String::from_utf8_lossy(&status.stdout).trim().is_empty());

        assert!(commit_all(&dir, "nothing to do").is_err());

        let _ = std::fs::remove_dir_all(&dir);
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
