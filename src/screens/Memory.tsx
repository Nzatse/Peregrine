import { useEffect, useMemo, useState } from "react";
import { listEvents, inTauri, type VaultEvent } from "../api";

function fmtDate(ts: number) {
  return new Date(ts).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function payloadText(e: VaultEvent): string {
  return (e.payload as { text?: string })?.text ?? "";
}

function source(e: VaultEvent): string {
  return (e.payload as { source?: string })?.source ?? "";
}

export default function Memory() {
  const [events, setEvents] = useState<VaultEvent[]>([]);
  const [q, setQ] = useState("");

  useEffect(() => {
    if (!inTauri) return;
    listEvents(1000).then(setEvents).catch(() => {});
  }, []);

  const wins = useMemo(
    () => events.filter((e) => e.kind === "win" && payloadText(e).toLowerCase().includes(q.toLowerCase())),
    [events, q]
  );

  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Memory</h1>
          <div className="day">Your curated career vault · {events.filter((e) => e.kind === "win").length} accomplishments</div>
        </div>
        <div className="pill trust"><span className="d" />vault encrypted · on this machine</div>
      </div>

      <input className="field win-input" placeholder="Search your accomplishments…" value={q} onChange={(e) => setQ(e.target.value)} />

      {wins.length === 0 ? (
        <div className="pkg-empty">
          {events.length === 0
            ? "Nothing here yet. Wins you log and save become quantified accomplishments — the polished record you'd actually show someone."
            : "No accomplishments match that search."}
        </div>
      ) : (
        <div className="pkg">
          {wins.map((e) => (
            <div className="pkg-item" key={e.id}>
              <div className="tick">✓</div>
              <div>
                <div className="t">{payloadText(e)}</div>
                <div className="m">captured {fmtDate(e.ts_ms)}{source(e) === "chat" ? " · from your chat" : ""}</div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
