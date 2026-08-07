// Peregrine — local-first AI coworker (Aerie suite).
//
// The web UI is CSP-sandboxed (connect-src 'self') and cannot reach the network.
// Every outbound call goes through model::send, which records it to the egress
// log first. Keep that boundary intact — it's the whole trust story.
//
// No OS keychain: the API key lives encrypted inside the vault; the passphrase
// (typed on launch) is the only secret. Both are held in memory (Session) only
// while the vault is unlocked, and never persisted outside the encrypted vault.

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
use std::sync::Mutex;
use vault::{Event, VaultState};

/// In-memory session secrets — present only while the vault is unlocked.
#[derive(Default)]
pub struct Session {
    api_key: Mutex<Option<String>>,
    passphrase: Mutex<Option<String>>,
}

impl Session {
    fn api_key(&self) -> Option<String> {
        self.api_key.lock().ok().and_then(|g| g.clone())
    }
    fn passphrase(&self) -> Option<String> {
        self.passphrase.lock().ok().and_then(|g| g.clone())
    }
    fn set_api_key(&self, v: Option<String>) {
        if let Ok(mut g) = self.api_key.lock() {
            *g = v.filter(|s| !s.trim().is_empty());
        }
    }
    fn set_passphrase(&self, v: Option<String>) {
        if let Ok(mut g) = self.passphrase.lock() {
            *g = v;
        }
    }
    fn clear(&self) {
        self.set_api_key(None);
        self.set_passphrase(None);
    }
}

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
fn has_api_key(session: tauri::State<'_, Session>) -> bool {
    session.api_key().is_some()
}

#[tauri::command]
fn set_api_key(
    vault: tauri::State<'_, VaultState>,
    session: tauri::State<'_, Session>,
    key: String,
) -> Result<(), String> {
    let g = vault.0.lock().map_err(|e| e.to_string())?;
    let conn = g.as_ref().ok_or("Unlock your vault first.")?;
    vault::set_meta(conn, "api_key", &key)?;
    session.set_api_key(Some(key));
    Ok(())
}

#[tauri::command]
fn activity_log(activity: tauri::State<'_, ActivityLog>) -> Vec<ActivityEntry> {
    activity.snapshot()
}

const NO_KEY: &str = "No model connected yet. Add your API key in Settings.";

#[tauri::command]
async fn test_connection(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
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
    session: tauri::State<'_, Session>,
    history: Vec<Msg>,
    context: Vec<String>,
) -> Result<Reply, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
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
    session: tauri::State<'_, Session>,
    accomplishments: Vec<String>,
    base: String,
    job: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
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
    session: tauri::State<'_, Session>,
    history: Vec<Msg>,
) -> Result<Reply, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let sys = system_prompt(&s);
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text.map(|text| Reply {
        text,
        source: format!("{} · {}", s.provider, s.model),
    })
}

const MAX_DOC_CHARS: usize = 40_000;

#[tauri::command]
async fn analyze_document(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    name: String,
    mime: String,
    data_base64: String,
    question: String,
) -> Result<Reply, String> {
    use base64::Engine;
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let q = if question.trim().is_empty() {
        "Explain this document to me clearly, and pull out the key information.".to_string()
    } else {
        question
    };
    let sys = "You are Peregrine. The user shared a document. Read it, extract the key information, and explain it back in plain, clear language so they truly understand it — like a helpful colleague breaking it down. Answer their question. Use ONLY what is in the document; never invent details. If part of it is unclear or unreadable, say so plainly.";

    let (user_text, image) = if mime.starts_with("image/") {
        (q, Some((mime.clone(), data_base64)))
    } else {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data_base64.as_bytes())
            .map_err(|e| e.to_string())?;
        let is_pdf = mime == "application/pdf" || name.to_lowercase().ends_with(".pdf");
        let text = if is_pdf {
            pdf_extract::extract_text_from_mem(&bytes).map_err(|e| format!("Couldn't read that PDF: {e}"))?
        } else {
            String::from_utf8(bytes)
                .map_err(|_| "That file type can't be read as text. Try an image, a PDF, or a text/CSV/code file.".to_string())?
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("That document looks empty or unreadable.".into());
        }
        let clipped: String = trimmed.chars().take(MAX_DOC_CHARS).collect();
        (format!("{q}\n\nDocument \"{name}\":\n{clipped}"), None)
    };

    let result = model::send_doc(&s, &key, sys, &user_text, image).await;
    activity.record(result.activity);
    result.text.map(|text| Reply { text, source: format!("document · {}", s.model) })
}

