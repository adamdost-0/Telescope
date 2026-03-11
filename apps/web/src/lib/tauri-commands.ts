/**
 * TypeScript types for Tauri IPC commands.
 * These match the #[tauri::command] functions in src-tauri/src/main.rs.
 */

export interface ClusterContext {
  name: string;
  cluster_server: string | null;
  namespace: string | null;
  is_active: boolean;
}

export interface ResourceEntry {
  gvk: string;
  namespace: string;
  name: string;
  resource_version: string;
  content: string;
  updated_at: string;
}

export type ConnectionState =
  | { state: "Disconnected" }
  | { state: "Connecting" }
  | {
      state: "Syncing";
      detail: { resources_synced: number; resources_total: number | null };
    }
  | { state: "Ready" }
  | { state: "Degraded"; detail: { message: string } }
  | { state: "Error"; detail: { message: string } }
  | {
      state: "Backoff";
      detail: { attempt: number; wait: { secs: number; nanos: number } };
    };

/**
 * Check if running inside Tauri (desktop) or browser (web).
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
