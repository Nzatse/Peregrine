import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  provider: string;
  endpoint: string;
  model: string;
  trust_mode: string;
  profession: string;
  seniority: string;
  appearance: string;
  whisper_model_path: string;
}

export interface WhisperStatus {
  path: string;
  present: boolean;
}

export interface Msg {
  role: "user" | "assistant";
  content: string;
}

export interface Reply {
  text: string;
  source: string;
}

export interface ActivityEntry {
  ts_ms: number;
  summary: string;
  destination: string;
  bytes_out: number;
  allowed: boolean;
}

/** True when running inside the Tauri shell (vs. a plain browser preview). */
export const inTauri = typeof (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !== "undefined";

export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
}

export interface VaultEvent {
  id: string;
  ts_ms: number;
  device: string;
  kind: string;
  payload: unknown;
}

export const vaultStatus = () => invoke<VaultStatus>("vault_status");
export const createVault = (passphrase: string) => invoke<void>("create_vault", { passphrase });
export const unlockVault = (passphrase: string) => invoke<void>("unlock_vault", { passphrase });
export const lockVault = () => invoke<void>("lock_vault");
export const addEvent = (kind: string, payload: unknown) => invoke<VaultEvent>("add_event", { kind, payload });
export const listEvents = (limit: number) => invoke<VaultEvent[]>("list_events", { limit });
export const exportVault = (dest: string) => invoke<void>("export_vault", { dest });
export const importMerge = (src: string) => invoke<number>("import_merge", { src });

export const whisperStatus = () => invoke<WhisperStatus>("whisper_status");
export const listenStart = () => invoke<void>("listen_start");
export const listenStop = () => invoke<string>("listen_stop");
export const captureMeeting = (transcript: string) => invoke<string>("capture_meeting", { transcript });

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (newSettings: Settings) => invoke<void>("save_settings", { newSettings });
export const hasApiKey = () => invoke<boolean>("has_api_key");
export const setApiKey = (key: string) => invoke<void>("set_api_key", { key });
export const testConnection = () => invoke<string>("test_connection");
export const sendMessage = (history: Msg[]) => invoke<Reply>("send_message", { history });
export const analyzeDocument = (name: string, mime: string, dataBase64: string, question: string) =>
  invoke<Reply>("analyze_document", { name, mime, dataBase64, question });
export const analyzeFolder = (name: string, content: string, question: string) =>
  invoke<Reply>("analyze_folder", { name, content, question });
export const debriefReply = (history: Msg[], context: string[]) => invoke<Reply>("debrief_reply", { history, context });
export const renderResume = (accomplishments: string[], base: string, job: string) =>
  invoke<string>("render_resume", { accomplishments, base, job });
export const activityLog = () => invoke<ActivityEntry[]>("activity_log");

// --- Updates ---------------------------------------------------------------
// Deliberately manual: Peregrine never phones home on its own. The user asks,
// we check, and only then does anything leave. Signatures are verified in Rust
// against the public key in tauri.conf.json — an unsigned or tampered bundle is
// rejected before it can be installed.

export async function checkForUpdate(): Promise<{ available: boolean; version?: string; notes?: string }> {
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return { available: false };
  return { available: true, version: update.version, notes: update.body };
}

/** Downloads, verifies, installs, then relaunches into the new version. */
export async function installUpdate(): Promise<void> {
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return;
  await update.downloadAndInstall();
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}
