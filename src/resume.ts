// Structured résumé model + vault storage + grounded AI wrappers.
// Profession-neutral by design: "tools" covers software, equipment, methods,
// certifications — whatever a given field uses — not just tech stacks.

import { invoke } from "@tauri-apps/api/core";
import { addEvent, listEvents, type VaultEvent } from "./api";

export interface RProfile {
  name: string;
  title: string;
  email: string;
  phone: string;
  location: string;
  website: string;
  linkedin: string;
  github: string;
}
export interface RExperience {
  company: string;
  role: string;
  location: string;
  date: string;
  bullets: string[];
  tools: string[];
}
export interface REducation {
  school: string;
  degree: string;
  field: string;
  location: string;
  date: string;
}
export interface RSkill {
  category: string;
  items: string[];
}
export interface RProject {
  name: string;
  bullets: string[];
  tools: string[];
  url: string;
}
export type SectionKey = "summary" | "experience" | "skills" | "education" | "projects";
export const SECTION_LABELS: Record<SectionKey, string> = {
  summary: "Summary",
  experience: "Experience",
  skills: "Skills",
  education: "Education",
  projects: "Projects",
};

export interface ResumeDoc {
  id: string;
  name: string;
  isBase: boolean;
  targetRole: string;
  jobDescription: string;
  profile: RProfile;
  summary: string;
  experience: RExperience[];
  education: REducation[];
  skills: RSkill[];
  projects: RProject[];
  sectionOrder: SectionKey[];
  coverLetter: string;
  sourceText: string; // the uploaded/pasted original — the "before" for comparison
  updatedAt: number;
  deleted?: boolean;
}

const DEFAULT_ORDER: SectionKey[] = ["summary", "experience", "skills", "education", "projects"];

function uuid(): string {
  try {
    return crypto.randomUUID();
  } catch {
    return `r-${Date.now()}-${Math.floor(Math.random() * 1e9)}`;
  }
}

export function blankProfile(): RProfile {
  return { name: "", title: "", email: "", phone: "", location: "", website: "", linkedin: "", github: "" };
}

export function blankDoc(name: string, isBase: boolean): ResumeDoc {
  return {
    id: uuid(),
    name,
    isBase,
    targetRole: "",
    jobDescription: "",
    profile: blankProfile(),
    summary: "",
    experience: [],
    education: [],
    skills: [],
    projects: [],
    sectionOrder: [...DEFAULT_ORDER],
    coverLetter: "",
    sourceText: "",
    updatedAt: Date.now(),
  };
}

export function blankExperience(): RExperience {
  return { company: "", role: "", location: "", date: "", bullets: [], tools: [] };
}
export function blankEducation(): REducation {
  return { school: "", degree: "", field: "", location: "", date: "" };
}
export function blankProject(): RProject {
  return { name: "", bullets: [], tools: [], url: "" };
}

// Latest event per résumé id wins (event-sourced: a "save" is a new event with the
// same id). Tombstoned (deleted) docs are dropped.
export async function loadResumes(): Promise<ResumeDoc[]> {
  const evs = await listEvents(4000);
  const latest = new Map<string, { ts: number; doc: ResumeDoc }>();
  for (const e of evs) {
    if (e.kind !== "resume_doc") continue;
    const doc = e.payload as ResumeDoc;
    if (!doc || !doc.id) continue;
    const prev = latest.get(doc.id);
    if (!prev || e.ts_ms > prev.ts) latest.set(doc.id, { ts: e.ts_ms, doc: { ...doc, updatedAt: e.ts_ms } });
  }
  return [...latest.values()]
    .map((v) => v.doc)
    .filter((d) => !d.deleted)
    .sort((a, b) => Number(b.isBase) - Number(a.isBase) || b.updatedAt - a.updatedAt);
}

export async function saveResume(doc: ResumeDoc): Promise<void> {
  await addEvent("resume_doc", { ...doc, updatedAt: Date.now() });
}

export async function deleteResume(doc: ResumeDoc): Promise<void> {
  await addEvent("resume_doc", { ...doc, deleted: true, updatedAt: Date.now() });
}

// Generated lines carry [n] citations for grounding review; strip them for the
// finished résumé (and for scoring/cover-letter/export text).
export function stripCites(s: string): string {
  return s.replace(/\s*\[\d+(?:\s*,\s*\d+)*\]/g, "").trim();
}

// Split a generated block into clean bullet strings (drop leading marks + citations).
export function toBullets(block: string): string[] {
  return block
    .split("\n")
    .map((l) => stripCites(l.replace(/^\s*[-*•]\s*/, "")))
    .filter(Boolean);
}