const MAX_FOLDER_CHARS: usize = 120_000;

#[tauri::command]
async fn analyze_folder(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    name: String,
    content: String,
    question: String,
) -> Result<Reply, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let q = if question.trim().is_empty() {
        "Explain what this folder is and does, and walk me through its structure.".to_string()
    } else {
        question
    };
    let sys = format!(
        "You are Peregrine. The user shared a folder called \"{name}\". Below are its files, each preceded by its path. \
Help them understand it: explain what the folder or project is and does, describe how it's organized, surface the key points, \
and answer their question in plain, clear language. Use ONLY what is in the files — never invent. If files were skipped or the \
content was truncated, note that you're working from a partial view."
    );
    let clipped: String = content.chars().take(MAX_FOLDER_CHARS).collect();
    let user = format!("{q}\n\nFolder \"{name}\":\n{clipped}");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text.map(|text| Reply { text, source: format!("folder · {}", s.model) })
}

fn is_skipped_dir(path: &str) -> bool {
    path.split('/').any(|seg| {
        matches!(
            seg,
            "node_modules" | ".git" | "target" | "dist" | "build" | "out" | ".next"
                | ".venv" | "venv" | "__pycache__" | ".cache" | "coverage" | ".idea" | ".vscode"
        )
    })
}

fn is_text_file(path: &str) -> bool {
    const EXTS: &[&str] = &[
        ".txt", ".md", ".markdown", ".rst", ".log", ".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs",
        ".json", ".jsonc", ".css", ".scss", ".less", ".html", ".htm", ".xml", ".svg", ".yml",
        ".yaml", ".toml", ".ini", ".cfg", ".conf", ".env", ".py", ".rs", ".go", ".java", ".kt",
        ".kts", ".c", ".h", ".cpp", ".hpp", ".cc", ".cs", ".rb", ".php", ".sql", ".sh", ".bash",
        ".zsh", ".swift", ".vue", ".svelte", ".dart", ".lua", ".r", ".jl", ".tex", ".csv", ".tsv",
        ".gradle", ".properties",
    ];
    let lower = path.to_lowercase();
    let base = lower.rsplit('/').next().unwrap_or(&lower);
    EXTS.iter().any(|e| lower.ends_with(e))
        || matches!(base, "dockerfile" | "makefile" | "readme" | "license")
}

/// Unzip in memory and concatenate the readable text/code files, with per-file
/// path headers — same shape the folder analysis consumes.
fn extract_zip_text(bytes: &[u8]) -> Result<String, String> {
    use std::io::Read;
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Couldn't open the zip: {e}"))?;
    let mut names: Vec<String> = Vec::new();
    let mut body = String::new();
    for i in 0..archive.len() {
        if names.len() >= 60 || body.len() > 100_000 {
            break;
        }
        let mut f = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        if f.is_dir() {
            continue;
        }
        let path = f.name().to_string();
        if is_skipped_dir(&path) || f.size() > 200 * 1024 || !is_text_file(&path) {
            continue;
        }
        let mut text = String::new();
        if f.read_to_string(&mut text).is_err() {
            continue; // not valid UTF-8 (binary) — skip
        }
        body.push_str(&format!("\n\n----- {path} -----\n{text}"));
        names.push(path);
    }
    if names.is_empty() {
        return Err("No readable text files found in that zip.".into());
    }
    let header = format!(
        "Files ({} shown):\n{}",
        names.len(),
        names.iter().map(|n| format!("- {n}")).collect::<Vec<_>>().join("\n")
    );
    Ok(format!("{header}{body}"))
}

#[tauri::command]
async fn analyze_zip(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    name: String,
    data_base64: String,
    question: String,
) -> Result<Reply, String> {
    use base64::Engine;
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|e| e.to_string())?;
    let content = extract_zip_text(&bytes)?;
    let q = if question.trim().is_empty() {
        "Unzip this and explain what's inside — what it is, and how it's organized.".to_string()
    } else {
        question
    };
    let sys = "You are Peregrine. The user shared a .zip archive; below are the text files extracted from it, each preceded by its path. \
Explain what the archive contains — what it is and does, and how it's organized — and answer their question in plain language. \
Use ONLY what is in the files — never invent. If files were skipped or content truncated, note that you're working from a partial view.";
    let clipped: String = content.chars().take(MAX_FOLDER_CHARS).collect();
    let user = format!("{q}\n\nArchive \"{name}\":\n{clipped}");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, sys, &history).await;
    activity.record(result.activity);
    result.text.map(|text| Reply { text, source: format!("zip · {}", s.model) })
}

