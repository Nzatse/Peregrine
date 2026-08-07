import { useEffect, useRef, useState } from "react";
import { listEvents, debriefReply, addEvent, inTauri, type Msg, type VaultEvent } from "../api";
import Markdown from "../components/Markdown";

function isToday(ts: number) {
  const d = new Date(ts);
  const n = new Date();
  return d.getFullYear() === n.getFullYear() && d.getMonth() === n.getMonth() && d.getDate() === n.getDate();
}
function payloadText(e: VaultEvent): string {
  return (e.payload as { text?: string })?.text ?? "";
}

const SEED = "Let's review my day and strengthen what I captured.";

// The debrief conversation is logged to the vault too, so it isn't lost on reload.
// Tagged as its own thread and scoped to today (the debrief is about the day).
const DEBRIEF_THREAD = "debrief";
function chatMsg(e: VaultEvent): Msg {
  const p = e.payload as { role?: string; content?: string };
  return { role: p.role === "assistant" ? "assistant" : "user", content: p.content ?? "" };
}

export default function Debrief() {
  const [context, setContext] = useState<string[]>([]);
  const [started, setStarted] = useState(false);
  const [messages, setMessages] = useState<Msg[]>([]);
  const [saved, setSaved] = useState<Set<number>>(new Set());
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);

  function logDebrief(role: string, content: string) {
    if (!inTauri || !content.trim()) return;
    addEvent("chat", { role, content, thread: DEBRIEF_THREAD }).catch(() => {});
  }

  useEffect(() => {
    if (!inTauri) return;
    listEvents(2000)
      .then((evs) => {
        setContext(evs.filter((e) => isToday(e.ts_ms) && e.kind === "win").map(payloadText));
        // Resume today's debrief if one is already underway.
        const hist = evs
          .filter(
            (e) => e.kind === "chat" && (e.payload as { thread?: string })?.thread === DEBRIEF_THREAD && isToday(e.ts_ms),
          )
          .sort((a, b) => a.ts_ms - b.ts_ms)
          .map(chatMsg)
          .filter((m) => m.content);
        if (hist.length) {
          setMessages(hist);
          setStarted(true);
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, busy]);

  async function ask(history: Msg[]) {
    setBusy(true);
    try {
      const reply = await debriefReply(history, context);
      setMessages([...history, { role: "assistant", content: reply.text }]);
      logDebrief("assistant", reply.text);
    } catch (e) {
      setMessages([...history, { role: "assistant", content: String(e) }]);
    } finally {
      setBusy(false);
    }
  }

  async function start() {
    setStarted(true);
    const seed: Msg[] = [{ role: "user", content: SEED }];
    setMessages(seed);
    logDebrief("user", SEED);
    await ask(seed);
  }

  async function send() {
    const text = input.trim();
    if (!text || busy) return;
    const next: Msg[] = [...messages, { role: "user", content: text }];
    setMessages(next);
    setInput("");
    logDebrief("user", text);
    await ask(next);
  }

  async function saveRefined(i: number, text: string) {
    const clean = text.replace(/^\s*Refined:\s*/i, "").trim();
    try {
      await addEvent("win", { text: clean, source: "debrief" });
      setSaved((s) => new Set(s).add(i));
    } catch (e) {
      alert(String(e));
    }
  }

  function onKey(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Debrief</h1>
          <div className="day">Tonight's debrief · {context.length} to review</div>
        </div>
        <div className="pill trust"><span className="d" />vault encrypted · on this machine</div>
      </div>

      {context.length === 0 ? (
        <div className="pkg-empty">Nothing to debrief yet. Capture a few wins on Today first, then come back tonight and I'll help you strengthen them.</div>
      ) : !started ? (
        <>
          <div className="sec-label">Today's captures</div>
          <div className="pkg">
            {context.map((c, i) => (
              <div className="pkg-item" key={i}><div className="tick">✓</div><div className="t">{c}</div></div>
            ))}
          </div>
          <button className="btn primary" style={{ alignSelf: "flex-start" }} onClick={start}>☾ Start tonight's debrief</button>
        </>
      ) : (
        <>
          <div className="chat">
            {messages.slice(1).map((m, idx) => {
              const i = idx + 1;
              return m.role === "user" ? (
                <div className="bub you" key={i}>{m.content}</div>
              ) : (
                <div className="bub per" key={i}>
                  <div className="who">Peregrine · debrief</div>
                  <Markdown text={m.content} />
                  {saved.has(i) ? (
                    <span className="cap done">✓ saved to memory</span>
                  ) : (
                    <button className="cap" onClick={() => saveRefined(i, m.content)}>+ save to memory</button>
                  )}
                </div>
              );
            })}
            {busy && <div className="bub per"><div className="who">Peregrine · debrief</div>…</div>}
            <div ref={endRef} />
          </div>
          <div className="compose">
            <textarea rows={2} value={input} onChange={(e) => setInput(e.target.value)} onKeyDown={onKey}
              placeholder="Answer, or say you're not sure…  (Enter to send)" />
            <button className="btn icon primary" aria-label="Send" onClick={send} disabled={busy || !input.trim()}>↑</button>
          </div>
        </>
      )}
    </div>
  );
}
