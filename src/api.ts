import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  provider: string;
  endpoint: string;
  model: string;
  trust_mode: string;
  profession: string;
  seniority: string;
  appearance: string;
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

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (newSettings: Settings) => invoke<void>("save_settings", { newSettings });
export const hasApiKey = () => invoke<boolean>("has_api_key");
export const setApiKey = (key: string) => invoke<void>("set_api_key", { key });
export const testConnection = () => invoke<string>("test_connection");
export const sendMessage = (history: Msg[]) => invoke<Reply>("send_message", { history });
export const activityLog = () => invoke<ActivityEntry[]>("activity_log");
