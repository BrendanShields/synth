mod command_dock;
mod runtime_status;

use command_dock::parse_command;
use runtime_status::{announce_runtime_status, get_runtime_status};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_runtime_status,
            announce_runtime_status,
            parse_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
