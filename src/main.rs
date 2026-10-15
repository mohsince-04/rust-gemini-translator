mod auth;

use axum::{
    routing::get,
    extract::State,
    http::{StatusCode, Method},
    response::{Html, IntoResponse, Response},
    Json, Router,
};
use reqwest::cookie::Jar;
use reqwest::Client;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    client: Client,
    at_token: String,
    bl_version: String,
}

#[derive(Deserialize)]
struct TranslateRequest {
    text: String,
}

#[derive(Serialize)]
struct TranslateResponse {
    translation: String,
}

// Request logging middleware to print incoming requests in your terminal
async fn print_request_uri(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    println!("--> Incoming Request: {} {}", req.method(), req.uri());
    Ok(next.run(req).await)
}

async fn root_handler() -> &'static str {
    "Gemini Local Translator Server is Active"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[36m[*] Fetching live session cookies from Firefox...\x1b[0m");
    let (psid, psidts) = match auth::get_browser_cookies() {
        Ok(cookies) => cookies,
        Err(e) => {
            eprintln!("\x1b[31m[-] Error: {}\x1b[0m", e);
            std::process::exit(1);
        }
    };

    // 1. Setup cookie jar and default browser headers
    let cookie_store = Arc::new(Jar::default());
    let base_url = "https://gemini.google.com".parse::<reqwest::Url>()?;
    
    cookie_store.add_cookie_str(&format!("__Secure-1PSID={}; Domain=.google.com; Path=/;", psid), &base_url);
    cookie_store.add_cookie_str(&format!("__Secure-1PSIDTS={}; Domain=.google.com; Path=/;", psidts), &base_url);

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0".parse()?);
    headers.insert("Referer", "https://gemini.google.com/".parse()?);
    headers.insert("Origin", "https://gemini.google.com".parse()?);
    headers.insert("Content-Type", "application/x-www-form-urlencoded;charset=utf-8".parse()?);
    headers.insert("X-Same-Domain", "1".parse()?);

    let client = Client::builder()
        .cookie_provider(cookie_store)
        .default_headers(headers)
        .build()?;

    // 2. Fetch page to extract dynamic SNlM0e token (`at`) and build label (`bl`)
    println!("\x1b[36m[*] Initializing session and fetching live tokens...\x1b[0m");
    let resp_text = client.get("https://gemini.google.com/")
        .send()
        .await?
        .text()
        .await?;

    let re_snlm0e = Regex::new(r#""SNlM0e":"(.*?)""#)?;
    let at_token = re_snlm0e.captures(&resp_text)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or("Failed to locate SNlM0e token. Check your cookies.")?;

    let re_bl = Regex::new(r#"boq_gemini-web-uiserver_[0-9.]+"#)?;
    let bl_version = re_bl.find(&resp_text)
        .map(|m| m.as_str())
        .unwrap_or("boq_gemini-web-uiserver_20261011.16_p0")
        .to_string();

    println!("\x1b[32m[+] Security token & build label ({}) acquired!\x1b[0m", bl_version);

    let state = AppState {
        client,
        at_token,
        bl_version,
    };

    // 3. Setup explicit CORS configuration for browser extensions
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(tower_http::cors::Any); 

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/translate", get(handle_translation).post(handle_translation))
        .layer(axum::middleware::from_fn(print_request_uri))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("\x1b[1;32m[+] Translation Server running at http://127.0.0.1:8080/translate\x1b[0m");
    
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_translation(
    State(state): State<AppState>,
    method: Method,
    body: Option<Json<TranslateRequest>>,
) -> Result<Response, StatusCode> {
    if method == Method::GET {
        let html_content = "<html><body style='font-family: sans-serif; text-align: center; padding-top: 50px;'>\
            <h2>Gemini Translator Endpoint is Active 🚀</h2>\
            <p>This endpoint expects a <code>POST</code> request with a JSON payload from your browser extension.</p>\
            </body></html>";
        return Ok(Html(html_content).into_response());
    }

    let payload = match body {
        Some(Json(p)) => p,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    if payload.text.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let prompt = format!("Translate the following text into natural, fluent English. Output ONLY the translated text without introductory remarks or conversational filler:\n\n{}", payload.text);

    let inner_req = json!([
        [prompt, 0, null, null, null, null, 0],
        ["en"],
        [null, null, null],
        null, null, null,
        [1]
    ]);

    let f_req_inner_string = inner_req.to_string();
    let f_req_array = json!([Value::Null, f_req_inner_string]);
    let form_data = [("f.req", f_req_array.to_string())];

    let url = "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate";
    let params = [
        ("bl", state.bl_version.as_str()),
        ("rt", "c"),
        ("at", state.at_token.as_str()),
    ];

    let resp = match state.client.post(url)
        .query(&params)
        .form(&form_data)
        .send()
        .await 
    {
        Ok(r) => r,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let response_text = match resp.text().await {
        Ok(t) => t,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let clean_response = response_text.trim_start_matches(")]}'").trim();
    let mut assistant_text = String::new();

    for chunk_line in clean_response.lines() {
        let trimmed_line = chunk_line.trim();
        if trimmed_line.chars().all(|c| c.is_ascii_digit()) || trimmed_line.is_empty() {
            continue;
        }

        if let Ok(outer_val) = serde_json::from_str::<Value>(trimmed_line) {
            if let Some(payload_str) = outer_val.get(0).and_then(|v| v.as_array()).and_then(|arr| arr.get(2)).and_then(|v| v.as_str()) {
                if let Ok(inner_json) = serde_json::from_str::<Value>(payload_str) {
                    if let Some(arr_4) = inner_json.get(4).and_then(|v| v.as_array()) {
                        for candidate in arr_4 {
                            if let Some(cand_arr) = candidate.as_array() {
                                if let Some(text_list) = cand_arr.get(1).and_then(|v| v.as_array()) {
                                    if let Some(text_val) = text_list.get(0).and_then(|v| v.as_str()) {
                                        if !text_val.is_empty() && text_val.len() > assistant_text.len() {
                                            assistant_text = text_val.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if assistant_text.is_empty() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let clean_output = assistant_text.trim_end_matches('\\').trim().to_string();
    
    // Cleaned return using Json directly
    Ok(Json(TranslateResponse { translation: clean_output }).into_response())
}
