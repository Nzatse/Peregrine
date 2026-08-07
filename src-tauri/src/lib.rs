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

// Shared tail for the "explain this to me" prompts, so their answers come back
// skimmable instead of as a single dense block.
const FORMAT_MD: &str = " Format the answer as skimmable Markdown: open with a one-line takeaway, \
then short '- ' bullets grouped under brief '### ' headers where useful, with **bold** on key terms — never one dense block.";

fn system_prompt(s: &Settings) -> String {
    format!(
        "You are Peregrine, a local, private AI work companion for a {prof}. \
Act as a genuinely helpful senior {prof} ({sen}) sitting beside them as a trusted colleague — \
guide direction, review their work, think through hard parts, and produce concrete, usable artifacts. \
As you help, quietly notice accomplishments worth remembering.\n\n\
Hard rules:\n\
- NEVER fabricate facts, numbers, or metrics. If a figure or detail is unknown, ask for it or say you don't know — never invent it.\n\
- Be concise, specific, and practical. Sound like a seasoned colleague, not a chatbot.\n\
- The user always reviews and ships; you draft.\n\
- Format for skimming, in Markdown: open with a one-line answer, then short '- ' bullets; **bold** the key term of each; group with brief '### ' headers only when there's genuinely more than one topic. Never return one dense block of prose.",
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

// The anti-hallucination core: entries are numbered, and every generated line must
// cite the entry numbers it draws from. This lets the UI show the source behind each
// bullet and makes an invented "fact" obvious (it would cite nothing, or the wrong
// entry). Grounding in the user's own captured work is the whole point.
const CITE_RULE: &str = "Each accomplishment below is numbered like [1], [2]. \
After every bullet, cite the entry numbers it draws from in square brackets, e.g. '• … [1, 3]'. \
Every bullet MUST cite at least one entry. Use ONLY facts present in the cited entries \
(and the base résumé, if one is given) — NEVER invent a metric, number, name, date, or detail. \
If a claim can't be grounded in an entry, leave it out.";

fn output_prompt(s: &Settings, base: &str, job: &str, mode: &str) -> String {
    if mode == "review" {
        // Self-review, grounded and STAR-framed. Base/job don't apply here.
        return format!(
            "You are Peregrine, helping a {prof} write an honest performance self-review from \
their own captured accomplishments. Organize it under these exact headers, each with 2–4 bullets \
starting with '• ':\nImpact & outcomes\nStrengths demonstrated\nWhere I grew / what's next\n\
Compress STAR framing (situation, task, action, result) into tight bullets. Be honest — if the \
record is thin on outcomes, say so plainly rather than inflating. {cite}",
            prof = s.profession,
            cite = CITE_RULE
        );
    }
    let mut p = format!(
        "You are Peregrine, helping a {prof} turn their accomplishments into strong résumé bullets.\n\
Rules:\n\
- Outcome-first, quantified, strong action verbs.\n\
- Return 4–8 concise bullets, each on its own line starting with '• '.\n\
{cite}",
        prof = s.profession,
        cite = CITE_RULE
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
    mode: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let sys = output_prompt(&s, &base, &job, &mode);
    // Number the entries so the model can cite them by [n]; the UI holds the same
    // ordered list, so [n] maps straight back to the source accomplishment.
    let acc = if accomplishments.is_empty() {
        "(none captured yet)".to_string()
    } else {
        accomplishments
            .iter()
            .enumerate()
            .map(|(i, a)| format!("[{}] {}", i + 1, a))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let ask = if mode == "review" {
        "Write my self-review."
    } else {
        "Write my résumé bullets."
    };
    let user = format!("Here are my accomplishments:\n{acc}\n\n{ask}");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text
}

// ---- Structured résumé builder: section generation, scoring, cover letter, import ----
// All grounded in the user's own captured work; the same no-invention rule applies.

fn resume_section_prompt(s: &Settings, section: &str, role: &str, job: &str) -> String {
    let mut p = match section {
        "summary" => format!(
            "You are Peregrine, writing the professional summary line for a {prof}'s résumé{tgt}. \
Two or three tight sentences in résumé voice (no 'I'/'my'). Lead with scope and the strongest outcomes. {cite}",
            prof = s.profession,
            tgt = if role.trim().is_empty() { String::new() } else { format!(" targeting a {role} role") },
            cite = CITE_RULE,
        ),
        "skills" => format!(
            "You are Peregrine, extracting a {prof}'s skills from their accomplishments. \
Return 3–6 lines, each 'Category: item, item, item'. Include only skills clearly evidenced by the cited accomplishments. {cite}",
            prof = s.profession,
            cite = CITE_RULE,
        ),
        // experience / project entries → achievement bullets
        _ => format!(
            "You are Peregrine, writing strong résumé bullets for the entry \"{role}\". \
Outcome-first, quantified, strong action verbs — 3–5 bullets, each on its own line starting with '• '. {cite}",
            cite = CITE_RULE,
        ),
    };
    if !job.trim().is_empty() {
        p.push_str(&format!("\n\nTailor the wording toward this job description:\n{job}"));
    }
    p
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command: state handles + section inputs
async fn resume_section(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    section: String,
    role: String,
    job: String,
    accomplishments: Vec<String>,
    existing: Vec<String>,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let sys = resume_section_prompt(&s, &section, &role, &job);
    let acc = if accomplishments.is_empty() {
        "(none captured yet)".to_string()
    } else {
        accomplishments
            .iter()
            .enumerate()
            .map(|(i, a)| format!("[{}] {}", i + 1, a))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut user = format!("My accomplishments:\n{acc}");
    if !existing.is_empty() {
        user.push_str(&format!(
            "\n\nMy current draft to build on (keep what's already good):\n{}",
            existing.join("\n")
        ));
    }
    user.push_str("\n\nWrite it now.");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text
}

#[tauri::command]
async fn resume_score(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    resume_text: String,
    job: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let sys = "You are an ATS and hiring-manager simulator. Compare the résumé to the job description. \
Respond with ONLY a JSON object (no prose, no code fences, no markdown) of exactly this shape: \
{\"score\": <integer 0-100>, \"matched\": [<skills/keywords present in BOTH>], \"missing\": [<important JD keywords/skills absent from the résumé>], \"recommendations\": [<up to 5 short, specific fixes>]}. \
Judge only on the text given; never invent the candidate's experience.";
    let user = format!("JOB DESCRIPTION:\n{job}\n\nRÉSUMÉ:\n{resume_text}");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, sys, &history).await;
    activity.record(result.activity);
    result.text
}

#[tauri::command]
async fn cover_letter(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    resume_text: String,
    job: String,
    name: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let who = if name.trim().is_empty() { "the candidate" } else { name.trim() };
    let sys = format!(
        "You are Peregrine, writing a concise, specific cover letter for {who}, a {prof}. \
Three to four short paragraphs — warm but professional, no clichés or filler. \
Ground every claim in the résumé; NEVER invent employers, dates, or metrics. \
If a job description is given, speak directly to its top needs.",
        prof = s.profession,
    );
    let user = format!("RÉSUMÉ:\n{resume_text}\n\nJOB DESCRIPTION (may be empty):\n{job}\n\nWrite the cover letter.");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, &sys, &history).await;
    activity.record(result.activity);
    result.text
}

#[tauri::command]
async fn parse_resume(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    raw: String,
) -> Result<String, String> {
    let s = settings::load(&app);
    let key = session.api_key().ok_or(NO_KEY)?;
    let sys = "You parse a résumé's raw text into structured JSON. Respond with ONLY a JSON object \
(no prose, no code fences, no markdown) of exactly this shape:\n\
{\"profile\":{\"name\":\"\",\"title\":\"\",\"email\":\"\",\"phone\":\"\",\"location\":\"\",\"website\":\"\",\"linkedin\":\"\",\"github\":\"\"},\
\"summary\":\"\",\
\"experience\":[{\"company\":\"\",\"role\":\"\",\"location\":\"\",\"date\":\"\",\"bullets\":[\"\"],\"tech\":[\"\"]}],\
\"education\":[{\"school\":\"\",\"degree\":\"\",\"field\":\"\",\"location\":\"\",\"date\":\"\"}],\
\"skills\":[{\"category\":\"\",\"items\":[\"\"]}],\
\"projects\":[{\"name\":\"\",\"bullets\":[\"\"],\"tech\":[\"\"],\"url\":\"\"}]}\n\
Use ONLY information present in the text; leave a field empty ('' or []) if it's absent. Never invent anything.";
    let user = format!("RÉSUMÉ TEXT:\n{raw}");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, sys, &history).await;
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
    let sys = format!("You are Peregrine. The user shared a document. Read it, extract the key information, and explain it back in plain, clear language so they truly understand it — like a helpful colleague breaking it down. Answer their question. Use ONLY what is in the document; never invent details. If part of it is unclear or unreadable, say so plainly.{FORMAT_MD}");

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

    let result = model::send_doc(&s, &key, &sys, &user_text, image).await;
    activity.record(result.activity);
    result.text.map(|text| Reply { text, source: format!("document · {}", s.model) })
}

// Pull the plain text out of an uploaded file — for importing a résumé or a job
// description from a real document instead of pasting. PDF, Word (.docx), and text
// files are read locally; images (no local OCR) are transcribed by the model's
// vision. Returns raw text, not an AI summary.
#[tauri::command]
async fn extract_text(
    app: tauri::AppHandle,
    activity: tauri::State<'_, ActivityLog>,
    session: tauri::State<'_, Session>,
    name: String,
    mime: String,
    data_base64: String,
) -> Result<String, String> {
    use base64::Engine;
    let lower = name.to_lowercase();

    // Images: transcribe verbatim with the model's vision (needs an API key).
    if mime.starts_with("image/") {
        let s = settings::load(&app);
        let key = session.api_key().ok_or(NO_KEY)?;
        let sys = "Transcribe ALL text in this document image verbatim as plain text. \
Preserve line breaks and reading order. Output ONLY the transcribed text — no commentary, no markdown.";
        let result = model::send_doc(&s, &key, sys, "Transcribe this document.", Some((mime, data_base64))).await;
        activity.record(result.activity);
        return result.text;
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|e| e.to_string())?;

    let is_pdf = mime == "application/pdf" || lower.ends_with(".pdf");
    let is_docx = lower.ends_with(".docx")
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

    let text = if is_pdf {
        pdf_extract::extract_text_from_mem(&bytes).map_err(|e| format!("Couldn't read that PDF: {e}"))?
    } else if is_docx {
        extract_docx_text(&bytes)?
    } else {
        String::from_utf8(bytes).map_err(|_| {
            "That file type can't be read as text. Try a PDF, Word (.docx), an image, or a text file.".to_string()
        })?
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("That file looks empty or unreadable.".into());
    }
    Ok(trimmed.chars().take(MAX_DOC_CHARS).collect())
}

// Extract readable text from a .docx (a zip of XML): paragraphs/breaks become
// newlines, tags are stripped, and the handful of XML entities are unescaped.
fn extract_docx_text(bytes: &[u8]) -> Result<String, String> {
    use std::io::Read;
    let mut zip =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Not a valid .docx: {e}"))?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|_| "That .docx has no readable document body.".to_string())?
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;
    let xml = xml
        .replace("</w:p>", "\n")
        .replace("<w:br/>", "\n")
        .replace("<w:tab/>", "\t");
    let mut out = String::with_capacity(xml.len());
    let mut in_tag = false;
    for c in xml.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    Ok(out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'"))
}

// ---- Résumé export: write the finished résumé to a downloadable file ----

fn xml_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
fn jstr(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn jstrs(v: &serde_json::Value, k: &str) -> Vec<String> {
    v.get(k)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default()
}
fn jarr<'a>(v: &'a serde_json::Value, k: &str) -> Vec<&'a serde_json::Value> {
    v.get(k).and_then(|x| x.as_array()).map(|a| a.iter().collect()).unwrap_or_default()
}
// Drop [n] / [n, m] citation markers from a finished résumé line.
fn strip_cites(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut buf = String::new();
            let mut only_digits = true;
            let mut closed = false;
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == ']' {
                    closed = true;
                    break;
                }
                if !(nc.is_ascii_digit() || nc == ',' || nc == ' ') {
                    only_digits = false;
                }
                buf.push(nc);
            }
            if !(closed && only_digits && !buf.trim().is_empty()) {
                out.push('[');
                out.push_str(&buf);
                if closed {
                    out.push(']');
                }
            }
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
// One Word paragraph. `size` is in half-points (16pt = 32); `before` is spacing above.
fn docx_para(text: &str, bold: bool, size: u32, before: u32) -> String {
    let rpr = if bold || size > 0 {
        format!(
            "<w:rPr>{}{}</w:rPr>",
            if bold { "<w:b/>" } else { "" },
            if size > 0 { format!("<w:sz w:val=\"{size}\"/>") } else { String::new() }
        )
    } else {
        String::new()
    };
    let ppr = if before > 0 { format!("<w:pPr><w:spacing w:before=\"{before}\"/></w:pPr>") } else { String::new() };
    format!("<w:p>{ppr}<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", xml_esc(text))
}

fn build_docx(doc: &serde_json::Value) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut body = String::new();
    let empty = serde_json::json!({});
    let profile = doc.get("profile").unwrap_or(&empty);
    body.push_str(&docx_para(&jstr(profile, "name"), true, 32, 0));
    let title = jstr(profile, "title");
    if !title.is_empty() {
        body.push_str(&docx_para(&title, false, 24, 0));
    }
    let contact: Vec<String> = ["email", "phone", "location", "website", "linkedin", "github"]
        .iter()
        .map(|k| jstr(profile, k))
        .filter(|s| !s.is_empty())
        .collect();
    if !contact.is_empty() {
        body.push_str(&docx_para(&contact.join("  ·  "), false, 18, 0));
    }

    let order = jstrs(doc, "sectionOrder");
    let order = if order.is_empty() {
        vec!["summary".into(), "experience".into(), "skills".into(), "education".into(), "projects".into()]
    } else {
        order
    };
    let heading = |t: &str| docx_para(&t.to_uppercase(), true, 22, 220);

    for sec in &order {
        match sec.as_str() {
            "summary" => {
                let s = jstr(doc, "summary");
                if !s.trim().is_empty() {
                    body.push_str(&heading("Summary"));
                    body.push_str(&docx_para(&strip_cites(&s), false, 0, 40));
                }
            }
            "experience" => {
                let items = jarr(doc, "experience");
                if !items.is_empty() {
                    body.push_str(&heading("Experience"));
                    for x in items {
                        let head: Vec<String> = [jstr(x, "role"), jstr(x, "company")].into_iter().filter(|s| !s.is_empty()).collect();
                        body.push_str(&docx_para(&head.join(", "), true, 22, 120));
                        let meta: Vec<String> = [jstr(x, "location"), jstr(x, "date")].into_iter().filter(|s| !s.is_empty()).collect();
                        if !meta.is_empty() {
                            body.push_str(&docx_para(&meta.join(" · "), false, 18, 0));
                        }
                        for b in jstrs(x, "bullets") {
                            if !b.trim().is_empty() {
                                body.push_str(&docx_para(&format!("•  {}", strip_cites(&b)), false, 0, 0));
                            }
                        }
                        let tools = jstrs(x, "tools");
                        if !tools.is_empty() {
                            body.push_str(&docx_para(&format!("Tools: {}", tools.join(", ")), false, 18, 0));
                        }
                    }
                }
            }
            "skills" => {
                let items = jarr(doc, "skills");
                let any = items.iter().any(|s| !jstrs(s, "items").is_empty());
                if any {
                    body.push_str(&heading("Skills"));
                    for s in items {
                        let its = jstrs(s, "items");
                        if !its.is_empty() {
                            body.push_str(&docx_para(&format!("{}: {}", jstr(s, "category"), its.join(", ")), false, 0, 20));
                        }
                    }
                }
            }
            "education" => {
                let items = jarr(doc, "education");
                if !items.is_empty() {
                    body.push_str(&heading("Education"));
                    for e in items {
                        let deg: Vec<String> = [jstr(e, "degree"), jstr(e, "field")].into_iter().filter(|s| !s.is_empty()).collect();
                        body.push_str(&docx_para(&deg.join(", "), true, 20, 100));
                        let meta: Vec<String> = [jstr(e, "school"), jstr(e, "location"), jstr(e, "date")].into_iter().filter(|s| !s.is_empty()).collect();
                        if !meta.is_empty() {
                            body.push_str(&docx_para(&meta.join(" · "), false, 18, 0));
                        }
                    }
                }
            }
            "projects" => {
                let items = jarr(doc, "projects");
                if !items.is_empty() {
                    body.push_str(&heading("Projects"));
                    for p in items {
                        body.push_str(&docx_para(&jstr(p, "name"), true, 22, 120));
                        for b in jstrs(p, "bullets") {
                            if !b.trim().is_empty() {
                                body.push_str(&docx_para(&format!("•  {}", strip_cites(&b)), false, 0, 0));
                            }
                        }
                        let tools = jstrs(p, "tools");
                        if !tools.is_empty() {
                            body.push_str(&docx_para(&format!("Tools: {}", tools.join(", ")), false, 18, 0));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}\
<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"720\" w:right=\"720\" w:bottom=\"720\" w:left=\"720\"/></w:sectPr></w:body></w:document>"
    );
    const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
    const RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";

    let mut buf = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in [
            ("[Content_Types].xml", CONTENT_TYPES),
            ("_rels/.rels", RELS),
            ("word/document.xml", document.as_str()),
        ] {
            zw.start_file(name, opts).map_err(|e| e.to_string())?;
            zw.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        }
        zw.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

#[tauri::command]
fn export_resume(
    app: tauri::AppHandle,
    format: String,
    filename: String,
    text: String,
    doc: serde_json::Value,
) -> Result<String, String> {
    use tauri::Manager;
    let (ext, bytes): (&str, Vec<u8>) = match format.as_str() {
        "docx" => ("docx", build_docx(&doc)?),
        "txt" => ("txt", text.into_bytes()),
        "md" => ("md", text.into_bytes()),
        "html" => ("html", text.into_bytes()),
        other => return Err(format!("Unknown export format: {other}")),
    };
    // Save into the user's Downloads folder (fall back to home).
    let dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| e.to_string())?;
    let safe: String = filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect();
    let safe = safe.trim();
    let safe = if safe.is_empty() { "resume" } else { safe };
    let mut path = dir.join(format!("{safe}.{ext}"));
    let mut n = 1;
    while path.exists() {
        path = dir.join(format!("{safe} ({n}).{ext}"));
        n += 1;
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
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
content was truncated, note that you're working from a partial view.{FORMAT_MD}"
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
    let sys = format!("You are Peregrine. The user shared a .zip archive; below are the text files extracted from it, each preceded by its path. \
Explain what the archive contains — what it is and does, and how it's organized — and answer their question in plain language. \
Use ONLY what is in the files — never invent. If files were skipped or content truncated, note that you're working from a partial view.{FORMAT_MD}");
    let clipped: String = content.chars().take(MAX_FOLDER_CHARS).collect();
    let user = format!("{q}\n\nArchive \"{name}\":\n{clipped}");
    let history = [Msg { role: "user".into(), content: user }];
    let result = model::send(&s, &key, &sys, &history).await;
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
Format as Markdown with these exact section headers, each on its own line: '### Summary', '### Decisions', \
'### Action items', '### What you contributed'. Under each, use short '- ' bullet points. \
Under 'What you contributed', include only things the user themselves said or did. \
Use ONLY what is in the transcript — never invent anything. Keep it tight.";
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
            resume_section,
            resume_score,
            cover_letter,
            parse_resume,
            extract_text,
            export_resume,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_cites_removes_only_numeric_markers() {
        assert_eq!(strip_cites("Shipped the Q3 report [1, 3]"), "Shipped the Q3 report");
        assert_eq!(strip_cites("No cites here"), "No cites here");
        // Non-numeric brackets are left alone.
        assert_eq!(strip_cites("Keep [TODO] brackets"), "Keep [TODO] brackets");
    }

    #[test]
    fn docx_is_a_valid_zip_with_expected_parts() {
        use std::io::Read;
        let doc = serde_json::json!({
            "profile": { "name": "Jordan Rivera", "title": "Registered Nurse" },
            "summary": "Critical-care nurse [1].",
            "sectionOrder": ["summary", "experience"],
            "experience": [{ "role": "RN", "company": "ICU", "bullets": ["Led codes calmly [2]"], "tools": ["ACLS"] }],
        });
        let bytes = build_docx(&doc).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert!(zip.by_name("[Content_Types].xml").is_ok());
        assert!(zip.by_name("_rels/.rels").is_ok());
        let mut xml = String::new();
        zip.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        assert!(xml.contains("Jordan Rivera"));
        assert!(xml.contains("Led codes calmly"));
        assert!(!xml.contains("[2]")); // citation stripped from the finished doc
    }
}
