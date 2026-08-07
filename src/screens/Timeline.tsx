import { useEffect, useState } from "react";
import { listEvents, inTauri, type VaultEvent } from "../api";

function fmtTime(ts: number) {
  const d = new Date(ts);
  let h = d.getHours();
  const m = d.getMinutes();
  const ap = h < 12 ? "am" : "pm";
  h = h % 12 || 12;
  return `${h}:${m.toString().padStart(2, "0")} ${ap}`;
}

function sameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function dayLabel(ts: number) {
  const d = new Date(ts);
  const now = new Date();
  const yest = new Date(now);
  yest.setDate(now.getDate() - 1);
  const base = d.toLocaleDateString(undefined, { weekday: "long", month: "short", day: "numeric" });
  if (sameDay(d, now)) return `Today — ${base}`;
  if (sameDay(d, yest)) return `Yesterday — ${base}`;
  return base;
}

function payloadText(e: VaultEvent): string {
  return (e.payload as { text?: string })?.text ?? "";
}

interface Group {
  key: number;
  label: string;
  items: VaultEvent[];
}

function groupByDay(events: VaultEvent[]): Group[] {
  const map = new Map<string, Group>();
  for (const e of events) {
    const d = new Date(e.ts_ms);
    const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
    if (!map.has(key)) {
      d.setHours(0, 0, 0, 0);
      map.set(key, { key: d.getTime(), label: dayLabel(e.ts_ms), items: [] });
    }
    map.get(key)!.items.push(e);
  }
  return [...map.values()].sort((a, b) => b.key - a.key);
}

export default function Timeline() {
  const [events, setEvents] = useState<VaultEvent[]>([]);

  useEffect(() => {
    if (!inTauri) return;
    // Raw conversation turns ("chat") are logged to the vault and shown in the
    // senior chat itself; keep them out of the captures timeline so it stays a
    // curated view of wins, meetings, and the like.
    listEvents(2000)
      .then((evs) => setEvents(evs.filter((e) => e.kind !== "chat")))
      .catch(() => {});
  }, []);

  const groups = groupByDay(events);

  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Timeline</h1>
          <div className="day">{events.length} captures · everything timestamped</div>
        </div>
        <div className="pill trust"><span className="d" />vault encrypted · on this machine</div>
      </div>

      {groups.length === 0 ? (
        <div className="pkg-empty">Nothing captured yet. As you log wins and talk to your senior, your work shows up here by day and time.</div>
      ) : (
        <div className="tl">
          {groups.map((g) => (
            <div className="tl-day" key={g.key}>
              <div className="dh">
                <span className="d">{g.label}</span>
                <span className="c">{g.items.length} captured</span>
              </div>
              <div className="tl-items">
                {g.items.map((e) => (
                  <div className="tl-i" key={e.id}>
                    <span className="time">{fmtTime(e.ts_ms)}</span>
                    <div className="tt">
                      {payloadText(e)}
                      {e.kind !== "win" && <span className="chip">{e.kind}</span>}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
