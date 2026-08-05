import { useEffect, useRef, useState } from "react";
import { Mic } from "../components/icons";
import { sendMessage, type Msg } from "../api";

const GREETING =
  "I'm Peregrine — your senior colleague. Tell me what you're working on and I'll help think it through, then quietly keep track of the wins.";

export default function Today() {
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, busy]);

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
          <div className="day">Thursday, 4 August · 6h 20m tracked · 4 wins captured</div>
        </div>
        <div className="badges">
          <div className="pill listen"><span className="d" />Listening · Design review</div>
          <div className="pill trust"><span className="d" />on-device · nothing left this machine</div>
        </div>
      </div>

      <div className="sec-label">The day's package</div>
      <div className="pkg">
        <div className="mtg">
          <div className="mtg-h">
            <Mic />
            <div>
              <div className="tt">Sprint planning</div>
              <div className="mm">45 min · 10:00 am</div>
            </div>
            <span className="tag">on-device · notes only</span>
          </div>
          <div className="mtg-grp">
            <div className="lab">Decisions</div>
            <ul>
              <li><span className="b">–</span>Ship export-to-CSV this sprint; defer SSO to next.</li>
              <li><span className="b">–</span>Cut the settings redesign from scope.</li>
            </ul>
          </div>
          <div className="mtg-grp mine">
            <div className="lab">What you contributed</div>
            <ul>
              <li><span className="b">–</span>Proposed the phased rollout that unblocked the estimate.<span className="save">saved to package</span></li>
              <li><span className="b">–</span>Reframed the SSO debate around the audit deadline.<span className="save">saved to package</span></li>
            </ul>
          </div>
        </div>

        <div className="pkg-item">
          <div className="tick">✓</div>
          <div>
            <div className="t">Reframed the onboarding epic into five user stories</div>
            <div className="m">2:10 pm · from your note<span className="chip">acceptance criteria drafted</span></div>
          </div>
        </div>
        <div className="pkg-item">
          <div className="tick gap">!</div>
          <div>
            <div className="t">Ran the roadmap review with nine stakeholders</div>
            <div className="m">9:30 am<span className="chip warn">needs an outcome</span></div>
          </div>
        </div>
      </div>

      <div className="sec-label">With your senior</div>
      <div className="chat">
        <div className="bub per"><div className="who">Peregrine</div>{GREETING}</div>
        {messages.map((m, i) =>
          m.role === "user" ? (
            <div className="bub you" key={i}>{m.content}</div>
          ) : (
            <div className="bub per" key={i}><div className="who">Peregrine</div>{m.content}</div>
          )
        )}
        {busy && <div className="bub per"><div className="who">Peregrine</div>…</div>}
        <div ref={endRef} />
      </div>

      <div className="compose">
        <textarea
          rows={2}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKey}
          placeholder="Tell your senior what you're working on…  (Enter to send)"
        />
        <button className="btn icon primary" aria-label="Send" onClick={send} disabled={busy || !input.trim()}>↑</button>
      </div>
    </div>
  );
}
