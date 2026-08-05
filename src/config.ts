export type ScreenId =
  | "today"
  | "meetings"
  | "timeline"
  | "debrief"
  | "memory"
  | "resume"
  | "settings";

export const NAV: { id: ScreenId; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "meetings", label: "Meetings" },
  { id: "timeline", label: "Timeline" },
  { id: "debrief", label: "Debrief" },
  { id: "memory", label: "Memory" },
  { id: "resume", label: "Résumé" },
  { id: "settings", label: "Settings" },
];

export type Mode = "daylight" | "fieldbook" | "instrument";

export const MODES: { id: Mode; label: string }[] = [
  { id: "daylight", label: "Daylight" },
  { id: "fieldbook", label: "Fieldbook" },
  { id: "instrument", label: "Instrument" },
];

/* Placeholder role until the vault + onboarding land (Phase 1/2).
   Peregrine mentors as the senior version of this — any profession. */
export const ROLE = { profession: "Product manager", seniority: "Senior" };
