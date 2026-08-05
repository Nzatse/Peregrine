import { useEffect, useState } from "react";
import { MODES, type Mode } from "../config";
import {
  getSettings,
  saveSettings,
  hasApiKey,
  setApiKey,
  testConnection,
  inTauri,
  type Settings as S,
} from "../api";

const DEFAULTS: S = {
  provider: "anthropic",
  endpoint: "https://api.anthropic.com",
  model: "claude-sonnet-4-5",
  trust_mode: "trusted",
  profession: "Product manager",
  seniority: "Senior",
  appearance: "daylight",
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
          <div className="l">API key<small>Stored in your OS keychain — never sent anywhere but the endpoint above</small></div>
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
        <div className="set-row"><div className="l">On-device model<small>Whisper · runs locally, no audio leaves</small></div><span className="ok">Phase 7</span></div>
        <div className="set-row"><div className="l">Microphone access<small>One-time OS permission — not admin</small></div><span className="ok">Phase 7</span></div>
      </div>

      <div className="sec-label">Vault &amp; sync</div>
      <div className="set-card">
        <div className="set-row"><div className="l">Career vault<small>Encrypted with your passphrase · you own the file</small></div><span className="ok">Phase 2</span></div>
        <div className="set-row"><div className="l">Sync work ↔ home<small>Through your own encrypted cloud folder</small></div><span className="ok">Phase 8</span></div>
      </div>

      <div className="sec-label">Privacy</div>
      <div className="set-card">
        <div className="set-row"><div className="l">Telemetry<small>None — no analytics, no phone-home</small></div><span className="ok">● off by design</span></div>
        <div className="set-row"><div className="l">Activity log<small>Plain-English record of everything that left</small></div><button className="btn">View</button></div>
      </div>
    </div>
  );
}
