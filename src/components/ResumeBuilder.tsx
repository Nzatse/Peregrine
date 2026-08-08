import { useEffect, useRef, useState } from "react";
import { inTauri } from "../api";
import ResumePreview from "./ResumePreview";
import {
  type ResumeDoc,
  type RSkill,
  type ScoreResult,
  blankDoc,
  blankProfile,
  blankExperience,
  blankEducation,
  blankProject,
  loadResumes,
  saveResume,
  deleteResume,
  resumeToText,
  resumeSection,
  scoreResume,
  generateCoverLetter,
  parseResumeText,
  accomplishmentsFrom,
  parseJsonLoose,
  stripCites,
  toBullets,
  exportResume,
  toMarkdown,
} from "../resume";
import { listEvents, extractText, type VaultEvent } from "../api";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

type GenTarget =
  | { kind: "summary" }
  | { kind: "skills" }
  | { kind: "experience"; i: number }
  | { kind: "project"; i: number };

function parseSkills(block: string): RSkill[] {
  const out: RSkill[] = [];
  for (const raw of block.split("\n")) {
    const line = stripCites(raw.replace(/^\s*[-*•]\s*/, ""));
    const idx = line.indexOf(":");
    if (idx === -1) continue;
    const category = line.slice(0, idx).trim();
    const items = line.slice(idx + 1).split(",").map((s) => s.trim()).filter(Boolean);
    if (category && items.length) out.push({ category, items });
  }
  return out;
}

// The backend parser mirrors resume-lm's shape (uses "tech"); our model calls it
// "tools" (profession-neutral). Accept either.
function adaptImported(parsed: Record<string, unknown>, base: ResumeDoc): ResumeDoc {
  const arr = (v: unknown): Record<string, unknown>[] => (Array.isArray(v) ? v : []);
  const strs = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x) => typeof x === "string") : []);
  const str = (v: unknown): string => (typeof v === "string" ? v : "");
  const prof = (parsed.profile ?? {}) as Record<string, unknown>;
  return {
    ...base,
    profile: {
      ...blankProfile(),
      name: str(prof.name),
      title: str(prof.title),
      email: str(prof.email),
      phone: str(prof.phone),
      location: str(prof.location),
      website: str(prof.website),
      linkedin: str(prof.linkedin),
      github: str(prof.github),
    },
    summary: str(parsed.summary),
    experience: arr(parsed.experience).map((x) => ({
      company: str(x.company),
      role: str(x.role ?? x.position),
      location: str(x.location),
      date: str(x.date),
      bullets: strs(x.bullets ?? x.description),
      tools: strs(x.tools ?? x.tech ?? x.technologies),
    })),
    education: arr(parsed.education).map((e) => ({
      school: str(e.school),
      degree: str(e.degree),
      field: str(e.field),
      location: str(e.location),
      date: str(e.date),
    })),
    skills: arr(parsed.skills).map((s) => ({ category: str(s.category), items: strs(s.items) })).filter((s) => s.category || s.items.length),
    projects: arr(parsed.projects).map((p) => ({
      name: str(p.name),
      bullets: strs(p.bullets ?? p.description),
      tools: strs(p.tools ?? p.tech ?? p.technologies),
      url: str(p.url),
    })),
  };
}

