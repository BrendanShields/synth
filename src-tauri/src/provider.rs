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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAnswer {
    pub model: String,
    pub prompt: String,
    pub answer: String,
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

pub fn build_generate_body(config: &ProviderConfig, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false,
    })
}

pub fn parse_generate_answer(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("invalid response: {error}"))?;

    parsed["response"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "response did not contain an answer".to_string())
}

#[tauri::command]
pub async fn ask_model(prompt: String) -> Result<ModelAnswer, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Ask needs a question after ?.".to_string());
    }

    let config = default_provider_config();
    let endpoint = format!("{}/api/generate", config.base_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| format!("client error: {error}"))?;

    let response = client
        .post(&endpoint)
        .json(&build_generate_body(&config, trimmed))
        .send()
        .await
        .map_err(|_| "Ollama is not reachable at the configured endpoint.".to_string())?;

    if !response.status().is_success() {
        return Err(format!("Ollama returned status {}.", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|error| format!("read error: {error}"))?;

    Ok(ModelAnswer {
        model: config.model,
        prompt: trimmed.to_string(),
        answer: parse_generate_answer(&body)?,
    })
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

    #[test]
    fn build_generate_body_requests_the_configured_model_without_streaming() {
        let body = build_generate_body(&default_provider_config(), "hello");

        assert_eq!(body["model"], "gemma4:e4b");
        assert_eq!(body["prompt"], "hello");
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn parses_answer_from_generate_response() {
        let body = r#"{"model":"gemma4:e4b","response":"4","done":true}"#;

        assert_eq!(parse_generate_answer(body).unwrap(), "4");
    }

    #[test]
    fn generate_answer_errors_on_malformed_or_missing_response() {
        assert!(parse_generate_answer("not json").is_err());
        assert!(parse_generate_answer(r#"{"done":true}"#).is_err());
    }

    #[test]
    fn serializes_model_answer_in_camel_case() {
        let serialized = serde_json::to_value(ModelAnswer {
            model: "gemma4:e4b".to_string(),
            prompt: "q".to_string(),
            answer: "a".to_string(),
        })
        .unwrap();

        assert_eq!(serialized["model"], "gemma4:e4b");
        assert_eq!(serialized["prompt"], "q");
        assert_eq!(serialized["answer"], "a");
    }
}
