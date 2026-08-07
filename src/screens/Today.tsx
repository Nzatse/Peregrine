import { useEffect, useRef, useState } from "react";
import { sendMessage, analyzeDocument, analyzeFolder, addEvent, listEvents, whisperStatus, listenStart, listenStop, captureMeeting, inTauri, type Msg, type VaultEvent } from "../api";
import { type ScreenId } from "../config";

const GREETING =
  "I'm Peregrine — your senior colleague. Tell me what you're working on and I'll help think it through, then quietly keep track of the wins.";

function fmtTime(ts: number) {
  const d = new Date(ts);
  let h = d.getHours();
  const m = d.getMinutes();
  const ap = h < 12 ? "am" : "pm";
  h = h % 12 || 12;
  return `${h}:${m.toString().padStart(2, "0")} ${ap}`;
}

function isToday(ts: number) {
  const d = new Date(ts);
  const n = new Date();
  return d.getFullYear() === n.getFullYear() && d.getMonth() === n.getMonth() && d.getDate() === n.getDate();
}

function payloadText(e: VaultEvent): string {
  return (e.payload as { text?: string })?.text ?? "";
}

export default function Today({ go }: { go: (s: ScreenId) => void }) {
  const [events, setEvents] = useState<VaultEvent[]>([]);
  const [win, setWin] = useState("");
  const [messages, setMessages] = useState<Msg[]>([]);
  const [saved, setSaved] = useState<Set<number>>(new Set());
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [meeting, setMeeting] = useState<"idle" | "rec" | "proc">("idle");
  const [whisperReady, setWhisperReady] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const folderRef = useRef<HTMLInputElement>(null);

  async function refresh() {
    try {
      setEvents(await listEvents(100));
    } catch {
      /* vault locked or browser preview */
    }
  }

  useEffect(() => {
    if (!inTauri) return;
    refresh();
    whisperStatus().then((w) => setWhisperReady(w.present)).catch(() => {});
  }, []);

  async function startMeeting() {
    try {
      await listenStart();
      setMeeting("rec");
    } catch (e) {
      alert(String(e));
    }
  }
  async function stopMeeting() {
    setMeeting("proc");
    try {
      const transcript = await listenStop();
      if (transcript.trim()) await captureMeeting(transcript);
      await refresh();
    } catch (e) {
      alert(String(e));
    } finally {
      setMeeting("idle");
    }
  }

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, busy]);

  const today = events.filter((e) => isToday(e.ts_ms));

  async function logWin(text: string) {
    const t = text.trim();
    if (!t) return;
    try {
      await addEvent("win", { text: t });
      setWin("");
      refresh();
    } catch (e) {
      alert(String(e));
    }
  }

  async function send() {
    const text = input.trim();
    if (!text || busy) return;
    const next: Msg[] = [...messages, { role: "user", content: text }];
    setMessages(next);
    setInput("");
    setBusy(true);
    try {
      const reply = await sendMessage(next);
      setMessages((m) => [...m, { role: "assistant", content: reply.text }]);
    } catch (e) {
      setMessages((m) => [...m, { role: "assistant", content: String(e) }]);
    } finally {
      setBusy(false);
    }
  }

  async function saveMsg(i: number, text: string) {
    try {
      await addEvent("win", { text, source: "chat" });
      setSaved((s) => new Set(s).add(i));
      refresh();
    } catch (e) {
      alert(String(e));
    }
  }

  function onFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const url = String(reader.result);
      const mime = url.slice(5, url.indexOf(";")) || file.type || "application/octet-stream";
      const b64 = url.slice(url.indexOf(",") + 1);
      analyzeDoc(file.name, mime, b64);
    };
    reader.readAsDataURL(file);
    e.target.value = "";
  }

  async function analyzeDoc(name: string, mime: string, b64: string) {
    if (busy) return;
    const q = input.trim();
    setMessages((m) => [...m, { role: "user", content: q ? `📎 ${name} — ${q}` : `📎 ${name}` }]);
    setInput("");
    setBusy(true);
    try {
      const reply = await analyzeDocument(name, mime, b64, q);
      setMessages((m) => [...m, { role: "assistant", content: reply.text }]);
    } catch (e) {
      setMessages((m) => [...m, { role: "assistant", content: String(e) }]);
    } finally {
      setBusy(false);
    }
  }

  function onFolder(e: React.ChangeEvent<HTMLInputElement>) {
    const TEXT_RE = /\.(txt|md|markdown|rst|log|js|jsx|ts|tsx|mjs|cjs|json|jsonc|css|scss|less|html|htm|xml|svg|yml|yaml|toml|ini|cfg|conf|env|py|rs|go|java|kt|kts|c|h|cpp|hpp|cc|cs|rb|php|sql|sh|bash|zsh|swift|vue|svelte|dart|lua|r|jl|tex|csv|tsv|gradle|properties)$/i;
    const SKIP_RE = /(^|\/)(node_modules|\.git|target|dist|build|out|\.next|\.venv|venv|__pycache__|\.cache|coverage|\.idea|\.vscode)(\/|$)/;
    const rel = (f: File) => (f as File & { webkitRelativePath?: string }).webkitRelativePath || f.name;
    const all = Array.from(e.target.files ?? []);
    e.target.value = "";
    if (!all.length) return;
    const folderName = rel(all[0]).split("/")[0] || "folder";
    const kept = all
      .filter((f) => {
        if (SKIP_RE.test(rel(f))) return false;
        if (f.size > 200 * 1024) return false;
        return TEXT_RE.test(f.name) || /^(dockerfile|makefile|readme|license)$/i.test(f.name);
      })
      .slice(0, 60);
    if (!kept.length) {
      alert("No readable text files found in that folder.");
      return;
    }
    readFolder(folderName, kept, all.length, rel);
  }

  async function readFolder(folderName: string, files: File[], total: number, rel: (f: File) => string) {
    const MAX = 80000;
    let combined = `Files (${files.length} of ${total} shown):\n` + files.map((f) => `- ${rel(f)}`).join("\n");
    for (const f of files) {
      if (combined.length > MAX) break;
      try {
        combined += `\n\n----- ${rel(f)} -----\n${await f.text()}`;
      } catch {
        /* skip unreadable file */
      }
    }
    if (combined.length > MAX) combined = combined.slice(0, MAX) + "\n…[truncated]";
    await analyzeFolderMsg(folderName, files.length, combined);
  }

  async function analyzeFolderMsg(name: string, count: number, content: string) {
    if (busy) return;
    const q = input.trim();
    setMessages((m) => [...m, { role: "user", content: q ? `📁 ${name} (${count} files) — ${q}` : `📁 ${name} (${count} files)` }]);
    setInput("");
    setBusy(true);
    try {
      const reply = await analyzeFolder(name, content, q);
      setMessages((m) => [...m, { role: "assistant", content: reply.text }]);
    } catch (e) {
      setMessages((m) => [...m, { role: "assistant", content: String(e) }]);
    } finally {
      setBusy(false);
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
          <h1>Today</h1>
          <div className="day">
            {new Date().toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" })}
            {" · "}
            {today.length} captured today
          </div>
        </div>
        <div className="badges">
          {meeting === "rec" ? (
            <button className="btn" onClick={stopMeeting}>■ Stop &amp; save notes</button>
          ) : meeting === "proc" ? (
            <button className="btn" disabled>Transcribing…</button>
          ) : whisperReady ? (
            <button className="btn primary" onClick={startMeeting}>Start meeting</button>
          ) : (
            <button className="btn" onClick={() => go("settings")}>Set up meetings</button>
          )}
          {meeting === "rec" && <div className="pill listen"><span className="d" />Listening…</div>}
          <div className="pill trust"><span className="d" />vault encrypted · on this machine</div>
        </div>
      </div>

      <div className="sec-label">The day's package</div>
      <div className="win-add">
        <input
          className="field win-input"
          placeholder="Log a win — what did you just get done?"
          value={win}
          onChange={(e) => setWin(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && logWin(win)}
        />
        <button className="btn primary" onClick={() => logWin(win)} disabled={!win.trim()}>Log</button>
      </div>
      <div className="pkg">
        {today.length === 0 && (
          <div className="pkg-empty">Nothing captured yet today. Log a win above, or talk to your senior below and save what matters.</div>
        )}
        {today.map((e) => (
          <div className="pkg-item" key={e.id}>
            <div className="tick">✓</div>
            <div>
              <div className="t">{payloadText(e)}</div>
              <div className="m">{fmtTime(e.ts_ms)}{(e.payload as { source?: string })?.source === "chat" ? " · from your chat" : ""}</div>
            </div>
          </div>
        ))}
      </div>

      <div className="sec-label">With your senior</div>
      <div className="chat">
        <div className="bub per"><div className="who">Peregrine</div>{GREETING}</div>
        {messages.map((m, i) =>
          m.role === "user" ? (
            <div className="bub you" key={i}>
              {m.content}
              {saved.has(i) ? (
                <span className="cap done">✓ saved to package</span>
              ) : (
                <button className="cap" onClick={() => saveMsg(i, m.content)}>+ save to package</button>
              )}
            </div>
          ) : (
            <div className="bub per" key={i}><div className="who">Peregrine</div>{m.content}</div>
          )
        )}
        {busy && <div className="bub per"><div className="who">Peregrine</div>…</div>}
        <div ref={endRef} />
      </div>

      <div className="compose">
        <button className="btn icon" aria-label="Attach a document" title="Attach a document or photo" onClick={() => fileRef.current?.click()} disabled={busy}>📎</button>
        <input ref={fileRef} type="file" style={{ display: "none" }} onChange={onFile} />
        <button className="btn icon" aria-label="Attach a folder" title="Attach a folder" onClick={() => folderRef.current?.click()} disabled={busy}>📁</button>
        <input
          ref={(el) => {
            folderRef.current = el;
            if (el) {
              el.setAttribute("webkitdirectory", "");
              el.setAttribute("directory", "");
            }
          }}
          type="file"
          multiple
          style={{ display: "none" }}
          onChange={onFolder}
        />
        <textarea
          rows={2}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKey}
          placeholder="Ask, or attach a document to explain…  (Enter to send)"
        />
        <button className="btn icon primary" aria-label="Send" onClick={send} disabled={busy || !input.trim()}>↑</button>
      </div>
    </div>
  );
}
