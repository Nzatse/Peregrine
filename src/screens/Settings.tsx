import { useState } from "react";
import { MODES, ROLE, type Mode } from "../config";

function Toggle({ initial = true }: { initial?: boolean }) {
  const [on, setOn] = useState(initial);
  return (
    <button
      className={`tg ${on ? "on" : ""}`}
      aria-pressed={on}
      aria-label="Toggle"
      onClick={() => setOn((v) => !v)}
    />
  );
}

export default function Settings({ mode, setMode }: { mode: Mode; setMode: (m: Mode) => void }) {
  const [trust, setTrust] = useState("trusted");
  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Settings</h1>
          <div className="day">Connect your model · make Peregrine yours</div>
        </div>
        <div className="pill trust"><span className="d" />outbound: api.anthropic.com only</div>
      </div>

      <div className="sec-label">Model — bring your own</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">API endpoint<small>Any Anthropic- or OpenAI-compatible URL, or a local model</small></div>
          <input className="field mono" defaultValue="https://api.anthropic.com" spellCheck={false} />
        </div>
        <div className="set-row">
          <div className="l">Model<small>Whichever model you want to run</small></div>
          <input className="field mono" defaultValue="claude-sonnet-4-5" spellCheck={false} />
        </div>
        <div className="set-row">
          <div className="l">API key<small>Stored in your OS keychain — never sent anywhere but the endpoint above</small></div>
          <input className="field mono" type="password" defaultValue="sk-ant-xxxxxxxx" spellCheck={false} />
        </div>
        <div className="set-row">
          <div className="l">Connection</div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span className="ok">● connected</span>
            <button className="btn">Test</button>
          </div>
        </div>
      </div>

      <div className="sec-label">Trust dial — where your work data may go</div>
      <div className="radio-cards">
        {[
          { id: "airtight", h: "Airtight", s: "Local model. Nothing leaves." },
          { id: "trusted", h: "Trusted cloud", s: "Zero-retention endpoint." },
          { id: "standard", h: "Standard cloud", s: "Consumer API terms." },
        ].map((o) => (
          <button key={o.id} className={`rc ${trust === o.id ? "on" : ""}`} onClick={() => setTrust(o.id)}>
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
              <button key={m.id} className={`mc ${mode === m.id ? "on" : ""}`} onClick={() => setMode(m.id)}>
                {m.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="sec-label">Your role</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Profession<small>Peregrine mentors as the senior version of this</small></div>
          <input className="field" defaultValue={ROLE.profession} />
        </div>
        <div className="set-row">
          <div className="l">Seniority</div>
          <input className="field" defaultValue={ROLE.seniority} />
        </div>
      </div>

      <div className="sec-label">Meeting listener</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Listen in meetings<small>Passive notes; you start it, always indicated</small></div>
          <Toggle />
        </div>
        <div className="set-row">
          <div className="l">Transcribe and discard audio<small>Keep the notes, not the recording</small></div>
          <Toggle />
        </div>
        <div className="set-row">
          <div className="l">On-device model<small>Whisper · runs locally, no audio leaves</small></div>
          <span className="ok">base · downloaded</span>
        </div>
        <div className="set-row">
          <div className="l">Microphone access<small>One-time OS permission — not admin</small></div>
          <span className="ok">● granted</span>
        </div>
      </div>

      <div className="sec-label">Vault &amp; sync</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Career vault<small>Encrypted with your passphrase · you own the file</small></div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn">Export</button>
            <button className="btn">Change passphrase</button>
          </div>
        </div>
        <div className="set-row">
          <div className="l">Sync work ↔ home<small>Through your own encrypted cloud folder</small></div>
          <Toggle />
        </div>
      </div>

      <div className="sec-label">Privacy</div>
      <div className="set-card">
        <div className="set-row">
          <div className="l">Telemetry<small>None — no analytics, no phone-home</small></div>
          <span className="ok">● off by design</span>
        </div>
        <div className="set-row">
          <div className="l">Activity log<small>Plain-English record of everything that left</small></div>
          <button className="btn">View</button>
        </div>
      </div>
    </div>
  );
}
