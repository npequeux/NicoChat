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
use std::process::Command;

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

async fn restart_ollama(payload: Option<Json<RestartOllamaRequest>>) -> impl IntoResponse {
    restart_ollama_with_accel(payload.map(|p| p.0)).await
}

#[derive(Debug, Deserialize)]
struct RestartOllamaRequest {
    accelerator: Option<String>,
    accelerators: Option<Vec<String>>,
}

async fn restart_ollama_with_accel(payload: Option<RestartOllamaRequest>) -> impl IntoResponse {
    // Stop existing Ollama server process if running.
    let _ = Command::new("pkill").args(["-f", "ollama serve"]).output();

    let selected = payload
        .as_ref()
        .and_then(|p| select_effective_accelerator(p.accelerators.as_deref(), p.accelerator.as_deref()));

    if let Some(accel) = selected.as_ref() {
        unsafe {
            std::env::set_var("OLLAMA_ACCELERATOR", accel);
        }
        match accel.as_str() {
            "gpu" => {
                unsafe {
                    std::env::remove_var("OLLAMA_LLM_LIBRARY");
                    std::env::set_var("OLLAMA_VULKAN", "1");
                }
            }
            "npu" => {
                // Best-effort: request oneAPI/OpenVINO backend if available.
                unsafe {
                    std::env::set_var("OLLAMA_LLM_LIBRARY", "openvino");
                    std::env::remove_var("OLLAMA_VULKAN");
                }
            }
            _ => {
                unsafe {
                    std::env::remove_var("OLLAMA_VULKAN");
                }
            }
        }
    }

    // Start Ollama with selected accelerator env.
    let mut cmd = Command::new("ollama");
    cmd.arg("serve");
    if let Some(accel) = selected {
        cmd.env("OLLAMA_ACCELERATOR", &accel);
        match accel.as_str() {
            "gpu" => {
                cmd.env_remove("OLLAMA_LLM_LIBRARY");
                cmd.env("OLLAMA_VULKAN", "1");
            }
            "npu" => {
                cmd.env("OLLAMA_LLM_LIBRARY", "openvino");
                cmd.env_remove("OLLAMA_VULKAN");
            }
            _ => {
                cmd.env_remove("OLLAMA_VULKAN");
            }
        }
    }
    let result = cmd.spawn();
    match result {
        Ok(_) => (StatusCode::OK, "Ollama restarted".to_string()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to restart Ollama: {e}")),
    }
}

fn select_effective_accelerator(accelerators: Option<&[String]>, fallback: Option<&str>) -> Option<String> {
    if let Some(values) = accelerators {
        let has_gpu = values.iter().any(|v| v.eq_ignore_ascii_case("gpu"));
        let has_npu = values.iter().any(|v| v.eq_ignore_ascii_case("npu"));

        // GPU/NPU are mutually exclusive. If both are present, GPU wins deterministically.
        if has_gpu {
            return Some("gpu".to_string());
        }
        if has_npu {
            return Some("npu".to_string());
        }
        if values.iter().any(|v| v.eq_ignore_ascii_case("remote")) {
            return Some("remote".to_string());
        }
    }

    fallback
        .map(|value| value.trim().to_lowercase())
        .filter(|value| value == "gpu" || value == "npu" || value == "remote")
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
    accelerator: String,
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

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current_weather: Option<OpenMeteoCurrent>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature: f64,
    weathercode: i32,
    windspeed: f64,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoGeocodeResponse {
    results: Option<Vec<OpenMeteoGeocodeItem>>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoGeocodeItem {
    name: String,
    country: Option<String>,
    admin1: Option<String>,
    latitude: f64,
    longitude: f64,
    timezone: Option<String>,
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
    let accelerator = env::var("OLLAMA_ACCELERATOR").unwrap_or_else(|_| "default".to_string());
    Json(HealthResponse {
        status: "ok",
        backend: "Rust",
        model: state.model.clone(),
        mode: state.mode(),
        accelerator,
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

fn select_effective_accelerator_for_chat(
    accelerators: Option<&[String]>,
    fallback: Option<&str>,
) -> Option<String> {
    if let Some(values) = accelerators {
        let has_gpu = values.iter().any(|v| v.eq_ignore_ascii_case("gpu"));
        let has_npu = values.iter().any(|v| v.eq_ignore_ascii_case("npu"));

        if has_gpu {
            return Some("gpu".to_string());
        }
        if has_npu {
            return Some("npu".to_string());
        }
        if values.iter().any(|v| v.eq_ignore_ascii_case("remote")) {
            return Some("remote".to_string());
        }
    }

    fallback
        .map(|value| value.trim().to_lowercase())
        .filter(|value| value == "gpu" || value == "npu" || value == "remote")
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
    let chosen_accel = select_effective_accelerator_for_chat(
        request.accelerators.as_deref(),
        request.accelerator.as_deref(),
    );
    if let Some(ref accel) = chosen_accel {
        unsafe {
            std::env::set_var("OLLAMA_ACCELERATOR", accel);
        }
    }

    let internet_access = request.internet_access.unwrap_or(true);

    // Deterministic weather fast-path: answer from live weather APIs when possible.
    let latest_user_message = request
        .messages
        .iter()
        .rev()
        .find(|message| message.role.eq_ignore_ascii_case("user"))
        .map(|m| m.content.clone())
        .unwrap_or_default();

    if internet_access && is_weather_query(&latest_user_message) {
        if let Some(city_weather) = fetch_weather_for_requested_city(&state.client, &latest_user_message).await {
            return Ok(Json(ChatReply {
                role: "assistant",
                content: format!(
                    "{city_weather}\nSource: Open-Meteo (meteo en temps reel)."
                ),
                mode: state.mode(),
                model,
            }));
        }

        if let Some(local_weather) = fetch_location_weather_context(&state.client).await {
            return Ok(Json(ChatReply {
                role: "assistant",
                content: format!(
                    "Je n'ai pas pu identifier la ville demandee avec certitude, voici la meteo locale detectee:\n{local_weather}\nSource: Open-Meteo (meteo en temps reel)."
                ),
                mode: state.mode(),
                model,
            }));
        }
    }

    // Enrich all questions with internet context when enabled.
    if internet_access {
        if let Some(user_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role.eq_ignore_ascii_case("user"))
        {
            if let Some(internet_context) =
                try_fetch_relevant_context(&state.client, &user_message.content).await
            {
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
                            "[Internet context for model]\n{}",
                            internet_context
                        ),
                    },
                );
            }
        }
    }

    let mut content = if state.use_mock {
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
            chosen_accel.as_deref(),
        )
        .await?
    };

    // Fallback: if the first answer is unhelpful, enrich context from internet and retry once.
    if internet_access && is_unhelpful_reply(&content) {
        if let Some(user_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role.eq_ignore_ascii_case("user"))
        {
            if let Some(internet_context) = try_fetch_retry_context(&state.client, &user_message.content).await {
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
                            "[Internet fallback context for accurate answer]\n{}",
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
                        chosen_accel.as_deref(),
                    )
                    .await?
                };

                // Hard guarantee: if model still returns a generic refusal, answer directly from fetched data.
                if is_unhelpful_reply(&content) {
                    content = build_grounded_answer_from_context(&internet_context);
                }
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
        || lower.contains("unable to provide real-time")
        || lower.contains("cannot provide real-time")
    || lower.contains("don't have real-time data")
    || lower.contains("do not have real-time data")
    || lower.contains("no real-time data")
    || lower.contains("last time the data was updated")
        || lower.contains("recommend checking")
        || lower.contains("check a reliable weather")
    || lower.contains("weather website or app")
        || lower.contains("je n'ai pas d'informations actualisees")
        || lower.contains("je n'ai pas d'informations actualisées")
        || lower.contains("je n'ai pas d'information actualisee")
        || lower.contains("je n'ai pas d'information actualisée")
        || lower.contains("je n'ai pas acces aux informations meteorologiques en temps reel")
        || lower.contains("je n'ai pas accès aux informations météorologiques en temps réel")
        || lower.contains("je ne peux pas fournir des informations meteorologiques en temps reel")
        || lower.contains("je ne peux pas fournir des informations météorologiques en temps réel")
        || lower.contains("en tant qu'assistant base sur le texte")
        || lower.contains("en tant qu'assistant basé sur le texte")
        || lower.contains("je vous recommande de consulter")
        || lower.contains("consulter directement les sites officiels")
        || lower.contains("rechercher les evenements locaux")
        || lower.contains("rechercher les événements locaux")
}

    fn is_weather_query(text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("meteo")
        || lower.contains("météo")
        || lower.contains("weather")
        || lower.contains("temperature")
        || lower.contains("température")
    }

fn build_grounded_answer_from_context(context: &str) -> String {
    let mut out = String::new();
    out.push_str("Voici les donnees recuperees en temps reel:\n");

    let mut lines_added = 0usize;
    for line in context.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push_str("- ");
        out.push_str(trimmed);
        out.push('\n');
        lines_added += 1;
        if lines_added >= 6 {
            break;
        }
    }

    if lines_added == 0 {
        "Je n'ai pas pu formater les donnees internet, mais la recuperation a ete tentee. Reessayez avec une question plus precise (ex: meteo a Bruxelles aujourd'hui).".to_string()
    } else {
        out.push_str("\nReponse basee sur les donnees ci-dessus.");
        out
    }
}

async fn try_fetch_retry_context(client: &Client, user_content: &str) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(base) = try_fetch_relevant_context(client, user_content).await {
        parts.push(base);
    }

    let forced_query = format!("latest {}", user_content.trim());
    if let Some(extra) = fetch_jina_search_snippet(client, &forced_query).await {
        parts.push(format!("Retry web context: {}", extra));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

async fn try_fetch_relevant_context(client: &Client, user_content: &str) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(default_location_context) = fetch_default_location_context(client).await {
        parts.push(default_location_context);
    }

    if is_weather_query(user_content) {
        if let Some(city_weather) = fetch_weather_for_requested_city(client, user_content).await {
            parts.push(city_weather);
        } else if let Some(local_context) = fetch_location_weather_context(client).await {
            parts.push(local_context);
        }
    } else if needs_location_context(user_content) {
        if let Some(local_context) = fetch_location_weather_context(client).await {
            parts.push(local_context);
        }
    }

    if let Some(url) = extract_first_url(user_content) {
        if let Some(url_context) = fetch_url_snippet(client, url).await {
            parts.push(url_context);
        }
    }

    if let Some(search_context) = fetch_search_snippet(client, user_content).await {
        parts.push(search_context);
    }

    // Always try to add a live web-search snapshot so each request has internet signals.
    if let Some(live_search_context) = fetch_jina_search_snippet(client, user_content).await {
        parts.push(format!("Live web context: {}", live_search_context));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

async fn fetch_default_location_context(client: &Client) -> Option<String> {
    let location = fetch_auto_location(client).await?;
    Some(format!(
        "Localisation automatique par defaut: {}, {}, {} (timezone: {}).",
        location.city, location.region, location.country, location.timezone
    ))
}

fn needs_location_context(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("meteo")
        || lower.contains("météo")
        || lower.contains("weather")
        || lower.contains("temperat")
        || lower.contains("today")
        || lower.contains("aujourd")
        || lower.contains("que faire")
        || lower.contains("quoi faire")
        || lower.contains("what to do")
        || lower.contains("activities")
}

fn extract_first_url(text: &str) -> Option<&str> {
    let regex = Regex::new(r"https?://\S+").ok()?;
    regex.find(text).map(|m| m.as_str())
}

fn extract_weather_city(text: &str) -> Option<String> {
    let pattern1 = Regex::new(
        r"(?i)(?:weather|meteo|météo)[^\n]*?(?:in|for|at|a|à)\s+([A-Za-zÀ-ÿ\-\s']{2,})",
    )
    .ok()?;
    if let Some(caps) = pattern1.captures(text) {
        let value = caps
            .get(1)?
            .as_str()
            .trim()
            .trim_matches(|c: char| c == '.' || c == ',' || c == '?' || c == '!');
        if let Some(clean) = sanitize_city_candidate(value) {
            return Some(clean);
        }
    }

    let pattern2 = Regex::new(r"(?i)([A-Za-zÀ-ÿ\-\s']{2,})\s+(?:weather|meteo|météo)").ok()?;
    if let Some(caps) = pattern2.captures(text) {
        let value = caps
            .get(1)?
            .as_str()
            .trim()
            .trim_matches(|c: char| c == '.' || c == ',' || c == '?' || c == '!');
        if let Some(clean) = sanitize_city_candidate(value) {
            return Some(clean);
        }
    }

    None
}

fn sanitize_city_candidate(raw: &str) -> Option<String> {
    let mut parts = raw
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| c == '.' || c == ',' || c == '?' || c == '!'))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    while let Some(last) = parts.last() {
        let lower = last.to_lowercase();
        let is_trailer = matches!(
            lower.as_str(),
            "like"
                | "today"
                | "aujourd'hui"
                | "aujourdhui"
                | "demain"
                | "tomorrow"
                | "tonight"
                | "now"
                | "please"
                | "currently"
                | "current"
                | "weather"
                | "meteo"
                | "météo"
                | "is"
                | "the"
                | "right"
        );
        if is_trailer {
            parts.pop();
        } else {
            break;
        }
    }

    if parts.is_empty() {
        return None;
    }

    if parts.len() == 1 {
        let lower = parts[0].to_lowercase();
        let is_question_word = matches!(
            lower.as_str(),
            "what"
                | "where"
                | "how"
                | "which"
                | "quel"
                | "quelle"
                | "quels"
                | "quelles"
                | "ou"
                | "où"
        );
        if is_question_word {
            return None;
        }
    }

    Some(parts.join(" "))
}

async fn geocode_city(client: &Client, city: &str) -> Option<OpenMeteoGeocodeItem> {
    let encoded_city = city.split_whitespace().collect::<Vec<_>>().join("+");
    let geocode_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={encoded_city}&count=1&language=en&format=json"
    );

    let geocode_response = client.get(geocode_url).send().await.ok()?;
    if !geocode_response.status().is_success() {
        return None;
    }

    let geocode_payload: OpenMeteoGeocodeResponse = geocode_response.json().await.ok()?;
    geocode_payload.results?.into_iter().next()
}

async fn fetch_weather_for_requested_city(client: &Client, user_content: &str) -> Option<String> {
    let city = extract_weather_city(user_content)?;
    let mut candidates = vec![city.clone()];
    if let Some(first_word) = city.split_whitespace().next() {
        if first_word != city {
            candidates.push(first_word.to_string());
        }
    }

    let mut first = None;
    for candidate in candidates {
        if let Some(found) = geocode_city(client, &candidate).await {
            first = Some(found);
            break;
        }
    }
    let first = first?;

    let weather = fetch_weather_for_location(client, first.latitude, first.longitude).await?;
    let location_label = format!(
        "{}, {}, {}",
        first.name,
        first.admin1.unwrap_or_else(|| "Unknown region".to_string()),
        first.country.unwrap_or_else(|| "Unknown country".to_string())
    );
    let activity_hint = build_activity_hint(weather.temperature, weather.weathercode);

    Some(format!(
        "Requested city weather: {location_label} (timezone: {}).\nCurrent weather: {:.1} C, wind {:.1} km/h, weather code {}.\nSuggested activity: {activity_hint}",
        first.timezone.unwrap_or_else(|| "Unknown timezone".to_string()),
        weather.temperature,
        weather.windspeed,
        weather.weathercode,
    ))
}

async fn fetch_location_weather_context(client: &Client) -> Option<String> {
    let location = fetch_auto_location(client).await?;
    let weather = fetch_weather_for_location(
        client,
        location.latitude,
        location.longitude,
    )
    .await;

    let location_label = format!(
        "{}, {}, {}",
        location.city,
        location.region,
        location.country
    );

    if let Some(weather_data) = weather {
        let activity_hint = build_activity_hint(weather_data.temperature, weather_data.weathercode);
        Some(format!(
            "Localisation automatique: {location_label} (timezone: {}).\nMeteo actuelle: {:.1} C, vent {:.1} km/h, code meteo {}.\nSuggestion locale du jour: {activity_hint}",
            location.timezone,
            weather_data.temperature,
            weather_data.windspeed,
            weather_data.weathercode,
        ))
    } else {
        Some(format!(
            "Localisation automatique: {location_label} (timezone: {}).",
            location.timezone,
        ))
    }
}

struct GeoLocation {
    city: String,
    region: String,
    country: String,
    latitude: f64,
    longitude: f64,
    timezone: String,
}

async fn fetch_auto_location(_client: &Client) -> Option<GeoLocation> {
    // Product rule: when location is not explicitly specified, default to Brussels.
    Some(GeoLocation {
        city: "Bruxelles".to_string(),
        region: "Bruxelles-Capitale".to_string(),
        country: "Belgique".to_string(),
        latitude: 50.8503,
        longitude: 4.3517,
        timezone: "Europe/Brussels".to_string(),
    })
}

async fn fetch_weather_for_location(
    client: &Client,
    latitude: f64,
    longitude: f64,
) -> Option<OpenMeteoCurrent> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude}&longitude={longitude}&current_weather=true"
    );
    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let payload: OpenMeteoResponse = response.json().await.ok()?;
    payload.current_weather
}

