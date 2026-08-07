import { useEffect, useState } from "react";
import { MODES, type Mode } from "../config";
import {
  getSettings,
  saveSettings,
  hasApiKey,
  setApiKey,
  testConnection,
  exportVault,
  importMerge,
  activityLog,
  checkForUpdate,
  installUpdate,
  inTauri,
  type Settings as S,
  type ActivityEntry,
} from "../api";

const DEFAULTS: S = {
  provider: "anthropic",
  endpoint: "https://api.anthropic.com",
  model: "claude-sonnet-4-5",
  trust_mode: "trusted",
  profession: "Product manager",
  seniority: "Senior",
  appearance: "daylight",
  whisper_model_path: "",
  capture_system_audio: false,
};

const TRUST = [
  { id: "airtight", h: "Airtight", s: "Local model. Nothing leaves." },
  { id: "trusted", h: "Trusted cloud", s: "Zero-retention endpoint." },
  { id: "standard", h: "Standard cloud", s: "Consumer API terms." },
];

function inferProvider(endpoint: string) {
  return endpoint.toLowerCase().includes("anthropic") ? "anthropic" : "openai";
}

function safeHost(u: string) {
  try {
    return new URL(u).host || u;
  } catch {
    return u || "—";
  }
}

function Toggle({ initial = true }: { initial?: boolean }) {
  const [on, setOn] = useState(initial);
  return <button className={`tg ${on ? "on" : ""}`} aria-pressed={on} aria-label="Toggle" onClick={() => setOn((v) => !v)} />;
}

