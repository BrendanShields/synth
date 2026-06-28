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
    let endpoint = format!("{}/api/generate", config.base_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| format!("client error: {error}"))?;

    let response = client
        .post(&endpoint)
        .json(&build_generate_body(config, prompt))
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

    parse_generate_answer(&body)
}

#[tauri::command]
pub async fn ask_model(prompt: String) -> Result<ModelAnswer, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Ask needs a question after ?.".to_string());
    }

    let config = default_provider_config();
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
pub async fn draft_spec(request: String) -> Result<SpecDraft, String> {
    if !spec_request_is_valid(&request) {
        return Err("Provide a request to draft a spec.".to_string());
    }

    let trimmed = request.trim();
    let config = default_provider_config();
    let draft = generate(&config, &build_spec_prompt_for_request(trimmed)).await?;

    Ok(SpecDraft {
        request: trimmed.to_string(),
        draft,
    })
}

#[tauri::command]
pub async fn ask_spec(spec_id: String, question: String) -> Result<ModelAnswer, String> {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return Err("Ask needs a question after ?.".to_string());
    }

    let detail = crate::specs_index::lookup_static_spec_detail(&spec_id)?;
    let config = default_provider_config();
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
    request_id: u64,
    spec_id: Option<String>,
    question: String,
) -> Result<(), String> {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return Err("Ask needs a question after ?.".to_string());
    }

    let config = default_provider_config();
    let prompt = match &spec_id {
        Some(id) => build_spec_prompt(&crate::specs_index::lookup_static_spec_detail(id)?, trimmed),
        None => trimmed.to_string(),
    };

    let emit_error = |message: String| {
        let _ = app.emit(ANSWER_ERROR_EVENT, AnswerError { request_id, message });
    };

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
