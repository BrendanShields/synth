use futures_util::StreamExt;
use serde::Serialize;
use tauri::Emitter;

use crate::specs_index::StaticSpecDetail;

pub const ANSWER_CHUNK_EVENT: &str = "synth-answer-chunk";
pub const ANSWER_DONE_EVENT: &str = "synth-answer-done";
pub const ANSWER_ERROR_EVENT: &str = "synth-answer-error";

const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const OLLAMA_MODEL: &str = "gemma4:e4b";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub kind: String,
    pub base_url: String,
    pub model: String,
    #[serde(skip)]
    pub api_key: Option<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecDraft {
    pub request: String,
    pub draft: String,
}

pub fn default_provider_config() -> ProviderConfig {
    ProviderConfig {
        kind: "ollama".to_string(),
        base_url: OLLAMA_BASE_URL.to_string(),
        model: OLLAMA_MODEL.to_string(),
        api_key: None,
    }
}

pub fn is_valid_provider_kind(kind: &str) -> bool {
    matches!(kind.trim().to_ascii_lowercase().as_str(), "ollama" | "openai")
}

pub struct ProviderState(pub std::sync::Mutex<ProviderConfig>);

impl Default for ProviderState {
    fn default() -> Self {
        ProviderState(std::sync::Mutex::new(default_provider_config()))
    }
}

fn current_config(state: &tauri::State<'_, ProviderState>) -> ProviderConfig {
    state
        .0
        .lock()
        .expect("provider state lock poisoned")
        .clone()
}

