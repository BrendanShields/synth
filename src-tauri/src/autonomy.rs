use std::sync::Mutex;

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomyMode {
    pub mode: String,
}

pub struct AutonomyState(pub Mutex<String>);

impl Default for AutonomyState {
    fn default() -> Self {
        AutonomyState(Mutex::new("supervised".to_string()))
    }
}

pub fn normalize_mode(input: &str) -> Option<&'static str> {
    match input.trim().to_ascii_lowercase().as_str() {
        "supervised" => Some("supervised"),
        "high_autonomy" | "high-autonomy" => Some("high_autonomy"),
        _ => None,
    }
}

#[tauri::command]
pub fn get_autonomy_mode(state: tauri::State<'_, AutonomyState>) -> AutonomyMode {
    AutonomyMode {
        mode: state.0.lock().expect("autonomy state lock poisoned").clone(),
    }
}

#[tauri::command]
pub fn set_autonomy_mode(
    state: tauri::State<'_, AutonomyState>,
    mode: String,
) -> Result<AutonomyMode, String> {
    let normalized = normalize_mode(&mode).ok_or("Invalid autonomy mode.")?;
    *state.0.lock().expect("autonomy state lock poisoned") = normalized.to_string();
    Ok(AutonomyMode {
        mode: normalized.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_valid_modes_case_insensitively() {
        assert_eq!(normalize_mode("supervised"), Some("supervised"));
        assert_eq!(normalize_mode("Supervised"), Some("supervised"));
        assert_eq!(normalize_mode("high_autonomy"), Some("high_autonomy"));
        assert_eq!(normalize_mode("High-Autonomy"), Some("high_autonomy"));
    }

    #[test]
    fn rejects_invalid_modes() {
        assert_eq!(normalize_mode("bogus"), None);
        assert_eq!(normalize_mode(""), None);
        assert_eq!(normalize_mode("autonomous"), None);
    }

    #[test]
    fn default_state_is_supervised() {
        let state = AutonomyState::default();
        assert_eq!(*state.0.lock().unwrap(), "supervised");
    }

    #[test]
    fn serializes_in_camel_case() {
        let serialized = serde_json::to_value(AutonomyMode {
            mode: "high_autonomy".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["mode"], "high_autonomy");
    }
}
