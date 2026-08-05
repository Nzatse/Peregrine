import { useEffect, useState } from "react";
import "./theme.css";
import { NAV, ROLE, type Mode, type ScreenId } from "./config";
import { Falcon } from "./components/icons";
import { inTauri, vaultStatus, tryAutoUnlock } from "./api";
import Today from "./screens/Today";
import Meetings from "./screens/Meetings";
import Timeline from "./screens/Timeline";
import Debrief from "./screens/Debrief";
import Memory from "./screens/Memory";
import Resume from "./screens/Resume";
import Settings from "./screens/Settings";
import VaultGate from "./screens/VaultGate";

type Gate = "loading" | "onboard" | "unlock" | "ready";

function loadMode(): Mode {
  const saved = localStorage.getItem("peregrine-mode");
  if (saved === "daylight" || saved === "fieldbook" || saved === "instrument") return saved;
  return "daylight";
}

export default function App() {
  const [mode, setMode] = useState<Mode>(loadMode);
  const [screen, setScreen] = useState<ScreenId>("today");
  const [gate, setGate] = useState<Gate>(inTauri ? "loading" : "ready");

  useEffect(() => {
    localStorage.setItem("peregrine-mode", mode);
  }, [mode]);

  useEffect(() => {
    if (!inTauri) return;
    (async () => {
      try {
        const st = await vaultStatus();
        if (!st.exists) return setGate("onboard");
        if (st.unlocked) return setGate("ready");
        const ok = await tryAutoUnlock();
        setGate(ok ? "ready" : "unlock");
      } catch {
        setGate("ready");
      }
    })();
  }, []);

  return (
    <div className="app" data-mode={mode}>
      {gate === "loading" && (
        <div className="gate"><div className="gate-card"><div className="gate-mark"><Falcon /></div><p>Opening your vault…</p></div></div>
      )}

      {(gate === "onboard" || gate === "unlock") && (
        <VaultGate kind={gate} onDone={() => setGate("ready")} />
      )}

      {gate === "ready" && (
        <>
          <aside className="side">
            <div className="brand">
              <Falcon />
              <div>
                <div className="nm">Peregrine</div>
                <div className="sub">Aerie · your senior</div>
              </div>
            </div>
            <nav className="nav">
              {NAV.map((n) => (
                <button key={n.id} className={screen === n.id ? "on" : ""} onClick={() => setScreen(n.id)}>
                  <span className="dot" />
                  {n.label}
                </button>
              ))}
            </nav>
            <div className="role">
              <b>Senior {ROLE.profession} mode</b>
              Guiding you as a {ROLE.seniority.toLowerCase()} {ROLE.profession.toLowerCase()}
            </div>
          </aside>

          <main className="main">
            {screen === "today" && <Today />}
            {screen === "meetings" && <Meetings />}
            {screen === "timeline" && <Timeline />}
            {screen === "debrief" && <Debrief />}
            {screen === "memory" && <Memory />}
            {screen === "resume" && <Resume />}
            {screen === "settings" && <Settings mode={mode} setMode={setMode} />}
          </main>
        </>
      )}
    </div>
  );
}
