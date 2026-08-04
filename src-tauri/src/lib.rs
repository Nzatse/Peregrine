// Peregrine — local-first AI coworker (Aerie suite)
//
// Architecture note: the web UI is network-sandboxed by CSP (connect-src 'self'),
// so it CANNOT reach the internet on its own. Every outbound request must pass
// through a Rust command here. That makes this file the single egress chokepoint
// where the allowlist and Activity log will be enforced. Keeping that boundary
// honest is the whole trust story — see README.md.

use serde::{Deserialize, Serialize};

/// A single entry in the Activity log — a plain-English record of anything that
/// left (or would leave) the machine. In v0 nothing leaves yet; this proves the
/// shape of the audit trail the user will be able to inspect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub summary: String,
    pub destination: String,
    pub bytes_out: usize,
}

/// The coworker's reply to a user message.
///
/// v0 is a skeleton: model routing is not connected yet, so this returns an
/// honest placeholder instead of pretending to think. When the model router
/// lands, this command is where a Bring-Your-Own-Model call will be made —
/// and the ONLY place it can be made — with the egress guard checking the
/// endpoint against the allowlist before a single byte leaves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoworkerReply {
    pub text: String,
    /// Where this reply came from, shown in the UI so "you said" vs "model said"
    /// is never ambiguous. Here: the local skeleton, nothing external.
    pub source: String,
    pub egress: Option<ActivityEntry>,
}

#[tauri::command]
fn coworker_reply(message: String) -> CoworkerReply {
    // No model is wired yet. Be honest about it rather than fabricating a reply.
    let text = format!(
        "Peregrine v0 skeleton here — I received your message ({} chars), but model \
         routing isn't connected yet. Next step is the Bring-Your-Own-Model layer, \
         which will run through this exact command so every call is egress-checked \
         and logged before anything leaves your machine.",
        message.chars().count()
    );

    CoworkerReply {
        text,
        source: "local-skeleton (no network)".into(),
        egress: None, // nothing left the machine
    }
}

/// Returns the current Activity log. Empty in v0 — which is the correct, honest
/// answer: nothing has left the machine.
#[tauri::command]
fn activity_log() -> Vec<ActivityEntry> {
    Vec::new()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![coworker_reply, activity_log])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