export default function Settings({ mode, setMode }: { mode: Mode; setMode: (m: Mode) => void }) {
  const [s, setS] = useState<S>(DEFAULTS);
  const [keyInput, setKeyInput] = useState("");
  const [keyPresent, setKeyPresent] = useState(false);
  const [conn, setConn] = useState<string>("");
  const [exportPath, setExportPath] = useState("");
  const [importPath, setImportPath] = useState("");
  const [syncMsg, setSyncMsg] = useState("");
  const [activity, setActivity] = useState<ActivityEntry[] | null>(null);
  const [update, setUpdate] = useState<{ version: string; notes?: string } | null>(null);
  const [updateMsg, setUpdateMsg] = useState("");

  async function doCheckUpdate() {
    setUpdateMsg("Checking…");
    try {
      const r = await checkForUpdate();
      if (!r.available) {
        setUpdate(null);
        setUpdateMsg("You're on the latest version.");
      } else {
        setUpdate({ version: r.version!, notes: r.notes });
        setUpdateMsg("");
      }
    } catch (e) {
      setUpdateMsg(String(e));
    }
  }

  async function doInstallUpdate() {
    setUpdateMsg("Downloading and verifying…");
    try {
      await installUpdate();
    } catch (e) {
      setUpdateMsg(String(e));
    }
  }

  async function viewActivity() {
    if (activity) {
      setActivity(null);
      return;
    }
    try {
      setActivity(await activityLog());
    } catch (e) {
      setSyncMsg(String(e));
    }
  }

  async function doExport() {
    try {
      await exportVault(exportPath);
      setSyncMsg(`Exported to ${exportPath}`);
    } catch (e) {
      setSyncMsg(String(e));
    }
  }
  async function doImport() {
    try {
      const n = await importMerge(importPath);
      setSyncMsg(`Merged ${n} new event${n === 1 ? "" : "s"}.`);
    } catch (e) {
      setSyncMsg(String(e));
    }
  }

  useEffect(() => {
    if (!inTauri) return;
    getSettings().then((loaded) => {
      setS(loaded);
      setMode(loaded.appearance as Mode);
    }).catch(() => {});
    hasApiKey().then(setKeyPresent).catch(() => {});
  }, [setMode]);

  function patch(p: Partial<S>) {
    setS((prev) => {
      const next = { ...prev, ...p };
      if (inTauri) saveSettings(next).catch(() => {});
      return next;
    });
  }

  async function saveKey() {
    if (!inTauri) return;
    try {
      await setApiKey(keyInput);
      setKeyPresent(keyInput.trim().length > 0);
      setConn("");
    } catch (e) {
      setConn(String(e));
    }
  }

  async function test() {
    setConn("testing…");
    try {
      await testConnection();
      setConn("● connected");
    } catch (e) {
      setConn(String(e));
    }
  }

  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Settings</h1>
          <div className="day">Connect your model · make Peregrine yours</div>
        </div>
        <div className="pill trust"><span className="d" />outbound: {safeHost(s.endpoint)} only</div>
      </div>

      <div className="sec-label">Model — bring your own</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">API endpoint<small>Any Anthropic- or OpenAI-compatible URL, or a local model</small></div>
          <input className="field mono" spellCheck={false} value={s.endpoint}
            onChange={(e) => patch({ endpoint: e.target.value, provider: inferProvider(e.target.value) })} />
        </div>
        <div className="set-row">
          <div className="l">Model<small>Whichever model you want to run</small></div>
          <input className="field mono" spellCheck={false} value={s.model} onChange={(e) => patch({ model: e.target.value })} />
        </div>
        <div className="set-row">
          <div className="l">API key<small>Stored encrypted inside your vault — never in an OS keychain, never sent anywhere but the endpoint above</small></div>
          <input className="field mono" type="password" spellCheck={false}
            placeholder={keyPresent ? "•••••••• saved" : "paste your key"}
            value={keyInput} onChange={(e) => setKeyInput(e.target.value)} onBlur={saveKey} />
        </div>
        <div className="set-row">
          <div className="l">Connection</div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {conn && <span className="ok">{conn}</span>}
            <button className="btn" onClick={test}>Test</button>
          </div>
        </div>
      </div>

      <div className="sec-label">Trust dial — where your work data may go</div>
      <div className="radio-cards">
        {TRUST.map((o) => (
          <button key={o.id} className={`rc ${s.trust_mode === o.id ? "on" : ""}`} onClick={() => patch({ trust_mode: o.id })}>
            <div className="rc-h">{o.h}</div>
            <div className="rc-s">{o.s}</div>
          </button>
        ))}
      </div>

      <div className="sec-label">Appearance</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Mode<small>Switch anytime; each follows system light / dark</small></div>
          <div className="mode-chips">
            {MODES.map((m) => (
              <button key={m.id} className={`mc ${mode === m.id ? "on" : ""}`}
                onClick={() => { setMode(m.id); patch({ appearance: m.id }); }}>{m.label}</button>
            ))}
          </div>
        </div>
      </div>

      <div className="sec-label">Your role</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Profession<small>Peregrine mentors as the senior version of this</small></div>
          <input className="field" value={s.profession} onChange={(e) => patch({ profession: e.target.value })} />
        </div>
        <div className="set-row">
          <div className="l">Seniority</div>
          <input className="field" value={s.seniority} onChange={(e) => patch({ seniority: e.target.value })} />
        </div>
      </div>

      <div className="sec-label">Meeting listener</div>
      <div className="set-card">
        <div className="set-row"><div className="l">Listen in meetings<small>Passive notes; you start it, always indicated</small></div><Toggle /></div>
        <div className="set-row"><div className="l">Transcribe and discard audio<small>Keep the notes, not the recording</small></div><Toggle /></div>
        <div className="set-row">
          <div className="l">Whisper model path<small>Path to a local ggml/gguf Whisper model — transcription runs on-device, no audio leaves</small></div>
          <input className="field mono" spellCheck={false} placeholder="/path/to/ggml-base.en.bin"
            value={s.whisper_model_path} onChange={(e) => patch({ whisper_model_path: e.target.value })} />
        </div>
        <div className="set-row">
          <div className="l">Capture all — include the other participants<small>Also records what you hear (the other people on the call, even through headphones) and mixes it with your mic before transcribing. Still fully on-device — nothing leaves. On macOS the OS asks for Screen&nbsp;&amp;&nbsp;System&nbsp;Audio Recording permission the first time.</small></div>
          <button className={`tg ${s.capture_system_audio ? "on" : ""}`} aria-pressed={s.capture_system_audio} aria-label="Capture all — include the other participants"
            onClick={() => patch({ capture_system_audio: !s.capture_system_audio })} />
        </div>
        <div className="set-row"><div className="l">Microphone access<small>Granted on your first listen — a one-time OS prompt, not admin</small></div><span className="ok">on first use</span></div>
      </div>

      <div className="sec-label">Vault &amp; sync</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Export vault<small>Copy your encrypted vault to a file — e.g. a folder synced to iCloud or Dropbox</small></div>
          <div className="row-actions">
            <input className="field mono" style={{ width: 180, minWidth: 0 }} placeholder="~/…/vault.peregrine" value={exportPath} onChange={(e) => setExportPath(e.target.value)} />
            <button className="btn" onClick={doExport} disabled={!exportPath.trim()}>Export</button>
          </div>
        </div>
        <div className="set-row">
          <div className="l">Import &amp; merge<small>Pull another machine's vault in — a lossless union, nothing lost even if you captured on both</small></div>
          <div className="row-actions">
            <input className="field mono" style={{ width: 180, minWidth: 0 }} placeholder="~/…/vault.peregrine" value={importPath} onChange={(e) => setImportPath(e.target.value)} />
            <button className="btn" onClick={doImport} disabled={!importPath.trim()}>Merge</button>
          </div>
        </div>
        {syncMsg && <div className="set-row"><span className="muted-note">{syncMsg}</span></div>}
      </div>

      <div className="sec-label">Updates</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Check for updates<small>Only when you ask — Peregrine never polls on its own. Updates are signature-verified before they install.</small></div>
          <button className="btn" onClick={doCheckUpdate} disabled={!inTauri}>Check</button>
        </div>
        {update && (
          <div className="set-row">
            <div className="l">Version {update.version} available{update.notes && <small>{update.notes}</small>}</div>
            <button className="btn" onClick={doInstallUpdate}>Install &amp; restart</button>
          </div>
        )}
        {updateMsg && <div className="set-row"><span className="muted-note">{updateMsg}</span></div>}
      </div>

      <div className="sec-label">Privacy</div>
      <div className="set-card">
        <div className="set-row"><div className="l">Telemetry<small>None — no analytics, no phone-home</small></div><span className="ok">● off by design</span></div>
        <div className="set-row"><div className="l">Activity log<small>Plain-English record of everything that left</small></div><button className="btn" onClick={viewActivity}>{activity ? "Hide" : "View"}</button></div>
      </div>

      {activity && (
        <div className="set-card">
          {activity.length === 0 ? (
            <div className="set-row"><span className="muted-note">Nothing has left this machine this session.</span></div>
          ) : (
            activity.map((a, i) => (
              <div className="set-row" key={i}>
                <div className="l">{a.summary}<small>{new Date(a.ts_ms).toLocaleTimeString()} · → {a.destination} · {a.bytes_out} bytes</small></div>
                <span className="ok" style={a.allowed ? undefined : { color: "var(--m-warn)" }}>{a.allowed ? "● allowed" : "● blocked"}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
