// Bring-your-own-model client. Supports Anthropic and OpenAI-compatible
// endpoints. This is the only code that makes model calls, and it records
// every attempt to the egress log before returning.

use crate::egress::{host_allowed, host_of, now_ms, ActivityEntry};
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    pub role: String, // "user" | "assistant"
    pub content: String,
}

pub struct ModelResult {
    pub activity: ActivityEntry,
    pub text: Result<String, String>,
}

fn endpoint_url(settings: &Settings) -> String {
    let base = settings.endpoint.trim_end_matches('/');
    match settings.provider.as_str() {
        "openai" => format!("{base}/chat/completions"),
        _ => format!("{base}/v1/messages"),
    }
}

fn build_body(settings: &Settings, system: &str, messages: &[Msg]) -> Value {
    match settings.provider.as_str() {
        "openai" => {
            let mut msgs = vec![json!({ "role": "system", "content": system })];
            for m in messages {
                msgs.push(json!({ "role": m.role, "content": m.content }));
            }
            json!({ "model": settings.model, "messages": msgs })
        }
        _ => json!({
            "model": settings.model,
            "max_tokens": 1024,
            "system": system,
            "messages": messages.iter().map(|m| json!({ "role": m.role, "content": m.content })).collect::<Vec<_>>(),
        }),
    }
}

/// A single user turn that may carry an image (base64). Used for document /
/// photo analysis. Non-image documents fold their extracted text into `user_text`.
fn build_doc_body(settings: &Settings, system: &str, user_text: &str, image: &Option<(String, String)>) -> Value {
    let user_content: Value = match (settings.provider.as_str(), image) {
        ("openai", Some((mt, data))) => json!([
            { "type": "text", "text": user_text },
            { "type": "image_url", "image_url": { "url": format!("data:{mt};base64,{data}") } }
        ]),
        (_, Some((mt, data))) => json!([
            { "type": "text", "text": user_text },
            { "type": "image", "source": { "type": "base64", "media_type": mt, "data": data } }
        ]),
        _ => json!(user_text),
    };
    match settings.provider.as_str() {
        "openai" => json!({
            "model": settings.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user_content }
            ]
        }),
        _ => json!({
            "model": settings.model,
            "max_tokens": 1024,
            "system": system,
            "messages": [ { "role": "user", "content": user_content } ]
        }),
    }
}

fn extract_text(settings: &Settings, v: &Value) -> Result<String, String> {
    let text = match settings.provider.as_str() {
        "openai" => v["choices"][0]["message"]["content"].as_str().map(String::from),
        _ => v["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .filter(|s| !s.is_empty()),
    };
    text.ok_or_else(|| format!("Unexpected response shape: {v}"))
}

/// The shared egress-guarded HTTP call. Records the attempt, enforces the
/// allowlist, then POSTs the pre-built body.
async fn post(settings: &Settings, api_key: &str, url: &str, summary: String, body: Value) -> ModelResult {
    let host = host_of(url);
    let body_str = body.to_string();
    let bytes_out = body_str.len();

    let mut allowed = host_allowed(url, std::slice::from_ref(&host));
    if settings.trust_mode == "airtight" && !(host == "localhost" || host == "127.0.0.1") {
        allowed = false;
    }

    let activity = ActivityEntry {
        ts_ms: now_ms(),
        summary,
        destination: host.clone(),
        bytes_out,
        allowed,
    };

    if !allowed {
        return ModelResult {
            activity,
            text: Err(format!("Egress blocked: {host} is not permitted in {} mode.", settings.trust_mode)),
        };
    }

    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(e) => return ModelResult { activity, text: Err(e.to_string()) },
    };

    let mut req = client.post(url).header("content-type", "application/json");
    req = match settings.provider.as_str() {
        "openai" => req.bearer_auth(api_key),
        _ => req.header("x-api-key", api_key).header("anthropic-version", "2023-06-01"),
    };

    let text = async {
        let resp = req.body(body_str).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let payload = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("Model returned {status}: {payload}"));
        }
        let v: Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        extract_text(settings, &v)
    }
    .await;

    ModelResult { activity, text }
}

pub async fn send(settings: &Settings, api_key: &str, system: &str, messages: &[Msg]) -> ModelResult {
    let url = endpoint_url(settings);
    let body = build_body(settings, system, messages);
    let summary = format!("Sent a message to your model ({})", settings.model);
    post(settings, api_key, &url, summary, body).await
}

pub async fn send_doc(
    settings: &Settings,
    api_key: &str,
    system: &str,
    user_text: &str,
    image: Option<(String, String)>,
) -> ModelResult {
    let url = endpoint_url(settings);
    let body = build_doc_body(settings, system, user_text, &image);
    let summary = format!("Analyzed a document with your model ({})", settings.model);
    post(settings, api_key, &url, summary, body).await
}