fn build_activity_hint(temperature: f64, weather_code: i32) -> &'static str {
    if weather_code >= 60 {
        "Pluie probable: privilegie une activite en interieur (musee, cafe, cinema)."
    } else if temperature >= 24.0 {
        "Temps chaud: bonne option pour balade, parc ou terrasse avec hydratation."
    } else if temperature <= 8.0 {
        "Temps froid: prefere une activite en interieur ou une sortie courte bien couverte."
    } else {
        "Temps plutot agreable: balade en ville, marche local ou activite exterieure legere."
    }
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

    if !abstract_text.is_empty() {
        if heading.is_empty() {
            return Some(format!("Search context: {}", abstract_text));
        }
        return Some(format!("Search context ({}): {}", heading, abstract_text));
    }

    if let Some(related) = extract_related_topics(&payload) {
        if heading.is_empty() {
            Some(format!("Search related context: {}", related))
        } else {
            Some(format!("Search related context ({}): {}", heading, related))
        }
    } else {
        fetch_jina_search_snippet(client, trimmed).await
    }
}

async fn fetch_jina_search_snippet(client: &Client, query: &str) -> Option<String> {
    let encoded_query = query.split_whitespace().collect::<Vec<_>>().join("+");
    let url = format!(
        "https://r.jina.ai/http://duckduckgo.com/?q={encoded_query}"
    );

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;
    let mut collected = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Keep lines likely to contain useful factual signals (dates/events/results).
        let lower = trimmed.to_lowercase();
        let has_signal = lower.contains("http")
            || lower.contains("result")
            || lower.contains("date")
            || lower.contains("event")
            || lower.contains("pride")
            || lower.contains("brussels")
            || lower.contains("202")
            || lower.contains("agenda")
            || lower.contains("official");

        if has_signal {
            collected.push(trimmed.to_string());
        }

        if collected.len() >= 8 {
            break;
        }
    }

    if collected.is_empty() {
        None
    } else {
        Some(format!(
            "Search web context: {}",
            collected.join(" | ")
        ))
    }
}

