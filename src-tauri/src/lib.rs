// Peregrine — local-first AI coworker (Aerie suite).
//
// The web UI is CSP-sandboxed (connect-src 'self') and cannot reach the network.
// Every outbound call goes through model::send, which records it to the egress
// log first. Keep that boundary intact — it's the whole trust story.

mod egress;
mod model;
mod settings;

use egress::{ActivityEntry, ActivityLog};
use model::Msg;
use serde::Serialize;
use settings::Settings;

#[derive(Serialize)]
pub struct Reply {
    pub text: String,
    pub source: String,
}

fn system_prompt(s: &Settings) -> String {
    format!(
        "You are Peregrine, a local, private AI work companion for a {prof}. \
Act as a genuinely helpful senior {prof} ({sen}) sitting beside them as a trusted colleague — \
guide direction, review their work, think through hard parts, and produce concrete, usable artifacts. \
As you help, quietly notice accomplishments worth remembering.\n\n\
Hard rules:\n\
- NEVER fabricate facts, numbers, or metrics. If a figure or detail is unknown, ask for it or say you don't know — never invent it.\n\
- Be concise, specific, and practical. Sound like a seasoned colleague, not a chatbot.\n\
- The user always reviews and ships; you draft.",
        prof = s.profession,
        sen = s.seniority
    )
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Settings {
    settings::load(&app)
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, new_settings: Settings) -> Result<(), String> {
    settings::save(&app, &new_settings)
}

#[tauri::command]
fn has_api_key() -> bool {
    settings::has_api_key()
}

#[tauri::command]
fn set_api_key(key: String) -> Result<(), String> {
    settings::set_api_key(&key)
}

#[tauri::command]
fn activity_log(activity: tauri::State<'_, ActivityLog>) -> Vec<ActivityEntry> {
    activity.snapshot()
}

#[tauri::command]
async fn test_connection(app: tauri::AppHandle, activity: tauri::State<'_, ActivityLog>) -> Result<String, String> {
    let s = settings::load(&app);
    let key = settings::get_api_key().ok_or("No API key set. Add one above first.")?;
    let msgs = [Msg { role: "user".into(), content: "Reply with the single word: connected.".into() }];
    let result = model::send(&s, &key, "You are a connection test. Reply with one word.", &msgs).await;
    activity.record(result.activity);
    result.text.map(|_| "connected".to_string())
}

#[tauri::command]
async fn send_message(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    history: Vec<Msg>,
) -> Result<Reply, String> {
    let s = settings::load(&app);
    let key = settings::get_api_key()
        .ok_or("No model connected yet. Add your API key in Settings.")?;
    let sys = system_prompt(&s);
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text.map(|text| Reply {
        text,
        source: format!("{} · {}", s.provider, s.model),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ActivityLog::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            has_api_key,
            set_api_key,
            activity_log,
            test_connection,
            send_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
