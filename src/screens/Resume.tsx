import { useEffect, useState } from "react";
import { listEvents, addEvent, renderResume, inTauri, type VaultEvent } from "../api";

function payloadText(e: VaultEvent): string {
  return (e.payload as { text?: string })?.text ?? "";
}
function fmtDate(ts: number) {
  return new Date(ts).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

type Item = { ts: number; date: string; text: string };
type Mode = "resume" | "review";

export default function Resume() {
  const [items, setItems] = useState<Item[]>([]);
  const [base, setBase] = useState("");
  const [job, setJob] = useState("");
  const [mode, setMode] = useState<Mode>("resume");
  const [bullets, setBullets] = useState("");
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const [baseSaved, setBaseSaved] = useState(false);

  useEffect(() => {
    if (!inTauri) return;
    listEvents(1000)
      .then((evs) => {
        const wins = evs
          .filter((e) => e.kind === "win")
          .map((e) => ({ ts: e.ts_ms, date: fmtDate(e.ts_ms), text: payloadText(e) }))
          .filter((it) => it.text)
          .sort((a, b) => a.ts - b.ts); // chronological, so [n] reads oldest → newest
        setItems(wins);
        const b = evs.find((e) => e.kind === "resume_base");
        if (b) setBase(payloadText(b));
      })
      .catch(() => {});
  }, []);

  async function saveBase() {
    try {
      await addEvent("resume_base", { text: base });
      setBaseSaved(true);
      setTimeout(() => setBaseSaved(false), 1500);
    } catch (e) {
      alert(String(e));
    }
  }

  async function generate() {
    setBusy(true);
    setBullets("");
    try {
      // Date-stamp each entry so citations carry a real date, and the numbering
      // here matches the [n] the model cites and the Sources list below.
      const accomplishments = items.map((it) => `${it.date} — ${it.text}`);
      setBullets(await renderResume(accomplishments, base, job, mode));
    } catch (e) {
      setBullets(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(bullets);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  const isReview = mode === "review";

  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Résumé &amp; review</h1>
          <div className="day">Built from {items.length} accomplishments in your Memory · every line cites its source</div>
        </div>
        <div className="pill trust"><span className="d" />you review · you publish</div>
      </div>

      <div className="seg">
        <button className={`seg-btn ${!isReview ? "on" : ""}`} onClick={() => setMode("resume")}>Résumé bullets</button>
        <button className={`seg-btn ${isReview ? "on" : ""}`} onClick={() => setMode("review")}>Performance self-review</button>
      </div>

      {items.length === 0 && (
        <div className="pkg-empty">
          Capture some wins first — your {isReview ? "self-review" : "résumé bullets"} are generated only from the
          accomplishments in your Memory, each one cited, never invented.
        </div>
      )}

      {!isReview && (
        <>
          <div className="sec-label">Your existing résumé (optional)</div>
          <textarea className="ta" rows={4} placeholder="Paste your current résumé — Peregrine builds on it instead of starting blank." value={base} onChange={(e) => setBase(e.target.value)} />
          <div className="row-actions">
            <button className="btn" onClick={saveBase}>{baseSaved ? "✓ saved" : "Save baseline"}</button>
            <span className="muted-note">Stored encrypted in your vault.</span>
          </div>

          <div className="sec-label">Tailor to a job (optional)</div>
          <textarea className="ta" rows={3} placeholder="Paste a job description to tailor the bullets to it." value={job} onChange={(e) => setJob(e.target.value)} />
        </>
      )}

      <div className="row-actions">
        <button className="btn primary" onClick={generate} disabled={busy || items.length === 0}>
          {busy ? "Writing…" : isReview ? "Generate self-review" : "Generate résumé bullets"}
        </button>
      </div>

      {bullets && (
        <>
          <div className="sec-label">{isReview ? "Draft self-review" : "Draft bullets"} — edit freely</div>
          <textarea className="ta out" value={bullets} onChange={(e) => setBullets(e.target.value)} />
          <div className="row-actions">
            <button className="btn" onClick={copy}>{copied ? "✓ copied" : "Copy"}</button>
            <span className="muted-note">Peregrine drafts; you review and publish. Nothing is posted anywhere.</span>
          </div>

          <div className="sec-label">Sources — what each [n] refers to</div>
          <div className="cites">
            {items.map((it, i) => (
              <div className="cite" key={i}>
                <span className="cite-n">[{i + 1}]</span>
                <span className="cite-d">{it.date}</span>
                <span className="cite-t">{it.text}</span>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
