import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Role = "you" | "peregrine";

interface Message {
  role: Role;
  text: string;
  source?: string;
}

interface CoworkerReply {
  text: string;
  source: string;
  egress: unknown | null;
}

interface ActivityEntry {
  summary: string;
  destination: string;
  bytes_out: number;
}

const GREETING: Message = {
  role: "peregrine",
  text:
    "I'm Peregrine — your local coworker. Point me at a project and we'll review it, " +
    "figure out where it should go, and think through the hard parts together. As we " +
    "work, I quietly keep track of what you accomplish so you never have to reconstruct " +
    "it later. Everything stays on this machine.",
  source: "local-skeleton",
};

function App() {
  const [messages, setMessages] = useState<Message[]>([GREETING]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [activity, setActivity] = useState<ActivityEntry[]>([]);
  const [showActivity, setShowActivity] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  async function refreshActivity() {
    try {
      const log = await invoke<ActivityEntry[]>("activity_log");
      setActivity(log);
    } catch {
      /* ignore in skeleton */
    }
  }

  useEffect(() => {
    refreshActivity();
  }, []);

  async function send() {
    const text = input.trim();
    if (!text || busy) return;
    setInput("");
    setMessages((m) => [...m, { role: "you", text }]);
    setBusy(true);
    try {
      const reply = await invoke<CoworkerReply>("coworker_reply", { message: text });
      setMessages((m) => [
        ...m,
        { role: "peregrine", text: reply.text, source: reply.source },
      ]);
      await refreshActivity();
    } catch (e) {
      setMessages((m) => [
        ...m,
        { role: "peregrine", text: `Something went wrong: ${String(e)}`, source: "error" },
      ]);
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="mark">🦅</span>
          <div className="titles">
            <h1>Peregrine</h1>
            <span className="suite">Aerie suite · local coworker</span>
          </div>
        </div>
        <div className="trust">
          <span
            className="badge badge-local"
            title="No model connected yet — nothing leaves this machine."
          >
            ● Airtight · offline
          </span>
          <button className="activity-toggle" onClick={() => setShowActivity((s) => !s)}>
            Activity{activity.length ? ` (${activity.length})` : " · 0"}
          </button>
        </div>
      </header>

      {showActivity && (
        <div className="activity-panel">
          <div className="activity-head">
            <strong>Activity — everything that left your machine</strong>
            <span className="muted">
              Plain-English audit trail. You can watch it on the wire too.
            </span>
          </div>
          {activity.length === 0 ? (
            <p className="activity-empty">
              Nothing has left this machine. No model is connected, no telemetry, no
              network calls.
            </p>
          ) : (
            <ul>
              {activity.map((a, i) => (
                <li key={i}>
                  <span>{a.summary}</span>
                  <span className="muted">
                    → {a.destination} · {a.bytes_out} bytes
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      <main className="chat">
        {messages.map((m, i) => (
          <div key={i} className={`msg msg-${m.role}`}>
            <div className="who">{m.role === "you" ? "You" : "Peregrine"}</div>
            <div className="bubble">
              <p>{m.text}</p>
              {m.source && m.role === "peregrine" && (
                <span className="source">source: {m.source}</span>
              )}
            </div>
          </div>
        ))}
        {busy && (
          <div className="msg msg-peregrine">
            <div className="who">Peregrine</div>
            <div className="bubble thinking">…</div>
          </div>
        )}
        <div ref={endRef} />
      </main>

      <footer className="composer">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Talk to your coworker…  (Enter to send, Shift+Enter for a new line)"
          rows={2}
        />
        <button onClick={send} disabled={busy || !input.trim()}>
          Send
        </button>
      </footer>
    </div>
  );
}

export default App;
