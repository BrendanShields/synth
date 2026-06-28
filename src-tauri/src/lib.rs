mod command_dock;
mod provider;
mod runtime_status;
mod specs_index;

use command_dock::{parse_command, route_command};
use provider::{ask_model, ask_spec, get_provider_status};
use runtime_status::{announce_runtime_status, get_runtime_status};
use specs_index::{get_static_spec_detail, list_specs_index};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_runtime_status,
            announce_runtime_status,
            parse_command,
            route_command,
            list_specs_index,
            get_static_spec_detail,
            get_provider_status,
            ask_model,
            ask_spec
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
