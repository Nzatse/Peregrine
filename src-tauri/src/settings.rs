// Non-secret settings live in a JSON file in the app config dir.
// The API key NEVER touches that file — it goes to the OS keychain.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub provider: String,   // "anthropic" | "openai"
    pub endpoint: String,   // base URL
    pub model: String,
    pub trust_mode: String, // "airtight" | "trusted" | "standard"
    pub profession: String,
    pub seniority: String,
    pub appearance: String, // "daylight" | "fieldbook" | "instrument"
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: "anthropic".into(),
            endpoint: "https://api.anthropic.com".into(),
            model: "claude-sonnet-4-5".into(),
            trust_mode: "trusted".into(),
            profession: "Product manager".into(),
            seniority: "Senior".into(),
            appearance: "daylight".into(),
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

pub fn load(app: &tauri::AppHandle) -> Settings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).map_err(|e| e.to_string()))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let p = settings_path(app)?;
    let s = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(p, s).map_err(|e| e.to_string())
}

// ---- API key in the OS keychain ----

const KEYCHAIN_SERVICE: &str = "dev.aerie.peregrine";
const KEYCHAIN_USER: &str = "model-api-key";

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).map_err(|e| e.to_string())
}

pub fn set_api_key(key: &str) -> Result<(), String> {
    let e = entry()?;
    if key.trim().is_empty() {
        let _ = e.delete_credential();
        return Ok(());
    }
    e.set_password(key).map_err(|e| e.to_string())
}

pub fn get_api_key() -> Option<String> {
    entry().ok()?.get_password().ok().filter(|k| !k.is_empty())
}

pub fn has_api_key() -> bool {
    get_api_key().is_some()
}
