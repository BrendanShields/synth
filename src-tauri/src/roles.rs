use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use crate::provider::ProviderState;

pub const ROLES: [&str; 5] = [
    "planner",
    "builder",
    "adversary",
    "summarizer",
    "requirements_critic",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignment {
    pub role: String,
    pub model: String,
    pub overridden: bool,
}

#[derive(Default)]
pub struct ModelRolesState(pub Mutex<HashMap<String, String>>);

pub fn is_valid_role(role: &str) -> bool {
    ROLES.contains(&role)
}

pub fn resolve_model_for_role(
    role: &str,
    overrides: &HashMap<String, String>,
    default_model: &str,
) -> String {
    overrides
        .get(role)
        .cloned()
        .unwrap_or_else(|| default_model.to_string())
}

#[tauri::command]
pub fn get_model_roles(
    provider: tauri::State<'_, ProviderState>,
    roles: tauri::State<'_, ModelRolesState>,
) -> Vec<RoleAssignment> {
    let default_model = provider
        .0
        .lock()
        .expect("provider state lock poisoned")
        .model
        .clone();
    let overrides = roles
        .0
        .lock()
        .expect("roles state lock poisoned")
        .clone();

    ROLES
        .iter()
        .map(|role| RoleAssignment {
            role: role.to_string(),
            model: resolve_model_for_role(role, &overrides, &default_model),
            overridden: overrides.contains_key(*role),
        })
        .collect()
}

#[tauri::command]
pub fn set_model_role(
    roles: tauri::State<'_, ModelRolesState>,
    role: String,
    model: String,
) -> Result<(), String> {
    if !is_valid_role(&role) {
        return Err("Invalid role.".to_string());
    }

    let mut overrides = roles.0.lock().expect("roles state lock poisoned");
    let model = model.trim();
    if model.is_empty() {
        overrides.remove(&role);
    } else {
        overrides.insert(role, model.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_roles() {
        assert!(is_valid_role("planner"));
        assert!(is_valid_role("requirements_critic"));
        assert!(!is_valid_role("bogus"));
        assert!(!is_valid_role(""));
    }

    #[test]
    fn resolves_override_or_default() {
        let mut overrides = HashMap::new();
        assert_eq!(
            resolve_model_for_role("planner", &overrides, "gemma4:e4b"),
            "gemma4:e4b"
        );
        overrides.insert("planner".to_string(), "llama3:8b".to_string());
        assert_eq!(
            resolve_model_for_role("planner", &overrides, "gemma4:e4b"),
            "llama3:8b"
        );
        assert_eq!(
            resolve_model_for_role("builder", &overrides, "gemma4:e4b"),
            "gemma4:e4b"
        );
    }

    #[test]
    fn serializes_assignment_in_camel_case() {
        let serialized = serde_json::to_value(RoleAssignment {
            role: "planner".to_string(),
            model: "gemma4:e4b".to_string(),
            overridden: false,
        })
        .unwrap();
        assert_eq!(serialized["role"], "planner");
        assert_eq!(serialized["model"], "gemma4:e4b");
        assert_eq!(serialized["overridden"], false);
    }
}