export default function ResumeBuilder({ accomplishments }: { accomplishments: string[] }) {
  const [resumes, setResumes] = useState<ResumeDoc[]>([]);
  const [doc, setDoc] = useState<ResumeDoc | null>(null);
  const [view, setView] = useState<"edit" | "preview" | "compare">("edit");
  const [uploadMsg, setUploadMsg] = useState("");
  const [savedAt, setSavedAt] = useState(0);
  const savedJson = useRef("");

  const [gen, setGen] = useState<{ target: GenTarget; role: string; draft: string; busy: boolean } | null>(null);
  const [score, setScore] = useState<ScoreResult | null>(null);
  const [scoring, setScoring] = useState(false);
  const [coverBusy, setCoverBusy] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [importText, setImportText] = useState("");
  const [importBusy, setImportBusy] = useState(false);
  const [copied, setCopied] = useState("");
  const [exported, setExported] = useState("");
  const [fileBusy, setFileBusy] = useState<"" | "import" | "job">("");
  const importFileRef = useRef<HTMLInputElement>(null);
  const jobFileRef = useRef<HTMLInputElement>(null);

  // Read any dropped/chosen file → extract its text → route it to the target.
  function readFileInto(file: File | undefined, target: "import" | "job") {
    if (!file) return;
    if (target === "import") setUploadMsg(`Reading ${file.name}…`);
    const reader = new FileReader();
    reader.onerror = () => {
      if (target === "import") setUploadMsg("Couldn't read that file.");
      setFileBusy("");
    };
    reader.onload = async () => {
      const url = String(reader.result);
      const mime = url.slice(5, url.indexOf(";")) || file.type || "application/octet-stream";
      const b64 = url.slice(url.indexOf(",") + 1);
      setFileBusy(target);
      try {
        const text = await extractText(file.name, mime, b64);
        if (target === "import") {
          // Store the original as the "before", show it, then structure it.
          setImportText(text);
          update({ sourceText: text });
          setUploadMsg(`✓ Loaded ${file.name} — structuring…`);
          setView("compare");
          await parseAndApply(text);
          setUploadMsg(`✓ Loaded ${file.name}`);
        } else {
          update({ jobDescription: text });
        }
      } catch (e) {
        if (target === "import") setUploadMsg(`Import failed: ${String(e)}`);
        else alert(String(e));
      } finally {
        setFileBusy("");
      }
    };
    reader.readAsDataURL(file);
  }

  // If the parent didn't hand us wins (e.g. deep-linked), load them ourselves.
  const [acc, setAcc] = useState<string[]>(accomplishments);
  useEffect(() => {
    setAcc(accomplishments);
  }, [accomplishments]);
  useEffect(() => {
    if (!inTauri || accomplishments.length) return;
    listEvents(2000).then((evs: VaultEvent[]) => setAcc(accomplishmentsFrom(evs))).catch(() => {});
  }, [accomplishments.length]);

  useEffect(() => {
    if (!inTauri) return;
    loadResumes()
      .then((list) => {
        if (list.length) {
          setResumes(list);
          setDoc(list[0]);
          savedJson.current = JSON.stringify(list[0]);
        } else {
          const base = blankDoc("Base résumé", true);
          saveResume(base)
            .then(() => {
              savedJson.current = JSON.stringify(base);
              setResumes([base]);
              setDoc(base);
            })
            .catch(() => {});
        }
      })
      .catch(() => {});
  }, []);

  // Debounced autosave with change detection: one event per edit-burst, never on load.
  useEffect(() => {
    if (!doc || !inTauri) return;
    const json = JSON.stringify(doc);
    if (json === savedJson.current) return;
    const t = setTimeout(() => {
      saveResume(doc)
        .then(() => {
          savedJson.current = json;
          setSavedAt(Date.now());
          setResumes((rs) => rs.map((r) => (r.id === doc.id ? doc : r)));
        })
        .catch(() => {});
    }, 900);
    return () => clearTimeout(t);
  }, [doc]);

  function update(patch: Partial<ResumeDoc>) {
    setDoc((d) => (d ? { ...d, ...patch } : d));
  }

  function selectResume(id: string) {
    const d = resumes.find((r) => r.id === id);
    if (!d) return;
    setDoc(d);
    savedJson.current = JSON.stringify(d);
    setScore(null);
    setView("edit");
  }

  async function newTailored() {
    const base = resumes.find((r) => r.isBase) ?? doc;
    const nd = blankDoc("Tailored résumé", false);
    if (base) {
      nd.profile = { ...base.profile };
      nd.summary = base.summary;
      nd.experience = base.experience.map((x) => ({ ...x, bullets: [...x.bullets], tools: [...x.tools] }));
      nd.education = base.education.map((e) => ({ ...e }));
      nd.skills = base.skills.map((s) => ({ ...s, items: [...s.items] }));
      nd.projects = base.projects.map((p) => ({ ...p, bullets: [...p.bullets], tools: [...p.tools] }));
      nd.sectionOrder = [...base.sectionOrder];
    }
    await saveResume(nd);
    savedJson.current = JSON.stringify(nd);
    setResumes([nd, ...resumes]);
    setDoc(nd);
    setScore(null);
  }

  async function removeCurrent() {
    if (!doc || doc.isBase) return;
    await deleteResume(doc);
    const list = await loadResumes();
    setResumes(list);
    const next = list[0] ?? null;
    setDoc(next);
    savedJson.current = next ? JSON.stringify(next) : "";
  }

  async function runGenerate(target: GenTarget) {
    if (!doc) return;
    let role = doc.targetRole;
    let existing: string[] = [];
    if (target.kind === "experience") {
      const x = doc.experience[target.i];
      role = [x.role, x.company].filter(Boolean).join(", ") || doc.targetRole;
      existing = x.bullets;
    } else if (target.kind === "project") {
      const p = doc.projects[target.i];
      role = p.name || doc.targetRole;
      existing = p.bullets;
    } else if (target.kind === "summary") {
      existing = doc.summary ? [doc.summary] : [];
    } else if (target.kind === "skills") {
      existing = doc.skills.map((s) => `${s.category}: ${s.items.join(", ")}`);
    }
    setGen({ target, role, draft: "", busy: true });
    try {
      const sectionArg = target.kind === "experience" ? "experience" : target.kind === "project" ? "project" : target.kind;
      const out = await resumeSection(sectionArg, role, doc.jobDescription, acc, existing);
      setGen({ target, role, draft: out, busy: false });
    } catch (e) {
      setGen({ target, role, draft: String(e), busy: false });
    }
  }

  function applyGen() {
    if (!gen || !doc) return;
    const t = gen.target;
    if (t.kind === "summary") update({ summary: stripCites(gen.draft) });
    else if (t.kind === "skills") update({ skills: parseSkills(gen.draft) });
    else if (t.kind === "experience") {
      const exp = doc.experience.map((x, i) => (i === t.i ? { ...x, bullets: toBullets(gen.draft) } : x));
      update({ experience: exp });
    } else if (t.kind === "project") {
      const pr = doc.projects.map((p, i) => (i === t.i ? { ...p, bullets: toBullets(gen.draft) } : p));
      update({ projects: pr });
    }
    setGen(null);
  }

  async function runScore() {
    if (!doc) return;
    setScoring(true);
    setScore(null);
    try {
      const raw = await scoreResume(resumeToText(doc), doc.jobDescription);
      setScore(parseJsonLoose<ScoreResult>(raw) ?? { score: 0, matched: [], missing: [], recommendations: [raw] });
    } catch (e) {
      setScore({ score: 0, matched: [], missing: [], recommendations: [String(e)] });
    } finally {
      setScoring(false);
    }
  }

  async function runCover() {
    if (!doc) return;
    setCoverBusy(true);
    try {
      const cl = await generateCoverLetter(resumeToText(doc), doc.jobDescription, doc.profile.name);
      update({ coverLetter: cl });
    } catch (e) {
      update({ coverLetter: String(e) });
    } finally {
      setCoverBusy(false);
    }
  }

  async function parseAndApply(text: string) {
    if (!doc || !text.trim()) return;
    setImportBusy(true);
    try {
      const raw = await parseResumeText(text);
      const parsed = parseJsonLoose<Record<string, unknown>>(raw);
      if (parsed) {
        update(adaptImported(parsed, doc));
        setImportOpen(false);
        setImportText("");
      } else {
        alert("Couldn't structure that résumé automatically. The text is in the box — edit it and press Parse, or fill the sections by hand.");
      }
    } catch (e) {
      alert(String(e));
    } finally {
      setImportBusy(false);
    }
  }

  async function doExport(format: "docx" | "txt" | "md") {
    if (!doc) return;
    const text = format === "md" ? toMarkdown(doc) : resumeToText(doc);
    const filename = doc.name || doc.profile.name || "resume";
    try {
      const path = await exportResume(format, filename, text, doc);
      setExported(path);
      try {
        await revealItemInDir(path);
      } catch {
        /* reveal is best-effort */
      }
    } catch (e) {
      alert(String(e));
    }
  }

  function exportPdf() {
    setView("preview");
    setTimeout(() => window.print(), 80);
  }

  async function copyText(what: "resume" | "cover") {
    if (!doc) return;
    const text = what === "cover" ? doc.coverLetter : resumeToText(doc);
    await navigator.clipboard.writeText(text);
    setCopied(what);
    setTimeout(() => setCopied(""), 1500);
  }

  if (!doc) {
    return <div className="pkg-empty">Setting up your résumé workspace…</div>;
  }

  const genLabel =
    gen?.target.kind === "summary" ? "summary" : gen?.target.kind === "skills" ? "skills" : "bullets";

  return (
    <div className="rb">
      {/* File inputs live at the top level (NOT inside the modal) so the native
          file dialog can't unmount them mid-pick and lose the selection. */}
      <input
        ref={importFileRef}
        type="file"
        style={{ display: "none" }}
        onChange={(e) => {
          readFileInto(e.target.files?.[0], "import");
          e.target.value = "";
        }}
      />
      <input
        ref={jobFileRef}
        type="file"
        style={{ display: "none" }}
        onChange={(e) => {
          readFileInto(e.target.files?.[0], "job");
          e.target.value = "";
        }}
      />

      {/* Toolbar: résumé selector + actions */}
      <div className="rb-bar">
        <select className="field rb-select" value={doc.id} onChange={(e) => selectResume(e.target.value)}>
          {resumes.map((r) => (
            <option key={r.id} value={r.id}>
              {r.isBase ? "★ " : ""}
              {r.name}
              {r.targetRole ? ` — ${r.targetRole}` : ""}
            </option>
          ))}
        </select>
        <button className="btn" onClick={newTailored}>+ Tailored</button>
        <button className="btn primary" onClick={() => importFileRef.current?.click()} disabled={fileBusy === "import"}>
          {fileBusy === "import" ? "Reading…" : "⬆ Upload résumé"}
        </button>
        <button className="btn" onClick={() => setImportOpen(true)}>Paste</button>
        {!doc.isBase && <button className="btn" onClick={removeCurrent}>Delete</button>}
        <span className="rb-saved">{savedAt ? "✓ saved" : ""}</span>
        <div className="rb-tabs">
          <button className={`seg-btn ${view === "edit" ? "on" : ""}`} onClick={() => setView("edit")}>Edit</button>
          <button className={`seg-btn ${view === "compare" ? "on" : ""}`} onClick={() => setView("compare")}>Before / after</button>
          <button className={`seg-btn ${view === "preview" ? "on" : ""}`} onClick={() => setView("preview")}>Preview</button>
        </div>
      </div>
      {uploadMsg && <div className="muted-note" style={{ marginTop: -2 }}>{uploadMsg}</div>}

      {view === "preview" ? (
        <>
          <div className="row-actions" style={{ marginBottom: 6, flexWrap: "wrap" }}>
            <span className="muted-note" style={{ marginRight: 2 }}>Download:</span>
            <button className="btn" onClick={() => doExport("docx")}>Word .docx</button>
            <button className="btn" onClick={() => doExport("txt")}>Text</button>
            <button className="btn" onClick={() => doExport("md")}>Markdown</button>
            <button className="btn primary" onClick={exportPdf}>PDF</button>
            <button className="btn" onClick={() => copyText("resume")}>{copied === "resume" ? "✓ copied" : "Copy"}</button>
          </div>
          <div className="muted-note" style={{ marginBottom: 10 }}>
            {exported ? `✓ Saved to ${exported}` : "Word / Text / Markdown save to your Downloads folder. PDF opens the print dialog — choose “Save as PDF”."}
          </div>
          <div className="rp-wrap">
            <ResumePreview doc={doc} />
          </div>
        </>
      ) : view === "compare" ? (
        <div className="rb-compare">
          <div className="rb-compare-col">
            <div className="sec-label">Before — your uploaded résumé</div>
            {doc.sourceText.trim() ? (
              <div className="rb-before">{doc.sourceText}</div>
            ) : (
              <div className="pkg-empty">
                Upload your current résumé (the <b>⬆ Upload résumé</b> button above) and the original text shows here,
                next to Peregrine's rebuilt version.
              </div>
            )}
          </div>
          <div className="rb-compare-col">
            <div className="sec-label">After — Peregrine's rebuild</div>
            <div className="rp-wrap rp-wrap-sm">
              <ResumePreview doc={doc} />
            </div>
          </div>
        </div>
      ) : (
        <div className="rb-edit">
          {/* Résumé name + rename */}
          <input
            className="field"
            value={doc.name}
            onChange={(e) => update({ name: e.target.value })}
            placeholder="Résumé name (e.g. Base résumé)"
          />

          {/* Profile */}
          <div className="sec-label">Profile</div>
          <div className="rb-grid">
            <input className="field" placeholder="Full name" value={doc.profile.name} onChange={(e) => update({ profile: { ...doc.profile, name: e.target.value } })} />
            <input className="field" placeholder="Title / headline" value={doc.profile.title} onChange={(e) => update({ profile: { ...doc.profile, title: e.target.value } })} />
            <input className="field" placeholder="Email" value={doc.profile.email} onChange={(e) => update({ profile: { ...doc.profile, email: e.target.value } })} />
            <input className="field" placeholder="Phone" value={doc.profile.phone} onChange={(e) => update({ profile: { ...doc.profile, phone: e.target.value } })} />
            <input className="field" placeholder="Location" value={doc.profile.location} onChange={(e) => update({ profile: { ...doc.profile, location: e.target.value } })} />
            <input className="field" placeholder="Website" value={doc.profile.website} onChange={(e) => update({ profile: { ...doc.profile, website: e.target.value } })} />
            <input className="field" placeholder="LinkedIn" value={doc.profile.linkedin} onChange={(e) => update({ profile: { ...doc.profile, linkedin: e.target.value } })} />
            <input className="field" placeholder="GitHub / portfolio" value={doc.profile.github} onChange={(e) => update({ profile: { ...doc.profile, github: e.target.value } })} />
          </div>

          {/* Summary */}
          <div className="sec-label rb-seclabel">Summary <button className="cap" onClick={() => runGenerate({ kind: "summary" })}>✨ generate from my wins</button></div>
          <textarea className="ta" rows={3} placeholder="A 2–3 sentence professional summary." value={doc.summary} onChange={(e) => update({ summary: e.target.value })} />

          {/* Experience */}
          <div className="sec-label rb-seclabel">Experience <button className="cap" onClick={() => update({ experience: [...doc.experience, blankExperience()] })}>+ add</button></div>
          {doc.experience.map((x, i) => (
            <div className="rb-entry" key={i}>
              <div className="rb-grid">
                <input className="field" placeholder="Role / title" value={x.role} onChange={(e) => update({ experience: doc.experience.map((v, k) => (k === i ? { ...v, role: e.target.value } : v)) })} />
                <input className="field" placeholder="Organization" value={x.company} onChange={(e) => update({ experience: doc.experience.map((v, k) => (k === i ? { ...v, company: e.target.value } : v)) })} />
                <input className="field" placeholder="Location" value={x.location} onChange={(e) => update({ experience: doc.experience.map((v, k) => (k === i ? { ...v, location: e.target.value } : v)) })} />
                <input className="field" placeholder="Dates (e.g. 2023–now)" value={x.date} onChange={(e) => update({ experience: doc.experience.map((v, k) => (k === i ? { ...v, date: e.target.value } : v)) })} />
              </div>
              <textarea className="ta" rows={3} placeholder="One achievement per line — outcome first." value={x.bullets.join("\n")} onChange={(e) => update({ experience: doc.experience.map((v, k) => (k === i ? { ...v, bullets: e.target.value.split("\n") } : v)) })} />
              <input className="field" placeholder="Tools / methods used (comma-separated)" value={x.tools.join(", ")} onChange={(e) => update({ experience: doc.experience.map((v, k) => (k === i ? { ...v, tools: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) } : v)) })} />
              <div className="rb-entry-actions">
                <button className="cap" onClick={() => runGenerate({ kind: "experience", i })}>✨ generate bullets</button>
                <button className="cap danger" onClick={() => update({ experience: doc.experience.filter((_, k) => k !== i) })}>remove</button>
              </div>
            </div>
          ))}

          {/* Skills */}
          <div className="sec-label rb-seclabel">Skills <button className="cap" onClick={() => runGenerate({ kind: "skills" })}>✨ generate</button> <button className="cap" onClick={() => update({ skills: [...doc.skills, { category: "", items: [] }] })}>+ add</button></div>
          {doc.skills.map((s, i) => (
            <div className="rb-skill-row" key={i}>
              <input className="field" style={{ maxWidth: 160 }} placeholder="Category" value={s.category} onChange={(e) => update({ skills: doc.skills.map((v, k) => (k === i ? { ...v, category: e.target.value } : v)) })} />
              <input className="field" placeholder="items, comma-separated" value={s.items.join(", ")} onChange={(e) => update({ skills: doc.skills.map((v, k) => (k === i ? { ...v, items: e.target.value.split(",").map((t) => t.trim()).filter(Boolean) } : v)) })} />
              <button className="cap danger" onClick={() => update({ skills: doc.skills.filter((_, k) => k !== i) })}>×</button>
            </div>
          ))}

          {/* Education */}
          <div className="sec-label rb-seclabel">Education <button className="cap" onClick={() => update({ education: [...doc.education, blankEducation()] })}>+ add</button></div>
          {doc.education.map((ed, i) => (
            <div className="rb-entry" key={i}>
              <div className="rb-grid">
                <input className="field" placeholder="Degree / credential" value={ed.degree} onChange={(e) => update({ education: doc.education.map((v, k) => (k === i ? { ...v, degree: e.target.value } : v)) })} />
                <input className="field" placeholder="Field" value={ed.field} onChange={(e) => update({ education: doc.education.map((v, k) => (k === i ? { ...v, field: e.target.value } : v)) })} />
                <input className="field" placeholder="School / institution" value={ed.school} onChange={(e) => update({ education: doc.education.map((v, k) => (k === i ? { ...v, school: e.target.value } : v)) })} />
                <input className="field" placeholder="Dates" value={ed.date} onChange={(e) => update({ education: doc.education.map((v, k) => (k === i ? { ...v, date: e.target.value } : v)) })} />
              </div>
              <div className="rb-entry-actions"><button className="cap danger" onClick={() => update({ education: doc.education.filter((_, k) => k !== i) })}>remove</button></div>
            </div>
          ))}

          {/* Projects */}
          <div className="sec-label rb-seclabel">Projects <button className="cap" onClick={() => update({ projects: [...doc.projects, blankProject()] })}>+ add</button></div>
          {doc.projects.map((pr, i) => (
            <div className="rb-entry" key={i}>
              <div className="rb-grid">
                <input className="field" placeholder="Project name" value={pr.name} onChange={(e) => update({ projects: doc.projects.map((v, k) => (k === i ? { ...v, name: e.target.value } : v)) })} />
                <input className="field" placeholder="Link (optional)" value={pr.url} onChange={(e) => update({ projects: doc.projects.map((v, k) => (k === i ? { ...v, url: e.target.value } : v)) })} />
              </div>
              <textarea className="ta" rows={2} placeholder="One highlight per line." value={pr.bullets.join("\n")} onChange={(e) => update({ projects: doc.projects.map((v, k) => (k === i ? { ...v, bullets: e.target.value.split("\n") } : v)) })} />
              <input className="field" placeholder="Tools / methods (comma-separated)" value={pr.tools.join(", ")} onChange={(e) => update({ projects: doc.projects.map((v, k) => (k === i ? { ...v, tools: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) } : v)) })} />
              <div className="rb-entry-actions">
                <button className="cap" onClick={() => runGenerate({ kind: "project", i })}>✨ generate bullets</button>
                <button className="cap danger" onClick={() => update({ projects: doc.projects.filter((_, k) => k !== i) })}>remove</button>
              </div>
            </div>
          ))}

          {/* Tailor to a job + score + cover letter */}
          <div className="sec-label rb-seclabel">
            Tailor to a job
            <button className="cap" onClick={() => jobFileRef.current?.click()} disabled={fileBusy === "job"}>
              {fileBusy === "job" ? "reading…" : "⬆ from a file"}
            </button>
          </div>
          <input className="field" placeholder="Target role (e.g. Senior Nurse Manager)" value={doc.targetRole} onChange={(e) => update({ targetRole: e.target.value })} />
          <textarea className="ta" rows={4} placeholder="Paste a job description, or attach a file — generation tailors toward it, and you can score against it." value={doc.jobDescription} onChange={(e) => update({ jobDescription: e.target.value })} />
          <div className="row-actions">
            <button className="btn" onClick={runScore} disabled={scoring || !doc.jobDescription.trim()}>{scoring ? "Scoring…" : "Score against this job"}</button>
            <button className="btn" onClick={runCover} disabled={coverBusy}>{coverBusy ? "Writing…" : "Generate cover letter"}</button>
          </div>

          {score && (
            <div className="rb-score">
              <div className="rb-score-h"><span className="rb-score-num">{score.score}</span><span className="rb-score-max">/100 match</span></div>
              {score.matched.length > 0 && <div className="rb-chips"><b>Matched:</b> {score.matched.map((m, i) => <span className="chip ok" key={i}>{m}</span>)}</div>}
              {score.missing.length > 0 && <div className="rb-chips"><b>Missing:</b> {score.missing.map((m, i) => <span className="chip miss" key={i}>{m}</span>)}</div>}
              {score.recommendations.length > 0 && (
                <ul className="rb-recs">{score.recommendations.map((r, i) => <li key={i}>{r}</li>)}</ul>
              )}
            </div>
          )}

          {doc.coverLetter && (
            <>
              <div className="sec-label rb-seclabel">Cover letter <button className="cap" onClick={() => copyText("cover")}>{copied === "cover" ? "✓ copied" : "copy"}</button></div>
              <textarea className="ta out" rows={10} value={doc.coverLetter} onChange={(e) => update({ coverLetter: e.target.value })} />
            </>
          )}
        </div>
      )}

      {/* Generation review modal — cited draft + sources, edit then apply */}
      {gen && (
        <div className="overlay" role="dialog" aria-modal="true" onClick={() => !gen.busy && setGen(null)}>
          <div className="gen-card" onClick={(e) => e.stopPropagation()}>
            <h2>Generated {genLabel} <span className="muted-note">grounded in your wins</span></h2>
            {gen.busy ? (
              <div className="pkg-empty">Writing from your captured wins…</div>
            ) : (
              <>
                <textarea className="ta out" rows={7} value={gen.draft} onChange={(e) => setGen({ ...gen, draft: e.target.value })} />
                {acc.length > 0 && (
                  <>
                    <div className="sec-label">Sources — what each [n] refers to</div>
                    <div className="cites" style={{ maxHeight: 160, overflowY: "auto" }}>
                      {acc.map((a, i) => (
                        <div className="cite" key={i}><span className="cite-n">[{i + 1}]</span><span className="cite-t">{a}</span></div>
                      ))}
                    </div>
                  </>
                )}
                <div className="row-actions" style={{ marginTop: 10 }}>
                  <button className="btn primary" onClick={applyGen}>Use this</button>
                  <button className="btn" onClick={() => setGen(null)}>Cancel</button>
                  <span className="muted-note">Citations are stripped when inserted — the résumé stays clean.</span>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      {/* Paste modal (files use the ⬆ Upload résumé button in the toolbar) */}
      {importOpen && (
        <div className="overlay" role="dialog" aria-modal="true" onClick={() => !importBusy && setImportOpen(false)}>
          <div className="gen-card" onClick={(e) => e.stopPropagation()}>
            <h2>Paste your résumé</h2>
            <p className="muted-note">Paste the text and Peregrine reads it into the sections — using only what's there, nothing invented. (For a file, use “⬆ Upload résumé”.)</p>
            <textarea className="ta" rows={10} placeholder="Paste résumé text here" value={importText} onChange={(e) => setImportText(e.target.value)} />
            <div className="row-actions" style={{ marginTop: 10 }}>
              <button
                className="btn primary"
                onClick={() => {
                  update({ sourceText: importText });
                  parseAndApply(importText);
                }}
                disabled={importBusy || !importText.trim()}
              >
                {importBusy ? "Reading into sections…" : "Parse into sections"}
              </button>
              <button className="btn" onClick={() => setImportOpen(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