pub fn is_valid_base_url(url: &str) -> bool {
    let trimmed = url.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 300
        && !trimmed.chars().any(char::is_whitespace)
        && (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
}

#[tauri::command]
pub fn get_provider_config(state: tauri::State<'_, ProviderState>) -> ProviderConfig {
    current_config(&state)
}

#[tauri::command]
pub fn set_provider_config(
    state: tauri::State<'_, ProviderState>,
    kind: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<ProviderConfig, String> {
    if !is_valid_provider_kind(&kind) {
        return Err("Invalid provider kind.".to_string());
    }
    if !is_valid_base_url(&base_url) {
        return Err("Invalid base URL.".to_string());
    }
    let model = model.trim().to_string();
    if model.is_empty() || model.len() > 200 {
        return Err("Invalid model.".to_string());
    }

    let api_key = api_key.and_then(|key| {
        let trimmed = key.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let config = ProviderConfig {
        kind: kind.trim().to_ascii_lowercase(),
        base_url: base_url.trim().to_string(),
        model,
        api_key,
    };
    *state.0.lock().expect("provider state lock poisoned") = config.clone();
    Ok(config)
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

fn is_openai(config: &ProviderConfig) -> bool {
    config.kind == "openai"
}

pub fn request_endpoint(config: &ProviderConfig) -> String {
    if is_openai(config) {
        format!("{}/v1/chat/completions", config.base_url)
    } else {
        format!("{}/api/generate", config.base_url)
    }
}

pub fn build_request_body(config: &ProviderConfig, prompt: &str) -> serde_json::Value {
    if is_openai(config) {
        serde_json::json!({
            "model": config.model,
            "messages": [{ "role": "user", "content": prompt }],
            "stream": false,
        })
    } else {
        build_generate_body(config, prompt)
    }
}

pub fn parse_answer(kind: &str, body: &str) -> Result<String, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("invalid response: {error}"))?;

    let answer = if kind == "openai" {
        parsed["choices"][0]["message"]["content"].as_str()
    } else {
        parsed["response"].as_str()
    };

    answer
        .map(str::to_string)
        .ok_or_else(|| "response did not contain an answer".to_string())
}

pub fn parse_openai_models(body: &str) -> Vec<String> {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };

    parsed["data"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter_map(|model| model["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn build_spec_prompt(detail: &StaticSpecDetail, question: &str) -> String {
    format!(
        "You are answering a question about a software feature spec. Use only the context below.\n\n\
         Spec: {id} — {title}\n\
         Summary: {summary}\n\
         Scope: {scope}\n\
         Limitations: {limitations}\n\n\
         Question: {question}\n\n\
         Answer concisely using only that context.",
        id = detail.spec_id,
        title = detail.title,
        summary = detail.summary,
        scope = detail.scope.join("; "),
        limitations = detail.limitations.join("; "),
        question = question,
    )
}

async fn generate(config: &ProviderConfig, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| format!("client error: {error}"))?;

    let mut request = client
        .post(request_endpoint(config))
        .json(&build_request_body(config, prompt));
    if is_openai(config) {
        if let Some(key) = &config.api_key {
            request = request.bearer_auth(key);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|_| "Provider is not reachable at the configured endpoint.".to_string())?;

    if !response.status().is_success() {
        return Err(format!("Provider returned status {}.", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|error| format!("read error: {error}"))?;

    parse_answer(&config.kind, &body)
}

#[tauri::command]
pub async fn ask_model(
    state: tauri::State<'_, ProviderState>,
    prompt: String,
) -> Result<ModelAnswer, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Ask needs a question after ?.".to_string());
    }

    let config = current_config(&state);
    let answer = generate(&config, trimmed).await?;

    Ok(ModelAnswer {
        model: config.model,
        prompt: trimmed.to_string(),
        answer,
    })
}

pub fn spec_request_is_valid(request: &str) -> bool {
    !request.trim().is_empty()
}

pub fn build_spec_prompt_for_request(request: &str) -> String {
    format!(
        "You are drafting a story-sized feature spec for the request below. \
         Write concise markdown using exactly these section headings:\n\
         ## 1. Problem statement\n\
         ## 2. Requirements\n\
         ## 3. Acceptance criteria\n\
         ## 4. Tests / verification plan\n\
         ## 5. Success criteria\n\
         ## 6. Metrics used to evaluate success\n\n\
         Keep requirements testable and solution-free where appropriate.\n\n\
         Request: {request}\n\n\
         Feature spec:"
    )
}

#[tauri::command]
pub async fn draft_spec(
    state: tauri::State<'_, ProviderState>,
    request: String,
) -> Result<SpecDraft, String> {
    if !spec_request_is_valid(&request) {
        return Err("Provide a request to draft a spec.".to_string());
    }

    let trimmed = request.trim();
    let config = current_config(&state);
    let draft = generate(&config, &build_spec_prompt_for_request(trimmed)).await?;

    Ok(SpecDraft {
        request: trimmed.to_string(),
        draft,
    })
}

#[tauri::command]
pub async fn ask_spec(
    state: tauri::State<'_, ProviderState>,
    spec_id: String,
    question: String,
) -> Result<ModelAnswer, String> {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return Err("Ask needs a question after ?.".to_string());
    }

    let detail = crate::specs_index::lookup_static_spec_detail(&spec_id)?;
    let config = current_config(&state);
    let answer = generate(&config, &build_spec_prompt(&detail, trimmed)).await?;

    Ok(ModelAnswer {
        model: config.model,
        prompt: trimmed.to_string(),
        answer,
    })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StreamChunk {
    pub token: Option<String>,
    pub done: bool,
}

pub fn parse_stream_line(line: &str) -> StreamChunk {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return StreamChunk::default();
    }

    let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(value) => value,
        Err(_) => return StreamChunk::default(),
    };

    StreamChunk {
        token: parsed["response"]
            .as_str()
            .filter(|text| !text.is_empty())
            .map(str::to_string),
        done: parsed["done"].as_bool().unwrap_or(false),
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnswerChunk {
    request_id: u64,
    token: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnswerDone {
    request_id: u64,
    model: String,
    answer: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnswerError {
    request_id: u64,
    message: String,
}

#[tauri::command]
pub async fn ask_stream(
    app: tauri::AppHandle,
    state: tauri::State<'_, ProviderState>,
    request_id: u64,
    spec_id: Option<String>,
    question: String,
) -> Result<(), String> {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return Err("Ask needs a question after ?.".to_string());
    }

    let config = current_config(&state);
    let prompt = match &spec_id {
        Some(id) => build_spec_prompt(&crate::specs_index::lookup_static_spec_detail(id)?, trimmed),
        None => trimmed.to_string(),
    };

    let emit_error = |message: String| {
        let _ = app.emit(ANSWER_ERROR_EVENT, AnswerError { request_id, message });
    };

    if is_openai(&config) {
        match generate(&config, &prompt).await {
            Ok(answer) => {
                let _ = app.emit(
                    ANSWER_CHUNK_EVENT,
                    AnswerChunk {
                        request_id,
                        token: answer.clone(),
                    },
                );
                let _ = app.emit(
                    ANSWER_DONE_EVENT,
                    AnswerDone {
                        request_id,
                        model: config.model,
                        answer,
                    },
                );
            }
            Err(error) => emit_error(error),
        }
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| format!("client error: {error}"))?;

    let body = serde_json::json!({
        "model": config.model,
        "prompt": prompt,
        "stream": true,
    });
    let endpoint = format!("{}/api/generate", config.base_url);

    let response = match client.post(&endpoint).json(&body).send().await {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            emit_error(format!("Ollama returned status {}.", response.status()));
            return Ok(());
        }
        Err(_) => {
            emit_error("Ollama is not reachable at the configured endpoint.".to_string());
            return Ok(());
        }
    };

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut answer = String::new();

    while let Some(item) = stream.next().await {
        let bytes = match item {
            Ok(bytes) => bytes,
            Err(error) => {
                emit_error(format!("stream error: {error}"));
                return Ok(());
            }
        };
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(newline) = buffer.find('\n') {
            let line: String = buffer.drain(..=newline).collect();
            let chunk = parse_stream_line(&line);
            if let Some(token) = chunk.token {
                answer.push_str(&token);
                let _ = app.emit(
                    ANSWER_CHUNK_EVENT,
                    AnswerChunk { request_id, token },
                );
            }
            if chunk.done {
                let _ = app.emit(
                    ANSWER_DONE_EVENT,
                    AnswerDone {
                        request_id,
                        model: config.model,
                        answer,
                    },
                );
                return Ok(());
            }
        }
    }

    let chunk = parse_stream_line(&buffer);
    if let Some(token) = chunk.token {
        answer.push_str(&token);
        let _ = app.emit(ANSWER_CHUNK_EVENT, AnswerChunk { request_id, token });
    }
    let _ = app.emit(
        ANSWER_DONE_EVENT,
        AnswerDone {
            request_id,
            model: config.model,
            answer,
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn get_provider_status(
    state: tauri::State<'_, ProviderState>,
) -> Result<ProviderStatus, String> {
    let config = current_config(&state);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(error) => return Ok(unreachable_status(config, format!("client error: {error}"))),
    };

    let (endpoint, openai) = if is_openai(&config) {
        (format!("{}/v1/models", config.base_url), true)
    } else {
        (format!("{}/api/tags", config.base_url), false)
    };

    let mut request = client.get(&endpoint);
    if openai {
        if let Some(key) = &config.api_key {
            request = request.bearer_auth(key);
        }
    }

    let status = match request.send().await {
        Ok(response) if response.status().is_success() => match response.text().await {
            Ok(body) => {
                let models = if openai {
                    parse_openai_models(&body)
                } else {
                    parse_ollama_models(&body)
                };
                reachable_status(config, models)
            }
            Err(error) => unreachable_status(config, format!("read error: {error}")),
        },
        Ok(response) => {
            let code = response.status();
            unreachable_status(config, format!("provider returned status {code}."))
        }
        Err(_) => unreachable_status(
            config,
            "Provider is not reachable at the configured endpoint.".to_string(),
        ),
    };
    Ok(status)
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
    fn build_spec_prompt_grounds_in_spec_context_and_question() {
        let detail = crate::specs_index::lookup_static_spec_detail("FS-005").unwrap();
        let prompt = build_spec_prompt(&detail, "what does this add?");

        assert!(prompt.contains("FS-005"));
        assert!(prompt.contains(&detail.title));
        assert!(prompt.contains(&detail.summary));
        assert!(prompt.contains("what does this add?"));
    }

    #[test]
    fn parses_stream_lines_into_tokens_and_done() {
        let chunk = parse_stream_line(r#"{"response":"4","done":false}"#);
        assert_eq!(chunk.token.as_deref(), Some("4"));
        assert!(!chunk.done);

        let done = parse_stream_line(r#"{"response":"","done":true}"#);
        assert_eq!(done.token, None);
        assert!(done.done);

        assert_eq!(parse_stream_line("not json"), StreamChunk::default());
        assert_eq!(parse_stream_line("   "), StreamChunk::default());
    }

    #[test]
    fn serializes_answer_events_in_camel_case() {
        let chunk = serde_json::to_value(AnswerChunk {
            request_id: 7,
            token: "x".to_string(),
        })
        .unwrap();
        assert_eq!(chunk["requestId"], 7);
        assert_eq!(chunk["token"], "x");

        let done = serde_json::to_value(AnswerDone {
            request_id: 7,
            model: "gemma4:e4b".to_string(),
            answer: "done".to_string(),
        })
        .unwrap();
        assert_eq!(done["requestId"], 7);
        assert_eq!(done["answer"], "done");
    }

    #[test]
    fn spec_prompt_includes_request_and_six_sections() {
        let prompt = build_spec_prompt_for_request("Add a loading state to the dock");
        assert!(prompt.contains("Add a loading state to the dock"));
        for heading in [
            "## 1. Problem statement",
            "## 2. Requirements",
            "## 3. Acceptance criteria",
            "## 4. Tests / verification plan",
            "## 5. Success criteria",
            "## 6. Metrics used to evaluate success",
        ] {
            assert!(prompt.contains(heading), "missing {heading}");
        }
    }

    #[test]
    fn validates_base_urls() {
        assert!(is_valid_base_url("http://localhost:11434"));
        assert!(is_valid_base_url("https://api.example.com"));
        assert!(!is_valid_base_url(""));
        assert!(!is_valid_base_url("ftp://x"));
        assert!(!is_valid_base_url("http://has space"));
        assert!(!is_valid_base_url("localhost:11434"));
    }

    #[test]
    fn validates_provider_kinds() {
        assert!(is_valid_provider_kind("ollama"));
        assert!(is_valid_provider_kind("openai"));
        assert!(is_valid_provider_kind("OpenAI"));
        assert!(!is_valid_provider_kind("anthropic"));
        assert!(!is_valid_provider_kind(""));
    }

    fn openai_config() -> ProviderConfig {
        ProviderConfig {
            kind: "openai".to_string(),
            base_url: "https://api.example.com".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: Some("sk-test".to_string()),
        }
    }

    #[test]
    fn builds_kind_specific_request_bodies_and_endpoints() {
        let ollama = build_request_body(&default_provider_config(), "hi");
        assert_eq!(ollama["prompt"], "hi");
        assert_eq!(ollama["stream"], false);
        assert_eq!(
            request_endpoint(&default_provider_config()),
            "http://localhost:11434/api/generate"
        );

        let openai = build_request_body(&openai_config(), "hi");
        assert_eq!(openai["messages"][0]["role"], "user");
        assert_eq!(openai["messages"][0]["content"], "hi");
        assert_eq!(openai["stream"], false);
        assert_eq!(
            request_endpoint(&openai_config()),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn parses_kind_specific_answers() {
        assert_eq!(
            parse_answer("ollama", r#"{"response":"4"}"#).unwrap(),
            "4"
        );
        assert_eq!(
            parse_answer(
                "openai",
                r#"{"choices":[{"message":{"content":"4"}}]}"#
            )
            .unwrap(),
            "4"
        );
        assert!(parse_answer("openai", r#"{"choices":[]}"#).is_err());
        assert!(parse_answer("ollama", "not json").is_err());
    }

    #[test]
    fn parses_openai_models() {
        let body = r#"{"data":[{"id":"gpt-4o-mini"},{"id":"gpt-4o"}]}"#;
        assert_eq!(
            parse_openai_models(body),
            vec!["gpt-4o-mini".to_string(), "gpt-4o".to_string()]
        );
        assert!(parse_openai_models("{}").is_empty());
    }

    #[test]
    fn api_key_is_not_serialized_to_the_renderer() {
        let serialized = serde_json::to_value(openai_config()).unwrap();
        assert_eq!(serialized["kind"], "openai");
        assert!(serialized.get("apiKey").is_none());
        assert!(serialized.get("api_key").is_none());
    }

    #[test]
    fn spec_request_validation_rejects_empty() {
        assert!(spec_request_is_valid("do a thing"));
        assert!(!spec_request_is_valid("   "));
        assert!(!spec_request_is_valid(""));
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
