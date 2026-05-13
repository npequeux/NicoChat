use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
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
        model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:1b".to_string()),
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
        .route("/api/chat", post(chat))
        .with_state(state);

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

async fn chat(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
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

    let content = if state.use_mock {
        build_mock_reply(&request.messages)
    } else {
        fetch_ollama_reply(&state, &request.messages).await?
    };

    Ok(Json(ChatReply {
        role: "assistant",
        content,
        mode: state.mode(),
        model: state.model.clone(),
    }))
}

async fn fetch_ollama_reply(
    state: &AppState,
    messages: &[ChatMessage],
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let normalized_messages = messages
        .iter()
        .map(|message| json!({ "role": message.role.to_lowercase(), "content": message.content }))
        .collect::<Vec<_>>();

    let response = state
        .client
        .post(format!("{}/api/chat", state.ollama_url))
        .json(&json!({
            "model": state.model,
            "stream": false,
            "messages": normalized_messages,
        }))
        .send()
        .await
        .map_err(|error| service_unavailable(format!("Unable to reach local Ollama instance: {error}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
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
