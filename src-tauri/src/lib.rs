// Peregrine — local-first AI coworker (Aerie suite).
//
// The web UI is CSP-sandboxed (connect-src 'self') and cannot reach the network.
// Every outbound call goes through model::send, which records it to the egress
// log first. Keep that boundary intact — it's the whole trust story.

mod egress;
mod listener;
mod model;
mod settings;
mod vault;

use egress::{ActivityEntry, ActivityLog};
use listener::ListenerState;
use model::Msg;
use serde::Serialize;
use settings::Settings;
use vault::{Event, VaultState};

#[derive(Serialize)]
pub struct Reply {
    pub text: String,
    pub source: String,
}

#[derive(Serialize)]
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
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

fn debrief_prompt(s: &Settings, context: &[String]) -> String {
    let list = if context.is_empty() {
        "(nothing specific captured today)".to_string()
    } else {
        context.iter().map(|c| format!("- {c}")).collect::<Vec<_>>().join("\n")
    };
    format!(
        "You are Peregrine, running the user's nightly debrief as a supportive senior {prof}. \
Here is what they captured today:\n{list}\n\n\
Run a short, focused debrief:\n\
- Go one item at a time. For anything missing a concrete outcome or metric, ask ONE specific question to draw out the impact (a number, a before/after, who benefited).\n\
- If they don't know, tell them exactly how to find it — where to look or who to ask.\n\
- When you've drawn out a stronger version, offer it as one polished bullet prefixed with 'Refined:' so they can save it.\n\
- Name the skill they demonstrated. NEVER invent a number — only use what they tell you.\n\
Keep every message short. Begin by briefly greeting them and asking about the first item worth strengthening.",
        prof = s.profession,
        list = list
    )
}

#[tauri::command]
async fn debrief_reply(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    history: Vec<Msg>,
    context: Vec<String>,
) -> Result<Reply, String> {
    let s = settings::load(&app);
    let key = settings::get_api_key().ok_or("No model connected yet. Add your API key in Settings.")?;
    let sys = debrief_prompt(&s, &context);
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text.map(|text| Reply {
        text,
        source: format!("debrief · {}", s.model),
    })
}

fn resume_prompt(s: &Settings, base: &str, job: &str) -> String {
    let mut p = format!(
        "You are Peregrine, helping a {prof} turn their accomplishments into strong résumé bullets.\n\
Rules:\n\
- Outcome-first, quantified, strong action verbs.\n\
- Use ONLY facts present in the accomplishments (and the base résumé if given). NEVER invent a metric, number, or detail. If an accomplishment has no number, phrase it honestly without inventing one.\n\
- Return 4–8 concise bullets, each on its own line starting with '• '.",
        prof = s.profession
    );
    if !base.trim().is_empty() {
        p.push_str(&format!("\n\nTheir current résumé, to build on and improve:\n{base}"));
    }
    if !job.trim().is_empty() {
        p.push_str(&format!("\n\nTailor the bullets to this job description:\n{job}"));
    }
    p
}

