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
    pub whisper_model_path: String,
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
            whisper_model_path: String::new(),
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