// Flatten to plain text — the input for scoring, cover letters, and copy/export.
export function resumeToText(d: ResumeDoc): string {
  const L: string[] = [];
  const p = d.profile;
  if (p.name) L.push(p.name);
  if (p.title) L.push(p.title);
  const contact = [p.email, p.phone, p.location, p.website, p.linkedin, p.github].filter(Boolean).join(" · ");
  if (contact) L.push(contact);
  const push = (k: SectionKey) => {
    if (k === "summary" && d.summary.trim()) L.push("", "SUMMARY", d.summary.trim());
    if (k === "experience" && d.experience.length) {
      L.push("", "EXPERIENCE");
      for (const x of d.experience) {
        L.push([x.role, x.company, x.location].filter(Boolean).join(" — ") + (x.date ? `  (${x.date})` : ""));
        for (const b of x.bullets) if (b.trim()) L.push(`• ${b}`);
        if (x.tools.length) L.push(`Tools: ${x.tools.join(", ")}`);
      }
    }
    if (k === "skills" && d.skills.length) {
      L.push("", "SKILLS");
      for (const s of d.skills) if (s.items.length) L.push(`${s.category}: ${s.items.join(", ")}`);
    }
    if (k === "education" && d.education.length) {
      L.push("", "EDUCATION");
      for (const e of d.education)
        L.push([e.degree, e.field].filter(Boolean).join(", ") + [e.school, e.location, e.date].filter(Boolean).map((x) => ` — ${x}`).join(""));
    }
    if (k === "projects" && d.projects.length) {
      L.push("", "PROJECTS");
      for (const pr of d.projects) {
        L.push(pr.name + (pr.url ? `  (${pr.url})` : ""));
        for (const b of pr.bullets) if (b.trim()) L.push(`• ${b}`);
        if (pr.tools.length) L.push(`Tools: ${pr.tools.join(", ")}`);
      }
    }
  };
  for (const k of d.sectionOrder) push(k);
  return L.join("\n");
}

// ---- Grounded AI wrappers (backend enforces the no-invention / citation rules) ----

export const resumeSection = (section: SectionKey | "experience" | "project", role: string, job: string, accomplishments: string[], existing: string[]) =>
  invoke<string>("resume_section", { section, role, job, accomplishments, existing });

export const scoreResume = (resumeText: string, job: string) => invoke<string>("resume_score", { resumeText, job });

export const generateCoverLetter = (resumeText: string, job: string, name: string) =>
  invoke<string>("cover_letter", { resumeText, job, name });

export const parseResumeText = (raw: string) => invoke<string>("parse_resume", { raw });

// Write the finished résumé to a real file in Downloads; returns the saved path.
export const exportResume = (format: "docx" | "txt" | "md" | "html", filename: string, text: string, doc: ResumeDoc) =>
  invoke<string>("export_resume", { format, filename, text, doc });

// A Markdown rendering of the résumé (for the .md export).
export function toMarkdown(d: ResumeDoc): string {
  const L: string[] = [];
  const p = d.profile;
  if (p.name) L.push(`# ${p.name}`);
  if (p.title) L.push(`*${p.title}*`);
  const contact = [p.email, p.phone, p.location, p.website, p.linkedin, p.github].filter(Boolean).join(" · ");
  if (contact) L.push(contact);
  for (const k of d.sectionOrder) {
    if (k === "summary" && d.summary.trim()) L.push("", "## Summary", stripCites(d.summary));
    if (k === "experience" && d.experience.length) {
      L.push("", "## Experience");
      for (const x of d.experience) {
        L.push(`**${[x.role, x.company].filter(Boolean).join(", ")}** — ${[x.location, x.date].filter(Boolean).join(" · ")}`);
        for (const b of x.bullets) if (b.trim()) L.push(`- ${stripCites(b)}`);
        if (x.tools.length) L.push(`*Tools: ${x.tools.join(", ")}*`);
      }
    }
    if (k === "skills" && d.skills.length) {
      L.push("", "## Skills");
      for (const s of d.skills) if (s.items.length) L.push(`**${s.category}:** ${s.items.join(", ")}`);
    }
    if (k === "education" && d.education.length) {
      L.push("", "## Education");
      for (const e of d.education) L.push(`**${[e.degree, e.field].filter(Boolean).join(", ")}** — ${[e.school, e.location, e.date].filter(Boolean).join(" · ")}`);
    }
    if (k === "projects" && d.projects.length) {
      L.push("", "## Projects");
      for (const pr of d.projects) {
        L.push(`**${pr.name}**${pr.url ? ` — ${pr.url}` : ""}`);
        for (const b of pr.bullets) if (b.trim()) L.push(`- ${stripCites(b)}`);
        if (pr.tools.length) L.push(`*Tools: ${pr.tools.join(", ")}*`);
      }
    }
  }
  return L.join("\n");
}

// The user's dated wins — the grounding for every generated line.
export function accomplishmentsFrom(events: VaultEvent[]): string[] {
  return events
    .filter((e) => e.kind === "win")
    .map((e) => {
      const text = (e.payload as { text?: string })?.text ?? "";
      const date = new Date(e.ts_ms).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
      return text ? `${date} — ${text}` : "";
    })
    .filter(Boolean)
    .reverse(); // oldest first, so [n] reads chronologically
}

export interface ScoreResult {
  score: number;
  matched: string[];
  missing: string[];
  recommendations: string[];
}

// Models sometimes wrap JSON in prose or code fences; extract the object safely.
export function parseJsonLoose<T>(raw: string): T | null {
  const start = raw.indexOf("{");
  const end = raw.lastIndexOf("}");
  if (start === -1 || end === -1 || end < start) return null;
  try {
    return JSON.parse(raw.slice(start, end + 1)) as T;
  } catch {
    return null;
  }
}
