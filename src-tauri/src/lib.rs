mod approvals;
mod autonomy;
mod classify;
mod command_dock;
mod git;
mod provider;
mod runtime_status;
mod specs_index;
mod workspace;

use approvals::{
    request_commit, request_create_branch, request_create_pr, request_push,
    request_save_spec, request_switch_branch, resolve_approval, ApprovalState,
};
use autonomy::{get_autonomy_mode, set_autonomy_mode, AutonomyState};
use classify::classify_request;
use command_dock::{parse_command, route_command};
use git::{git_diff, git_log, git_status};
use provider::{ask_model, ask_spec, ask_stream, draft_spec, get_provider_status};
use runtime_status::{announce_runtime_status, get_runtime_status};
use specs_index::{get_static_spec_detail, list_specs_index};
use workspace::{
    get_workspace, inspect_planning_baseline, list_workspace_specs, open_workspace,
    read_workspace_doc, WorkspaceState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(WorkspaceState::default())
        .manage(ApprovalState::default())
        .manage(AutonomyState::default())
        .invoke_handler(tauri::generate_handler![
            get_runtime_status,
            announce_runtime_status,
            parse_command,
            route_command,
            list_specs_index,
            get_static_spec_detail,
            get_provider_status,
            ask_model,
            ask_spec,
            ask_stream,
            open_workspace,
            get_workspace,
            inspect_planning_baseline,
            read_workspace_doc,
            list_workspace_specs,
            git_status,
            git_log,
            git_diff,
            request_create_branch,
            request_commit,
            request_switch_branch,
            request_push,
            request_create_pr,
            request_save_spec,
            resolve_approval,
            classify_request,
            draft_spec,
            get_autonomy_mode,
            set_autonomy_mode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
