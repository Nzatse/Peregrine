import { useEffect, useState } from "react";
import { listEvents, addEvent, renderResume, inTauri, type VaultEvent } from "../api";

function payloadText(e: VaultEvent): string {
  return (e.payload as { text?: string })?.text ?? "";
}

export default function Resume() {
  const [accomplishments, setAccomplishments] = useState<string[]>([]);
  const [base, setBase] = useState("");
  const [job, setJob] = useState("");
  const [bullets, setBullets] = useState("");
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const [baseSaved, setBaseSaved] = useState(false);

  useEffect(() => {
    if (!inTauri) return;
    listEvents(1000)
      .then((evs) => {
        setAccomplishments(evs.filter((e) => e.kind === "win").map(payloadText).filter(Boolean));
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
      setBullets(await renderResume(accomplishments, base, job));
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

  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Résumé</h1>
          <div className="day">Built from {accomplishments.length} accomplishments in your Memory</div>
        </div>
        <div className="pill trust"><span className="d" />you review · you publish</div>
      </div>

      {accomplishments.length === 0 && (
        <div className="pkg-empty">Capture some wins first — your résumé bullets are generated from the accomplishments in your Memory, never invented.</div>
      )}

      <div className="sec-label">Your existing résumé (optional)</div>
      <textarea className="ta" rows={4} placeholder="Paste your current résumé — Peregrine builds on it instead of starting blank." value={base} onChange={(e) => setBase(e.target.value)} />
      <div className="row-actions">
        <button className="btn" onClick={saveBase}>{baseSaved ? "✓ saved" : "Save baseline"}</button>
        <span className="muted-note">Stored encrypted in your vault.</span>
      </div>

      <div className="sec-label">Tailor to a job (optional)</div>
      <textarea className="ta" rows={3} placeholder="Paste a job description to tailor the bullets to it." value={job} onChange={(e) => setJob(e.target.value)} />

      <div className="row-actions">
        <button className="btn primary" onClick={generate} disabled={busy || accomplishments.length === 0}>
          {busy ? "Writing…" : "Generate résumé bullets"}
        </button>
      </div>

      {bullets && (
        <>
          <div className="sec-label">Draft bullets — edit freely</div>
          <textarea className="ta out" value={bullets} onChange={(e) => setBullets(e.target.value)} />
          <div className="row-actions">
            <button className="btn" onClick={copy}>{copied ? "✓ copied" : "Copy"}</button>
            <span className="muted-note">Peregrine drafts; you review and publish. Nothing is posted anywhere.</span>
          </div>
        </>
      )}
    </div>
  );
}
