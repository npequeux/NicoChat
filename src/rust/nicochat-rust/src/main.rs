use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, sync::Arc};

const INDEX_HTML: &str = include_str!("../../../../web/index.html");
const APP_JS: &str = include_str!("../../../../web/app.js");
const STYLES_CSS: &str = include_str!("../../../../web/styles.css");
const MANIFEST: &str = include_str!("../../../../web/manifest.webmanifest");
const SW_JS: &str = include_str!("../../../../web/sw.js");

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(5000);

    let state = Arc::new(AppState {
        client: Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .expect("reqwest client"),
        ollama_url: env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
            .trim_end_matches('/')
            .to_string(),
        model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3".to_string()),
        use_mock: env::var("NICOCHAT_USE_MOCK")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/app.js", get(app_js))
        .route("/styles.css", get(styles_css))
        .route("/manifest.webmanifest", get(manifest))
        .route("/sw.js", get(sw_js))
        .route("/api/health", get(health))
        .route("/api/models", get(models))
        .route("/api/chat", post(chat))
        .route("/api/restart-ollama", post(restart_ollama))
        .with_state(state);

use std::process::Command;

async fn restart_ollama() -> impl IntoResponse {
    // Try to stop Ollama (ignore errors if not running)
    let _ = Command::new("ollama").arg("stop").output();
    // Start Ollama (in background)
    let result = Command::new("ollama").arg("serve").spawn();
    match result {
        Ok(_) => (StatusCode::OK, "Ollama restarted".to_string()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to restart Ollama: {e}")),
    }
}

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind local interface");

    println!("NicoChat Rust backend listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("server");
}

#[derive(Clone)]
struct AppState {
    client: Client,
    ollama_url: String,
    model: String,
    use_mock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<ChatMessage>,
    model: Option<String>,
    speed_mode: Option<String>,
    accelerator: Option<String>, // legacy single value
    accelerators: Option<Vec<String>>, // new: array of enabled
    internet_access: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatReply {
    role: &'static str,
    content: String,
    mode: &'static str,
    model: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    backend: &'static str,
    model: String,
    mode: &'static str,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    models: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Option<Vec<OllamaModelInfo>>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelInfo {
    name: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: Option<OllamaMessage>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: Option<String>,
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn app_js() -> Response {
    static_text(APP_JS, "application/javascript; charset=utf-8")
}

async fn styles_css() -> Response {
    static_text(STYLES_CSS, "text/css; charset=utf-8")
}

async fn manifest() -> Response {
    static_text(MANIFEST, "application/manifest+json; charset=utf-8")
}

async fn sw_js() -> Response {
    static_text(SW_JS, "application/javascript; charset=utf-8")
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        backend: "Rust",
        model: state.model.clone(),
        mode: state.mode(),
    })
}

async fn models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if state.use_mock {
        return Json(ModelsResponse { models: vec![state.model.clone()] });
    }

    let result = state
        .client
        .get(format!("{}/api/tags", state.ollama_url))
        .send()
        .await;

    let model_names = match result {
        Ok(response) if response.status().is_success() => {
            response
                .json::<OllamaTagsResponse>()
                .await
                .ok()
                .and_then(|tags| tags.models)
                .map(|list| list.into_iter().map(|m| m.name).filter(|n| !n.is_empty()).collect::<Vec<_>>())
                .filter(|v| !v.is_empty())
                .unwrap_or_default()
        }
        _ => Vec::new(),
    };

    Json(ModelsResponse { models: model_names })
}

async fn chat(
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<ChatRequest>,
) -> Result<Json<ChatReply>, (StatusCode, Json<ErrorResponse>)> {
    if request.messages.is_empty() {
        return Err(bad_request("Please send at least one message."));
    }

    if !request
        .messages
        .iter()
        .any(|message| message.role.eq_ignore_ascii_case("user") && !message.content.trim().is_empty())
    {
        return Err(bad_request("At least one non-empty user message is required."));
    }

    let model = request
        .model
        .as_deref()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(&state.model)
        .to_string();


    // Speed mode tuning
    let (temperature, top_p, top_k, repeat_penalty, max_tokens) = match request.speed_mode.as_deref() {
        Some("fast") => (0.2, 0.8, 10, 1.1, 256),
        Some("quality") => (0.8, 1.0, 40, 1.0, 1024),
        _ => (0.5, 0.95, 20, 1.0, 512), // balanced
    };

    // Accelerator: set env for new Ollama launches (not for running daemon)
    // Use first enabled accelerator from array, fallback to legacy single value
    let chosen_accel = request
        .accelerators
        .as_ref()
        .and_then(|v| v.first().cloned())
        .or_else(|| request.accelerator.clone());
    if let Some(accel) = chosen_accel {
        unsafe {
            std::env::set_var("OLLAMA_ACCELERATOR", &accel);
        }
    }

    let internet_access = request.internet_access.unwrap_or(true);
    let mut content = if state.use_mock {
        build_mock_reply(&request.messages)
    } else {
        fetch_ollama_reply_tuned(&state, &request.messages, &model, temperature, top_p, top_k, repeat_penalty, max_tokens, internet_access).await?
    };

    // Fallback: if the first answer is unhelpful, enrich context from internet and retry once.
    if internet_access && is_unhelpful_reply(&content) {
        if let Some(user_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role.eq_ignore_ascii_case("user"))
        {
            if let Some(internet_context) = try_fetch_relevant_context(&state.client, &user_message.content).await {
                let insert_index = request
                    .messages
                    .iter()
                    .rposition(|message| message.role.eq_ignore_ascii_case("user"))
                    .unwrap_or(0);

                request.messages.insert(
                    insert_index,
                    ChatMessage {
                        role: "system".to_string(),
                        content: format!(
                            "[Internet context for better answer]\n{}",
                            internet_context
                        ),
                    },
                );

                content = if state.use_mock {
                    build_mock_reply(&request.messages)
                } else {
                    fetch_ollama_reply_tuned(
                        &state,
                        &request.messages,
                        &model,
                        temperature,
                        top_p,
                        top_k,
                        repeat_penalty,
                        max_tokens,
                        internet_access,
                    )
                    .await?
                };
            }
        }
    }

    Ok(Json(ChatReply {
        role: "assistant",
        content,
        mode: state.mode(),
        model,
    }))
}

fn is_unhelpful_reply(reply: &str) -> bool {
    let lower = reply.trim().to_lowercase();
    lower.is_empty()
        || lower.contains("i don't know")
        || lower.contains("i am not sure")
        || lower.contains("i'm not sure")
        || lower.contains("i cannot")
        || lower.contains("i can't")
        || lower.contains("not enough information")
}

async fn try_fetch_relevant_context(client: &Client, user_content: &str) -> Option<String> {
    if let Some(url) = extract_first_url(user_content) {
        return fetch_url_snippet(client, url).await;
    }
    fetch_search_snippet(client, user_content).await
}

fn extract_first_url(text: &str) -> Option<&str> {
    let regex = Regex::new(r"https?://\\S+").ok()?;
    regex.find(text).map(|m| m.as_str())
}

async fn fetch_url_snippet(client: &Client, url: &str) -> Option<String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;
    let snippet = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(12)
        .collect::<Vec<_>>()
        .join(" ");

    if snippet.is_empty() {
        None
    } else {
        Some(format!(
            "Source: {}\\nSnippet: {}",
            url,
            snippet.chars().take(1200).collect::<String>()
        ))
    }
}

async fn fetch_search_snippet(client: &Client, query: &str) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }

    let encoded_query = trimmed.split_whitespace().collect::<Vec<_>>().join("+");
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        encoded_query
    );

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let payload: serde_json::Value = response.json().await.ok()?;
    let abstract_text = payload
        .get("AbstractText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let heading = payload
        .get("Heading")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if abstract_text.is_empty() {
        None
    } else if heading.is_empty() {
        Some(format!("Search context: {}", abstract_text))
    } else {
        Some(format!("Search context ({}): {}", heading, abstract_text))
    }
}