#[tauri::command]
async fn render_resume(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    accomplishments: Vec<String>,
    base: String,
    job: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = settings::get_api_key().ok_or("No model connected yet. Add your API key in Settings.")?;
    let sys = resume_prompt(&s, &base, &job);
    let acc = if accomplishments.is_empty() {
        "(none captured yet)".to_string()
    } else {
        accomplishments.iter().map(|a| format!("- {a}")).collect::<Vec<_>>().join("\n")
    };
    let user = format!("Here are my accomplishments:\n{acc}\n\nWrite my résumé bullets.");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text
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

// ---- vault ----

#[tauri::command]
fn vault_status(app: tauri::AppHandle, vault: tauri::State<'_, VaultState>) -> VaultStatus {
    let unlocked = vault.0.lock().map(|g| g.is_some()).unwrap_or(false);
    VaultStatus { exists: vault::exists(&app), unlocked }
}

#[tauri::command]
fn create_vault(app: tauri::AppHandle, vault: tauri::State<'_, VaultState>, passphrase: String) -> Result<(), String> {
    let path = vault::vault_path(&app)?;
    let conn = vault::create(&path, &passphrase)?;
    *vault.0.lock().map_err(|e| e.to_string())? = Some(conn);
    let _ = settings::set_vault_passphrase(&passphrase);
    Ok(())
}

#[tauri::command]
fn unlock_vault(app: tauri::AppHandle, vault: tauri::State<'_, VaultState>, passphrase: String) -> Result<(), String> {
    let path = vault::vault_path(&app)?;
    let conn = vault::unlock(&path, &passphrase)?;
    *vault.0.lock().map_err(|e| e.to_string())? = Some(conn);
    let _ = settings::set_vault_passphrase(&passphrase);
    Ok(())
}

#[tauri::command]
fn try_auto_unlock(app: tauri::AppHandle, vault: tauri::State<'_, VaultState>) -> bool {
    if !vault::exists(&app) {
        return false;
    }
    if vault.0.lock().map(|g| g.is_some()).unwrap_or(false) {
        return true;
    }
    let Some(pass) = settings::get_vault_passphrase() else { return false };
    let Ok(path) = vault::vault_path(&app) else { return false };
    match vault::unlock(&path, &pass) {
        Ok(conn) => {
            if let Ok(mut g) = vault.0.lock() {
                *g = Some(conn);
            }
            true
        }
        Err(_) => false,
    }
}

#[tauri::command]
fn lock_vault(vault: tauri::State<'_, VaultState>) {
    if let Ok(mut g) = vault.0.lock() {
        *g = None;
    }
}

#[tauri::command]
fn add_event(vault: tauri::State<'_, VaultState>, kind: String, payload: serde_json::Value) -> Result<Event, String> {
    let g = vault.0.lock().map_err(|e| e.to_string())?;
    let conn = g.as_ref().ok_or("Vault is locked.")?;
    vault::add_event(conn, &kind, &payload)
}

#[tauri::command]
fn list_events(vault: tauri::State<'_, VaultState>, limit: i64) -> Result<Vec<Event>, String> {
    let g = vault.0.lock().map_err(|e| e.to_string())?;
    let conn = g.as_ref().ok_or("Vault is locked.")?;
    vault::list_events(conn, limit)
}

#[tauri::command]
fn export_vault(app: tauri::AppHandle, dest: String) -> Result<(), String> {
    let path = vault::vault_path(&app)?;
    if !path.exists() {
        return Err("No vault to export.".into());
    }
    std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn import_merge(vault: tauri::State<'_, VaultState>, src: String) -> Result<usize, String> {
    let pass = settings::get_vault_passphrase().ok_or("Vault passphrase unavailable on this machine.")?;
    let g = vault.0.lock().map_err(|e| e.to_string())?;
    let conn = g.as_ref().ok_or("Vault is locked.")?;
    vault::merge_from(conn, &std::path::PathBuf::from(src), &pass)
}

// ---- meeting listener ----

#[derive(Serialize)]
pub struct WhisperStatus {
    pub path: String,
    pub present: bool,
}

#[tauri::command]
fn whisper_status(app: tauri::AppHandle) -> WhisperStatus {
    let s = settings::load(&app);
    WhisperStatus {
        present: listener::model_present(&s.whisper_model_path),
        path: s.whisper_model_path,
    }
}

#[tauri::command]
fn listen_start(listener_state: tauri::State<'_, ListenerState>) -> Result<(), String> {
    let mut g = listener_state.0.lock().map_err(|e| e.to_string())?;
    if g.is_some() {
        return Err("Already listening.".into());
    }
    *g = Some(listener::start()?);
    Ok(())
}

#[tauri::command]
async fn listen_stop(
    app: tauri::AppHandle,
    listener_state: tauri::State<'_, ListenerState>,
) -> Result<String, String> {
    let rec = { listener_state.0.lock().map_err(|e| e.to_string())?.take() };
    let rec = rec.ok_or("Not listening.")?;
    let (samples, rate) = listener::stop_and_take(rec);
    let model = settings::load(&app).whisper_model_path;
    tauri::async_runtime::spawn_blocking(move || listener::transcribe(&samples, rate, &model))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn capture_meeting(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    vault: tauri::State<'_, VaultState>,
    transcript: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = settings::get_api_key().ok_or("No model connected yet. Add your API key in Settings.")?;
    let sys = "You are Peregrine. From this meeting transcript, produce concise notes for the user's career record. \
Output four short sections with these exact headers: Summary, Decisions, Action items, What you contributed. \
Under 'What you contributed', include only things the user themselves said or did. Use ONLY what is in the transcript — never invent anything. Keep it tight.";
    let history = [Msg { role: "user".into(), content: format!("Transcript:\n{transcript}") }];
    let result = model::send(&s, &key, sys, &history).await;
    activity.record(result.activity);
    let notes = result.text?;
    {
        let g = vault.0.lock().map_err(|e| e.to_string())?;
        if let Some(conn) = g.as_ref() {
            let _ = vault::add_event(conn, "meeting", &serde_json::json!({ "text": notes }));
        }
    }
    Ok(notes)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ActivityLog::default())
        .manage(VaultState::default())
        .manage(ListenerState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            has_api_key,
            set_api_key,
            activity_log,
            test_connection,
            send_message,
            debrief_reply,
            render_resume,
            vault_status,
            create_vault,
            unlock_vault,
            try_auto_unlock,
            lock_vault,
            add_event,
            list_events,
            export_vault,
            import_merge,
            whisper_status,
            listen_start,
            listen_stop,
            capture_meeting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
