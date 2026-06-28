use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;

use crate::git;
use crate::workspace::WorkspaceState;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub id: u64,
    pub action: String,
    pub summary: String,
    pub command: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalOutcome {
    pub id: u64,
    pub approved: bool,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingAction {
    CreateBranch(String),
    Commit(String),
    SwitchBranch(String),
    Push(String),
    CreatePr { title: String, body: String },
    SaveSpec { spec_id: String, content: String },
    SaveAmendment {
        spec_id: String,
        amendment_id: String,
        content: String,
    },
}

#[derive(Default)]
struct ApprovalInner {
    next_id: u64,
    pending: HashMap<u64, PendingAction>,
}

#[derive(Default)]
pub struct ApprovalState(Mutex<ApprovalInner>);

pub fn is_valid_branch_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 200 {
        return false;
    }
    if name.starts_with('-') || name.starts_with('/') || name.ends_with('/') {
        return false;
    }
    if name.contains("..") || name.contains("//") {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
}

pub fn is_valid_commit_message(message: &str) -> bool {
    !message.trim().is_empty() && message.len() <= 2000
}

pub fn is_valid_remote_name(remote: &str) -> bool {
    if remote.is_empty() || remote.len() > 100 || remote.starts_with('-') {
        return false;
    }
    if remote.contains("..") {
        return false;
    }
    remote
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub fn is_valid_pr_title(title: &str) -> bool {
    !title.trim().is_empty() && title.len() <= 200
}

fn truncate_for_display(text: &str) -> String {
    let oneline = text.replace('\n', " ");
    if oneline.chars().count() <= 60 {
        oneline
    } else {
        let head: String = oneline.chars().take(60).collect();
        format!("{head}…")
    }
}

impl ApprovalInner {
    fn record_branch(&mut self, name: &str) -> ApprovalRequest {
        let id = self.next_id;
        self.next_id += 1;
        self.pending
            .insert(id, PendingAction::CreateBranch(name.to_string()));
        ApprovalRequest {
            id,
            action: "create-branch".to_string(),
            summary: format!("Create branch {name}"),
            command: format!("git branch {name}"),
        }
    }

    fn record_commit(&mut self, message: &str) -> ApprovalRequest {
        let id = self.next_id;
        self.next_id += 1;
        self.pending
            .insert(id, PendingAction::Commit(message.to_string()));
        ApprovalRequest {
            id,
            action: "commit".to_string(),
            summary: format!("Commit: {message}"),
            command: format!("git add -A && git commit -m \"{message}\""),
        }
    }

    fn record_switch_branch(&mut self, name: &str) -> ApprovalRequest {
        let id = self.next_id;
        self.next_id += 1;
        self.pending
            .insert(id, PendingAction::SwitchBranch(name.to_string()));
        ApprovalRequest {
            id,
            action: "switch-branch".to_string(),
            summary: format!("Switch to branch {name}"),
            command: format!("git switch {name}"),
        }
    }

    fn record_push(&mut self, remote: &str) -> ApprovalRequest {
        let id = self.next_id;
        self.next_id += 1;
        self.pending
            .insert(id, PendingAction::Push(remote.to_string()));
        ApprovalRequest {
            id,
            action: "push".to_string(),
            summary: format!("Push current branch to {remote}"),
            command: format!("git push -u {remote} HEAD"),
        }
    }

    fn record_create_pr(&mut self, title: &str, body: &str) -> ApprovalRequest {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.insert(
            id,
            PendingAction::CreatePr {
                title: title.to_string(),
                body: body.to_string(),
            },
        );
        ApprovalRequest {
            id,
            action: "create-pr".to_string(),
            summary: format!("Open pull request: {title}"),
            command: format!(
                "gh pr create --title \"{title}\" --body \"{}\"",
                truncate_for_display(body)
            ),
        }
    }

    fn record_save_spec(&mut self, spec_id: &str, content: &str) -> ApprovalRequest {
        let id = self.next_id;
        self.next_id += 1;
        let path = format!("docs/specs/{spec_id}/spec.md");
        self.pending.insert(
            id,
            PendingAction::SaveSpec {
                spec_id: spec_id.to_string(),
                content: content.to_string(),
            },
        );
        ApprovalRequest {
            id,
            action: "save-spec".to_string(),
            summary: format!("Save spec {spec_id}"),
            command: format!("write {path}"),
        }
    }

    fn record_save_amendment(
        &mut self,
        spec_id: &str,
        amendment_id: &str,
        content: &str,
    ) -> ApprovalRequest {
        let id = self.next_id;
        self.next_id += 1;
        let path = format!("docs/specs/{spec_id}/amendments/{amendment_id}.md");
        self.pending.insert(
            id,
            PendingAction::SaveAmendment {
                spec_id: spec_id.to_string(),
                amendment_id: amendment_id.to_string(),
                content: content.to_string(),
            },
        );
        ApprovalRequest {
            id,
            action: "save-amendment".to_string(),
            summary: format!("Save amendment {amendment_id} for {spec_id}"),
            command: format!("write {path}"),
        }
    }

    fn take(&mut self, id: u64) -> Option<PendingAction> {
        self.pending.remove(&id)
    }
}

#[tauri::command]
pub fn request_create_branch(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    name: String,
) -> Result<ApprovalRequest, String> {
    if !is_valid_branch_name(&name) {
        return Err("Invalid branch name.".to_string());
    }
    if workspace
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .is_none()
    {
        return Err("No workspace is open.".to_string());
    }

    Ok(approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .record_branch(&name))
}

#[tauri::command]
pub fn request_commit(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    message: String,
) -> Result<ApprovalRequest, String> {
    if !is_valid_commit_message(&message) {
        return Err("Invalid commit message.".to_string());
    }
    if workspace
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .is_none()
    {
        return Err("No workspace is open.".to_string());
    }

    Ok(approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .record_commit(&message))
}

#[tauri::command]
pub fn request_switch_branch(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    name: String,
) -> Result<ApprovalRequest, String> {
    if !is_valid_branch_name(&name) {
        return Err("Invalid branch name.".to_string());
    }
    if workspace
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .is_none()
    {
        return Err("No workspace is open.".to_string());
    }

    Ok(approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .record_switch_branch(&name))
}

#[tauri::command]
pub fn request_push(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    remote: String,
) -> Result<ApprovalRequest, String> {
    let remote = {
        let trimmed = remote.trim();
        if trimmed.is_empty() {
            "origin".to_string()
        } else {
            trimmed.to_string()
        }
    };
    if !is_valid_remote_name(&remote) {
        return Err("Invalid remote name.".to_string());
    }
    if workspace
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .is_none()
    {
        return Err("No workspace is open.".to_string());
    }

    Ok(approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .record_push(&remote))
}

#[tauri::command]
pub fn request_save_spec(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    spec_id: String,
    content: String,
) -> Result<ApprovalRequest, String> {
    let canonical = crate::workspace::spec_id_from_dir_name(&spec_id)
        .ok_or("Invalid spec id.")?;
    if content.trim().is_empty() || content.len() > 100_000 {
        return Err("Invalid spec content.".to_string());
    }
    if workspace
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .is_none()
    {
        return Err("No workspace is open.".to_string());
    }

    Ok(approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .record_save_spec(&canonical, &content))
}

#[tauri::command]
pub fn request_save_amendment(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    spec_id: String,
    amendment_id: String,
    content: String,
) -> Result<ApprovalRequest, String> {
    let spec = crate::workspace::spec_id_from_dir_name(&spec_id).ok_or("Invalid spec id.")?;
    let amendment =
        crate::workspace::amendment_id_from_name(&amendment_id).ok_or("Invalid amendment id.")?;
    if content.trim().is_empty() || content.len() > 100_000 {
        return Err("Invalid amendment content.".to_string());
    }
    if workspace
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .is_none()
    {
        return Err("No workspace is open.".to_string());
    }

    Ok(approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .record_save_amendment(&spec, &amendment, &content))
}

#[tauri::command]
pub fn request_create_pr(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    title: String,
    body: String,
) -> Result<ApprovalRequest, String> {
    if !is_valid_pr_title(&title) {
        return Err("Invalid pull request title.".to_string());
    }
    if body.len() > 50_000 {
        return Err("Pull request body is too long.".to_string());
    }
    if workspace
        .0
        .lock()
        .expect("workspace state lock poisoned")
        .is_none()
    {
        return Err("No workspace is open.".to_string());
    }

    Ok(approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .record_create_pr(&title, &body))
}

#[tauri::command]
pub fn resolve_approval(
    approvals: tauri::State<'_, ApprovalState>,
    workspace: tauri::State<'_, WorkspaceState>,
    id: u64,
    approved: bool,
) -> Result<ApprovalOutcome, String> {
    let action = approvals
        .0
        .lock()
        .expect("approval state lock poisoned")
        .take(id)
        .ok_or("Unknown or already-resolved approval.")?;

    if !approved {
        return Ok(ApprovalOutcome {
            id,
            approved: false,
            message: "Denied.".to_string(),
        });
    }

    let root = {
        let guard = workspace.0.lock().expect("workspace state lock poisoned");
        guard.as_ref().ok_or("No workspace is open.")?.root.clone()
    };

    match action {
        PendingAction::CreateBranch(name) => {
            git::create_branch(Path::new(&root), &name)?;
            Ok(ApprovalOutcome {
                id,
                approved: true,
                message: format!("Created branch {name}."),
            })
        }
        PendingAction::Commit(message) => {
            git::commit_all(Path::new(&root), &message)?;
            Ok(ApprovalOutcome {
                id,
                approved: true,
                message: "Committed changes.".to_string(),
            })
        }
        PendingAction::SwitchBranch(name) => {
            git::switch_branch(Path::new(&root), &name)?;
            Ok(ApprovalOutcome {
                id,
                approved: true,
                message: format!("Switched to branch {name}."),
            })
        }
        PendingAction::Push(remote) => {
            git::push(Path::new(&root), &remote)?;
            Ok(ApprovalOutcome {
                id,
                approved: true,
                message: format!("Pushed to {remote}."),
            })
        }
        PendingAction::CreatePr { title, body } => {
            let url = git::create_pr(Path::new(&root), &title, &body)?;
            Ok(ApprovalOutcome {
                id,
                approved: true,
                message: if url.is_empty() {
                    "Pull request created.".to_string()
                } else {
                    url
                },
            })
        }
        PendingAction::SaveSpec { spec_id, content } => {
            let path = crate::workspace::write_spec_file(Path::new(&root), &spec_id, &content)?;
            Ok(ApprovalOutcome {
                id,
                approved: true,
                message: format!("Saved {path}."),
            })
        }
        PendingAction::SaveAmendment {
            spec_id,
            amendment_id,
            content,
        } => {
            let path = crate::workspace::write_amendment_file(
                Path::new(&root),
                &spec_id,
                &amendment_id,
                &content,
            )?;
            Ok(ApprovalOutcome {
                id,
                approved: true,
                message: format!("Saved {path}."),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_branch_names() {
        for name in ["feature/x", "fix-1", "a.b", "release/v1.2", "synth/fs-018"] {
            assert!(is_valid_branch_name(name), "{name} should be valid");
        }
    }

    #[test]
    fn rejects_invalid_branch_names() {
        for name in ["", "-rf", "has space", "a..b", "feature/", "/x", "a//b", "x\u{7}"] {
            assert!(!is_valid_branch_name(name), "{name} should be invalid");
        }
    }

    #[test]
    fn request_records_a_pending_action_and_builds_the_exact_command() {
        let mut inner = ApprovalInner::default();
        let request = inner.record_branch("feature/x");

        assert_eq!(request.id, 0);
        assert_eq!(request.action, "create-branch");
        assert_eq!(request.command, "git branch feature/x");
        assert_eq!(inner.pending.len(), 1);
    }

    #[test]
    fn validates_commit_messages() {
        assert!(is_valid_commit_message("docs: update"));
        assert!(!is_valid_commit_message(""));
        assert!(!is_valid_commit_message("   \n"));
        assert!(!is_valid_commit_message(&"x".repeat(2001)));
    }

    #[test]
    fn records_commit_action_with_exact_effect() {
        let mut inner = ApprovalInner::default();
        let request = inner.record_commit("docs: update");

        assert_eq!(request.action, "commit");
        assert_eq!(
            request.command,
            "git add -A && git commit -m \"docs: update\""
        );
        assert_eq!(
            inner.take(request.id),
            Some(PendingAction::Commit("docs: update".to_string()))
        );
    }

    #[test]
    fn records_switch_action_with_exact_command() {
        let mut inner = ApprovalInner::default();
        let request = inner.record_switch_branch("feature/x");

        assert_eq!(request.action, "switch-branch");
        assert_eq!(request.command, "git switch feature/x");
        assert_eq!(
            inner.take(request.id),
            Some(PendingAction::SwitchBranch("feature/x".to_string()))
        );
    }

    #[test]
    fn validates_remote_names() {
        for remote in ["origin", "upstream", "my-fork", "r1.2"] {
            assert!(is_valid_remote_name(remote), "{remote} should be valid");
        }
        for remote in ["", "-x", "a/b", "has space", "https://x", "a..b"] {
            assert!(!is_valid_remote_name(remote), "{remote} should be invalid");
        }
    }

    #[test]
    fn records_push_action_with_exact_command() {
        let mut inner = ApprovalInner::default();
        let request = inner.record_push("origin");

        assert_eq!(request.action, "push");
        assert_eq!(request.command, "git push -u origin HEAD");
        assert_eq!(
            inner.take(request.id),
            Some(PendingAction::Push("origin".to_string()))
        );
    }

    #[test]
    fn records_save_spec_action_with_path_command() {
        let mut inner = ApprovalInner::default();
        let request = inner.record_save_spec("FS-099", "content");

        assert_eq!(request.action, "save-spec");
        assert_eq!(request.command, "write docs/specs/FS-099/spec.md");
        assert_eq!(
            inner.take(request.id),
            Some(PendingAction::SaveSpec {
                spec_id: "FS-099".to_string(),
                content: "content".to_string(),
            })
        );
    }

    #[test]
    fn records_save_amendment_action_with_path_command() {
        let mut inner = ApprovalInner::default();
        let request = inner.record_save_amendment("FS-005", "AMD-001", "deviation");

        assert_eq!(request.action, "save-amendment");
        assert_eq!(request.command, "write docs/specs/FS-005/amendments/AMD-001.md");
        assert_eq!(
            inner.take(request.id),
            Some(PendingAction::SaveAmendment {
                spec_id: "FS-005".to_string(),
                amendment_id: "AMD-001".to_string(),
                content: "deviation".to_string(),
            })
        );
    }

    #[test]
    fn validates_pr_titles() {
        assert!(is_valid_pr_title("Add feature"));
        assert!(!is_valid_pr_title(""));
        assert!(!is_valid_pr_title("   "));
        assert!(!is_valid_pr_title(&"x".repeat(201)));
    }

    #[test]
    fn records_pr_action_capturing_title_and_body_with_truncated_display() {
        let mut inner = ApprovalInner::default();
        let body = "line one\n".to_string() + &"y".repeat(100);
        let request = inner.record_create_pr("Add feature", &body);

        assert_eq!(request.action, "create-pr");
        assert!(request.command.starts_with("gh pr create --title \"Add feature\""));
        assert!(request.command.contains('…'));
        assert!(!request.command.contains('\n'));
        assert_eq!(
            inner.take(request.id),
            Some(PendingAction::CreatePr {
                title: "Add feature".to_string(),
                body,
            })
        );
    }

    #[test]
    fn take_resolves_once_then_is_gone() {
        let mut inner = ApprovalInner::default();
        let request = inner.record_branch("feature/x");

        assert_eq!(
            inner.take(request.id),
            Some(PendingAction::CreateBranch("feature/x".to_string()))
        );
        assert_eq!(inner.take(request.id), None);
        assert_eq!(inner.take(999), None);
    }

    #[test]
    fn ids_increment_per_request() {
        let mut inner = ApprovalInner::default();
        assert_eq!(inner.record_branch("a").id, 0);
        assert_eq!(inner.record_branch("b").id, 1);
        assert_eq!(inner.pending.len(), 2);
    }

    #[test]
    fn serializes_request_and_outcome_in_camel_case() {
        let request = serde_json::to_value(ApprovalRequest {
            id: 3,
            action: "create-branch".to_string(),
            summary: "Create branch x".to_string(),
            command: "git branch x".to_string(),
        })
        .unwrap();
        assert_eq!(request["id"], 3);
        assert_eq!(request["command"], "git branch x");

        let outcome = serde_json::to_value(ApprovalOutcome {
            id: 3,
            approved: true,
            message: "ok".to_string(),
        })
        .unwrap();
        assert_eq!(outcome["approved"], true);
    }
}