fn extract_related_topics(payload: &serde_json::Value) -> Option<String> {
    let topics = payload.get("RelatedTopics")?.as_array()?;
    let mut collected = Vec::new();

    for item in topics {
        if let Some(text) = item.get("Text").and_then(|v| v.as_str()) {
            if !text.trim().is_empty() {
                collected.push(text.trim().to_string());
            }
        }

        if let Some(nested) = item.get("Topics").and_then(|v| v.as_array()) {
            for nested_item in nested {
                if let Some(text) = nested_item.get("Text").and_then(|v| v.as_str()) {
                    if !text.trim().is_empty() {
                        collected.push(text.trim().to_string());
                    }
                }
            }
        }

        if collected.len() >= 3 {
            break;
        }
    }

    if collected.is_empty() {
        None
    } else {
        Some(collected.join(" | "))
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
    accelerator: Option<&str>,
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

    let mut options = serde_json::Map::new();
    options.insert("temperature".to_string(), json!(temperature));
    options.insert("top_p".to_string(), json!(top_p));
    options.insert("top_k".to_string(), json!(top_k));
    options.insert("repeat_penalty".to_string(), json!(repeat_penalty));
    options.insert("num_predict".to_string(), json!(max_tokens));

    // Make accelerator choice visible in inference behavior.
    match accelerator {
        Some("gpu") => {
            options.insert("num_gpu".to_string(), json!(999));
        }
        Some("npu") => {
            // Best effort fallback when true NPU backend is unavailable.
            options.insert("num_gpu".to_string(), json!(0));
        }
        _ => {}
    }

    let response = state
        .client
        .post(format!("{}/api/chat", state.ollama_url))
        .json(&json!({
            "model": model,
            "stream": false,
            "messages": normalized_messages,
            "options": options
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
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(header::EXPIRES, HeaderValue::from_static("0"));
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

#[cfg(test)]
mod tests {
    use super::extract_weather_city;

    #[test]
    fn test_extract_weather_city_no_city_provided() {
        assert_eq!(extract_weather_city("Quelle meteo aujourd'hui ?"), None);
        assert_eq!(extract_weather_city("weather today"), None);
    }

    #[test]
    fn test_extract_weather_city_with_city_provided() {
        assert_eq!(
            extract_weather_city("meteo a Bruxelles aujourd'hui"),
            Some("Bruxelles".to_string())
        );
        assert_eq!(
            extract_weather_city("weather in New York"),
            Some("New York".to_string())
        );
    }
}
