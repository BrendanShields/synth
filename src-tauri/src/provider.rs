use serde::Serialize;

const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const OLLAMA_MODEL: &str = "gemma4:e4b";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub kind: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub kind: String,
    pub base_url: String,
    pub model: String,
    pub state: String,
    pub model_present: bool,
    pub available_models: Vec<String>,
    pub detail: String,
}

pub fn default_provider_config() -> ProviderConfig {
    ProviderConfig {
        kind: "ollama".to_string(),
        base_url: OLLAMA_BASE_URL.to_string(),
        model: OLLAMA_MODEL.to_string(),
    }
}

pub fn parse_ollama_models(body: &str) -> Vec<String> {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };

    parsed["models"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter_map(|model| model["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn model_is_present(model: &str, available: &[String]) -> bool {
    available.iter().any(|candidate| candidate == model)
}

fn reachable_status(config: ProviderConfig, available_models: Vec<String>) -> ProviderStatus {
    let model_present = model_is_present(&config.model, &available_models);
    let detail = if model_present {
        format!("{} is available on {}.", config.model, config.kind)
    } else {
        format!("{} not found on {}.", config.model, config.kind)
    };

    ProviderStatus {
        kind: config.kind,
        base_url: config.base_url,
        model: config.model,
        state: "reachable".to_string(),
        model_present,
        available_models,
        detail,
    }
}

fn unreachable_status(config: ProviderConfig, detail: String) -> ProviderStatus {
    ProviderStatus {
        kind: config.kind,
        base_url: config.base_url,
        model: config.model,
        state: "unreachable".to_string(),
        model_present: false,
        available_models: Vec::new(),
        detail,
    }
}

#[tauri::command]
pub async fn get_provider_status() -> ProviderStatus {
    let config = default_provider_config();
    let endpoint = format!("{}/api/tags", config.base_url);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(error) => return unreachable_status(config, format!("client error: {error}")),
    };

    match client.get(&endpoint).send().await {
        Ok(response) => match response.text().await {
            Ok(body) => reachable_status(config, parse_ollama_models(&body)),
            Err(error) => unreachable_status(config, format!("read error: {error}")),
        },
        Err(_) => unreachable_status(
            config,
            "Ollama is not reachable at the configured endpoint.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_targets_local_ollama() {
        let config = default_provider_config();

        assert_eq!(config.kind, "ollama");
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "gemma4:e4b");
    }

    #[test]
    fn parses_model_names_from_tags_body() {
        let body = r#"{"models":[{"name":"gemma4:e4b"},{"name":"llama3:8b"}]}"#;

        assert_eq!(
            parse_ollama_models(body),
            vec!["gemma4:e4b".to_string(), "llama3:8b".to_string()]
        );
    }

    #[test]
    fn returns_empty_for_malformed_or_empty_tags_body() {
        assert!(parse_ollama_models("not json").is_empty());
        assert!(parse_ollama_models("{}").is_empty());
    }

    #[test]
    fn model_presence_is_exact() {
        let available = vec!["gemma4:e4b".to_string(), "llama3:8b".to_string()];

        assert!(model_is_present("gemma4:e4b", &available));
        assert!(!model_is_present("gemma4", &available));
        assert!(!model_is_present("GEMMA4:E4B", &available));
    }

    #[test]
    fn reachable_status_reports_model_presence() {
        let present = reachable_status(
            default_provider_config(),
            vec!["gemma4:e4b".to_string()],
        );
        assert_eq!(present.state, "reachable");
        assert!(present.model_present);
        assert!(present.detail.contains("available"));

        let absent = reachable_status(default_provider_config(), vec!["llama3:8b".to_string()]);
        assert_eq!(absent.state, "reachable");
        assert!(!absent.model_present);
        assert!(absent.detail.contains("not found"));
    }

    #[test]
    fn unreachable_status_is_graceful() {
        let status = unreachable_status(default_provider_config(), "down".to_string());

        assert_eq!(status.state, "unreachable");
        assert!(!status.model_present);
        assert!(status.available_models.is_empty());
        assert_eq!(status.detail, "down");
    }

    #[test]
    fn serializes_status_for_the_react_ipc_contract_in_camel_case() {
        let status = reachable_status(default_provider_config(), vec!["gemma4:e4b".to_string()]);
        let serialized = serde_json::to_value(status).unwrap();

        assert_eq!(serialized["baseUrl"], "http://localhost:11434");
        assert_eq!(serialized["modelPresent"], true);
        assert!(serialized["availableModels"].is_array());
    }
}