async fn fetch_ollama_reply_tuned(
    state: &AppState,
    messages: &[ChatMessage],
    model: &str,
    temperature: f32,
    top_p: f32,
    top_k: u32,
    repeat_penalty: f32,
    max_tokens: u32,
    internet_access: bool,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {

    // If internet access is disabled, block any user/system message that looks like a web request
    if !internet_access {
        let forbidden = messages.iter().any(|m| m.content.contains("http://") || m.content.contains("https://") || m.content.to_lowercase().contains("fetch ") || m.content.to_lowercase().contains("curl "));
        if forbidden {
            return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "Internet access is disabled for this conversation.".to_string() })));
        }
    }

    let normalized_messages = messages
        .iter()
        .map(|message| json!({ "role": message.role.to_lowercase(), "content": message.content }))
        .collect::<Vec<_>>();

    let response = state
        .client
        .post(format!("{}/api/chat", state.ollama_url))
        .json(&json!({
            "model": model,
            "stream": false,
            "messages": normalized_messages,
            "options": {
                "temperature": temperature,
                "top_p": top_p,
                "top_k": top_k,
                "repeat_penalty": repeat_penalty,
                "num_predict": max_tokens
            }
        }))
        .send()
        .await
        .map_err(|error| service_unavailable(format!("Unable to reach local Ollama instance: {error}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status == StatusCode::NOT_FOUND && body.to_lowercase().contains("not found") {
            return Err(service_unavailable(format!(
                "Selected model '{}' is not installed in Ollama. Run 'ollama list' and choose an available model from the GUI.",
                model
            )));
        }

        return Err(service_unavailable(format!(
            "Ollama responded with {}: {}",
            status, body
        )));
    }

    let payload: OllamaResponse = response
        .json()
        .await
        .map_err(|error| service_unavailable(format!("Unable to decode Ollama response: {error}")))?;

    let content = payload
        .message
        .and_then(|message| message.content)
        .unwrap_or_default()
        .trim()
        .to_string();

    if content.is_empty() {
        return Err(service_unavailable(
            "Ollama returned an empty response.".to_string(),
        ));
    }

    Ok(content)
}

fn build_mock_reply(messages: &[ChatMessage]) -> String {
    let last_user_message = messages
        .iter()
        .rev()
        .find(|message| message.role.eq_ignore_ascii_case("user"))
        .map(|message| message.content.trim())
        .unwrap_or("Hello");

    format!(
        "[Mock Rust] You said: \"{last_user_message}\". Conversation length: {} message(s).",
        messages.len()
    )
}

fn static_text(contents: &'static str, content_type: &'static str) -> Response {
    let mut response = contents.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type),
    );
    response
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}

fn service_unavailable(message: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse { error: message }),
    )
}

impl AppState {
    fn mode(&self) -> &'static str {
        if self.use_mock {
            "mock"
        } else {
            "ollama"
        }
    }
}