// ---- vault ----

#[tauri::command]
fn vault_status(app: tauri::AppHandle, vault: tauri::State<'_, VaultState>) -> VaultStatus {
    let unlocked = vault.0.lock().map(|g| g.is_some()).unwrap_or(false);
    VaultStatus { exists: vault::exists(&app), unlocked }
}

#[tauri::command]
fn create_vault(
    app: tauri::AppHandle,
    vault: tauri::State<'_, VaultState>,
    session: tauri::State<'_, Session>,
    passphrase: String,
) -> Result<(), String> {
    let path = vault::vault_path(&app)?;
    let conn = vault::create(&path, &passphrase)?;
    session.set_api_key(None);
    session.set_passphrase(Some(passphrase));
    *vault.0.lock().map_err(|e| e.to_string())? = Some(conn);
    Ok(())
}

#[tauri::command]
fn unlock_vault(
    app: tauri::AppHandle,
    vault: tauri::State<'_, VaultState>,
    session: tauri::State<'_, Session>,
    passphrase: String,
) -> Result<(), String> {
    let path = vault::vault_path(&app)?;
    let conn = vault::unlock(&path, &passphrase)?;
    session.set_api_key(vault::get_meta(&conn, "api_key"));
    session.set_passphrase(Some(passphrase));
    *vault.0.lock().map_err(|e| e.to_string())? = Some(conn);
    Ok(())
}

#[tauri::command]
fn lock_vault(vault: tauri::State<'_, VaultState>, session: tauri::State<'_, Session>) {
    if let Ok(mut g) = vault.0.lock() {
        *g = None;
    }
    session.clear();
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
fn import_merge(
    vault: tauri::State<'_, VaultState>,
    session: tauri::State<'_, Session>,
    src: String,
) -> Result<usize, String> {
    let pass = session.passphrase().ok_or("Unlock your vault first.")?;
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
fn listen_start(
    app: tauri::AppHandle,
    listener_state: tauri::State<'_, ListenerState>,
    system: Option<bool>,
) -> Result<(), String> {
    let mut g = listener_state.0.lock().map_err(|e| e.to_string())?;
    if g.is_some() {
        return Err("Already listening.".into());
    }
    // `system` lets a caller override the meeting "capture all" setting — quick
    // dictation passes Some(false) so it only ever records the user's own mic.
    let capture_system = system.unwrap_or_else(|| settings::load(&app).capture_system_audio);
    *g = Some(listener::start(capture_system)?);
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
    session: tauri::State<'_, Session>,
    vault: tauri::State<'_, VaultState>,
    transcript: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let sys = "You are Peregrine. From this meeting transcript, produce concise notes for the user's career record. \
Output four short sections with these exact headers: Summary, Decisions, Action items, What you contributed. \
Under 'What you contributed', include only things the user themselves said or did. Use ONLY what is in the transcript — never invent anything. Keep it tight.";
    let history = [Msg { role: "user".into(), content: format!("Transcript:\n{transcript}") }];
    let result = model::send(&s, &key, sys, &history).await;
    activity.record(result.activity);
    let notes = result.text?;
    {
        // Don't silently drop the notes: if the vault is locked or the write fails,
        // tell the user instead of reporting "saved" when nothing was persisted.
        let g = vault.0.lock().map_err(|e| e.to_string())?;
        let conn = g
            .as_ref()
            .ok_or("Vault is locked — unlock it to save meeting notes.")?;
        vault::add_event(conn, "meeting", &serde_json::json!({ "text": notes }))?;
    }
    Ok(notes)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    // Signed updates, checked against the public key in tauri.conf.json. The
    // update payload is fetched by Rust, not the webview, so the CSP stays shut.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .manage(ActivityLog::default())
        .manage(VaultState::default())
        .manage(ListenerState::default())
        .manage(Session::default())
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
            analyze_document,
            analyze_folder,
            analyze_zip,
            vault_status,
            create_vault,
            unlock_vault,
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
