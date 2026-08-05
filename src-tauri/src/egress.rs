// The egress chokepoint. Every outbound request is recorded here, in plain
// English, with whether the allowlist permitted it. The web UI is CSP-sandboxed
// (connect-src 'self'), so this is the ONLY place bytes can leave the machine.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    /// Unix epoch milliseconds; the UI formats it.
    pub ts_ms: u64,
    /// Plain-English description, e.g. "Sent a message to your model".
    pub summary: String,
    /// The host it went to, e.g. "api.anthropic.com".
    pub destination: String,
    pub bytes_out: usize,
    pub allowed: bool,
}

/// In-memory activity log for the session. Managed as Tauri state.
#[derive(Default)]
pub struct ActivityLog(pub Mutex<Vec<ActivityEntry>>);

impl ActivityLog {
    pub fn record(&self, entry: ActivityEntry) {
        if let Ok(mut v) = self.0.lock() {
            v.push(entry);
        }
    }
    pub fn snapshot(&self) -> Vec<ActivityEntry> {
        self.0.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// True only if the URL's host is on the allowlist. This is what keeps
/// outbound traffic limited to exactly the endpoint the user chose.
pub fn host_allowed(url: &str, allowed_hosts: &[String]) -> bool {
    match reqwest::Url::parse(url) {
        Ok(u) => u
            .host_str()
            .map(|h| allowed_hosts.iter().any(|a| a == h))
            .unwrap_or(false),
        Err(_) => false,
    }
}

pub fn host_of(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".into())
}
