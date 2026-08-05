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
export const tryAutoUnlock = () => invoke<boolean>("try_auto_unlock");
export const lockVault = () => invoke<void>("lock_vault");
export const addEvent = (kind: string, payload: unknown) => invoke<VaultEvent>("add_event", { kind, payload });
export const listEvents = (limit: number) => invoke<VaultEvent[]>("list_events", { limit });

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (newSettings: Settings) => invoke<void>("save_settings", { newSettings });
export const hasApiKey = () => invoke<boolean>("has_api_key");
export const setApiKey = (key: string) => invoke<void>("set_api_key", { key });
export const testConnection = () => invoke<string>("test_connection");
export const sendMessage = (history: Msg[]) => invoke<Reply>("send_message", { history });
export const activityLog = () => invoke<ActivityEntry[]>("activity_log");
