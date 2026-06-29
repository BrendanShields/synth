use serde::Serialize;
use tauri::Emitter;

pub const RUNTIME_STATUS_EVENT: &str = "synth-runtime-status";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub product_name: String,
    pub app_version: String,
    pub runtime_boundary: String,
    pub renderer_boundary: String,
    pub autonomy_mode: String,
    pub planning_gate: String,
    pub workspace_state: String,
    pub provider_state: String,
    pub event_stream_state: String,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEvent {
    pub event_id: String,
    pub event_type: String,
    pub status: RuntimeStatus,
}

pub fn bootstrap_runtime_status() -> RuntimeStatus {
    RuntimeStatus {
        product_name: "Synth".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        runtime_boundary: "rust-tauri-core".to_string(),
        renderer_boundary: "react-thin-renderer".to_string(),
        autonomy_mode: "supervised".to_string(),
        planning_gate: "clear".to_string(),
        workspace_state: "not_opened".to_string(),
        provider_state: "not_configured".to_string(),
        event_stream_state: "ready".to_string(),
        summary: "Planning baseline merged. Ready for Phase 1 walking skeleton.".to_string(),
    }
}

pub fn bootstrap_runtime_event() -> RuntimeEvent {
    RuntimeEvent {
        event_id: "runtime-status-bootstrap".to_string(),
        event_type: "runtime.status.snapshot".to_string(),
        status: bootstrap_runtime_status(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppIdentity {
    pub name: String,
    pub version: String,
}

#[tauri::command]
pub fn app_identity(app: tauri::AppHandle) -> AppIdentity {
    let info = app.package_info();
    AppIdentity {
        name: info.name.clone(),
        version: info.version.to_string(),
    }
}

#[tauri::command]
pub fn get_runtime_status() -> RuntimeStatus {
    bootstrap_runtime_status()
}

#[tauri::command]
pub fn announce_runtime_status(app: tauri::AppHandle) -> Result<RuntimeEvent, String> {
    let event = bootstrap_runtime_event();

    app.emit(RUNTIME_STATUS_EVENT, &event)
        .map_err(|error| format!("failed to emit runtime status event: {error}"))?;

    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_identity_serializes_in_camel_case() {
        let serialized = serde_json::to_value(AppIdentity {
            name: "synth".to_string(),
            version: "0.1.0".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["name"], "synth");
        assert_eq!(serialized["version"], "0.1.0");
    }

    #[test]
    fn bootstrap_runtime_status_matches_fs_001_contract() {
        let status = bootstrap_runtime_status();

        assert_eq!(status.product_name, "Synth");
        assert_eq!(status.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(status.runtime_boundary, "rust-tauri-core");
        assert_eq!(status.renderer_boundary, "react-thin-renderer");
        assert_eq!(status.autonomy_mode, "supervised");
        assert_eq!(status.planning_gate, "clear");
        assert_eq!(status.workspace_state, "not_opened");
        assert_eq!(status.provider_state, "not_configured");
        assert_eq!(status.event_stream_state, "ready");
        assert_eq!(
            status.summary,
            "Planning baseline merged. Ready for Phase 1 walking skeleton."
        );
    }

    #[test]
    fn bootstrap_runtime_event_wraps_status_snapshot() {
        let event = bootstrap_runtime_event();

        assert_eq!(event.event_id, "runtime-status-bootstrap");
        assert_eq!(event.event_type, "runtime.status.snapshot");
        assert_eq!(event.status, bootstrap_runtime_status());
    }
}
