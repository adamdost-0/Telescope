/**
 * TypeScript types for Tauri IPC commands.
 * These match the #[tauri::command] functions in src-tauri/src/main.rs.
 */

export interface ClusterContext {
  name: string;
  cluster_server: string | null;
  namespace: string | null;
  is_active: boolean;
  /** Authentication method: "exec" | "token" | "certificate" | "unknown" */
  auth_type: string;
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

export interface LogChunk {
  lines: string;
  is_complete: boolean;
}

export interface ClusterInfo {
  server_version: string;
  platform: string;
  server_url: string;
  auth_type: string;
  exec_plugin: string | null;
  is_aks: boolean;
  auth_hint: string | null;
}

export interface ContainerMetrics {
  name: string;
  cpu_millicores: number;
  memory_bytes: number;
}

export interface PodMetrics {
  name: string;
  namespace: string;
  containers: ContainerMetrics[];
  cpu_millicores: number;
  memory_bytes: number;
}

export interface NodeMetricsData {
  name: string;
  cpu_millicores: number;
  memory_bytes: number;
  cpu_allocatable: number;
  memory_allocatable: number;
  cpu_percent: number;
  memory_percent: number;
}

export interface HelmRelease {
  name: string;
  namespace: string;
  chart: string;
  app_version: string;
  revision: number;
  status: string;
  updated: string;
}

/**
 * Check if running inside Tauri (desktop) or browser (web).
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
