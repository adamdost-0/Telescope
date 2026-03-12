/**
 * Unified API layer that works in both Tauri (desktop) and browser (web) contexts.
 * Desktop: Uses Tauri invoke for IPC.
 * Web: Falls back to HTTP fetch against the engine API.
 */
import {
  isTauri,
  type ClusterContext,
  type ClusterInfo,
  type HelmRelease,
  type ResourceEntry,
  type ConnectionState,
  type PodMetrics,
  type NodeMetricsData,
} from './tauri-commands';

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  console.log(`[telescope] invoke: ${command}`, args, `isTauri=${isTauri()}`);
  if (isTauri()) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    try {
      const result = await tauriInvoke<T>(command, args);
      console.log(`[telescope] invoke ${command} OK:`, result);
      return result;
    } catch (e) {
      console.error(`[telescope] invoke ${command} FAILED:`, e);
      throw e;
    }
  }

  // Web fallback: map commands to HTTP endpoints
  console.log(`[telescope] using web fallback for: ${command}`);
  return webFallback<T>(command, args);
}

async function webFallback<T>(command: string, _args?: Record<string, unknown>): Promise<T> {
  switch (command) {
    case 'list_contexts': {
      // In web mode, fetch from /api/clusters and map to ClusterContext format
      const res = await fetch('/api/clusters');
      if (!res.ok) return [] as unknown as T;
      const data = await res.json();
      const clusters = data.clusters ?? [];
      return clusters.map((c: Record<string, unknown>) => ({
        name: (c.name as string) ?? (c.id as string),
        cluster_server: (c.server as string) ?? null,
        namespace: null,
        is_active: false,
        auth_type: (c.auth_type as string) ?? 'unknown',
      })) as unknown as T;
    }
    case 'get_connection_state':
      return { state: 'Disconnected' } as unknown as T;
    case 'get_cluster_info':
      return null as unknown as T;
    case 'get_resource_counts':
    case 'get_pods':
    case 'get_resources':
    case 'get_events':
      return [] as unknown as T;
    case 'list_namespaces':
      return ['default'] as unknown as T;
    case 'connect_to_context':
    case 'disconnect':
    case 'set_namespace':
      return undefined as unknown as T;
    case 'delete_resource':
      return 'Deleted (stub)' as unknown as T;
    case 'rollout_restart':
      return 'Rollout restart initiated (stub)' as unknown as T;
    case 'rollout_status':
      return { desired: 1, ready: 1, updated: 1, available: 1, is_complete: true, message: 'Rollout complete' } as unknown as T;
    case 'exec_command':
      return { stdout: 'Exec not available in web mode', stderr: '', success: false } as unknown as T;
    case 'apply_resource':
      return { success: true, message: 'Applied (stub)' } as unknown as T;
    case 'scale_resource':
    case 'start_port_forward':
      return undefined as unknown as T;
    case 'get_pod_metrics':
    case 'get_node_metrics':
      return [] as unknown as T;
    case 'check_metrics_available':
      return false as unknown as T;
    case 'list_helm_releases':
      return [] as unknown as T;
    case 'get_helm_release_history':
      return [] as unknown as T;
    case 'get_helm_release_values':
      return '# Values not available in web mode\n' as unknown as T;
    case 'helm_rollback':
      return 'Rollback not available in web mode' as unknown as T;
    default:
      throw new Error(`Command "${command}" not available in web mode`);
  }
}

/** Fetch counts for all major resource types. */
export async function getResourceCounts(): Promise<[string, number][]> {
  try {
    return await invoke<[string, number][]>('get_resource_counts');
  } catch {
    return [];
  }
}

export async function listContexts(): Promise<ClusterContext[]> {
  try {
    return await invoke<ClusterContext[]>('list_contexts');
  } catch {
    return [];
  }
}

export async function activeContext(): Promise<string | null> {
  try {
    return await invoke<string>('active_context');
  } catch {
    return null;
  }
}

export async function getConnectionState(): Promise<ConnectionState> {
  try {
    return await invoke<ConnectionState>('get_connection_state');
  } catch {
    return { state: 'Disconnected' };
  }
}

/** Fetch cluster version, auth, and AKS info for the connected context. */
export async function getClusterInfo(): Promise<ClusterInfo | null> {
  try {
    return await invoke<ClusterInfo>('get_cluster_info');
  } catch {
    return null;
  }
}

export async function getPods(namespace?: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_pods', { namespace: namespace ?? null });
  } catch {
    return [];
  }
}

/** Fetch resources of any GVK from the SQLite store. */
export async function getResources(gvk: string, namespace?: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_resources', { gvk, namespace: namespace ?? null });
  } catch {
    return [];
  }
}

/** Fetch events, optionally filtered by namespace and/or involved object name. */
export async function getEvents(namespace?: string | null, involvedObject?: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_events', {
      namespace: namespace ?? null,
      involvedObject: involvedObject ?? null,
    });
  } catch {
    return [];
  }
}

/** Rollout status shape returned by the engine. */
export interface RolloutStatus {
  desired: number;
  ready: number;
  updated: number;
  available: number;
  is_complete: boolean;
  message: string;
}

/** Restart a Deployment rollout. */
export async function rolloutRestart(namespace: string, name: string): Promise<string> {
  return invoke<string>('rollout_restart', { namespace, name });
}

/** Get rollout status for a Deployment. */
export async function rolloutStatus(namespace: string, name: string): Promise<RolloutStatus> {
  return invoke<RolloutStatus>('rollout_status', { namespace, name });
}

/** Delete a namespaced Kubernetes resource by GVK, namespace, and name. */
export async function deleteResource(
  gvk: string,
  namespace: string,
  name: string,
): Promise<string> {
  return invoke<string>('delete_resource', { gvk, namespace, name });
}

export async function connectToContext(contextName: string): Promise<void> {
  await invoke<void>('connect_to_context', { contextName });
}

export async function disconnect(): Promise<void> {
  await invoke<void>('disconnect');
}

export async function setNamespace(namespace: string): Promise<void> {
  await invoke<void>('set_namespace', { namespace });
}

export async function listNamespaces(): Promise<string[]> {
  try {
    return await invoke<string[]>('list_namespaces');
  } catch {
    return ['default'];
  }
}

/** Fetch log output for a pod container. */
export async function getPodLogs(
  namespace: string,
  pod: string,
  container?: string,
  previous?: boolean,
  tailLines?: number,
): Promise<string> {
  try {
    return await invoke<string>('get_pod_logs', {
      namespace,
      pod,
      container: container ?? null,
      previous: previous ?? false,
      tailLines: tailLines ?? 500,
    });
  } catch {
    return '';
  }
}

/** List container names for a pod (init containers prefixed with "init:"). */
export async function listContainers(namespace: string, pod: string): Promise<string[]> {
  try {
    return await invoke<string[]>('list_containers', { namespace, pod });
  } catch {
    return [];
  }
}

/** Start a streaming log tail. In Tauri, emits 'log-chunk' events. */
export async function startLogStream(
  namespace: string,
  pod: string,
  container?: string,
  tailLines?: number,
): Promise<void> {
  await invoke<void>('start_log_stream', {
    namespace,
    pod,
    container: container ?? null,
    tailLines: tailLines ?? 0,
  });
}

/** Result of a non-interactive exec command. */
export interface ExecResult {
  stdout: string;
  stderr: string;
  success: boolean;
}

/** Execute a command in a running container (non-interactive). */
export async function execCommand(
  namespace: string,
  pod: string,
  container: string | undefined,
  command: string[],
): Promise<ExecResult> {
  return invoke<ExecResult>('exec_command', {
    namespace,
    pod,
    container: container ?? null,
    command,
  });
}


/** Result of applying a resource. */
export interface ApplyResult {
  success: boolean;
  message: string;
}

/** Apply (create or update) a Kubernetes resource from a JSON/YAML manifest. */
export async function applyResource(manifest: string, dryRun = false): Promise<ApplyResult> {
  return invoke<ApplyResult>('apply_resource', { manifest, dryRun });
}

/** Scale a Deployment or StatefulSet to the desired replica count. */
export async function scaleResource(gvk: string, namespace: string, name: string, replicas: number): Promise<string> {
  return invoke<string>('scale_resource', { gvk, namespace, name, replicas });
}

/** Start a port-forward session to a pod. Returns the local port number. */
export async function startPortForward(namespace: string, pod: string, localPort: number, remotePort: number): Promise<number> {
  return invoke<number>('start_port_forward', { namespace, pod, localPort, remotePort });
}

/** Fetch pod-level CPU/memory metrics from the metrics-server API. */
export async function getPodMetrics(namespace?: string): Promise<PodMetrics[]> {
  try {
    return await invoke<PodMetrics[]>('get_pod_metrics', { namespace: namespace ?? null });
  } catch {
    return [];
  }
}

/** Check whether the metrics-server API is reachable on the cluster. */
export async function checkMetricsAvailable(): Promise<boolean> {
  try {
    return await invoke<boolean>('check_metrics_available');
  } catch {
    return false;
  }
}

/** List Helm releases across all namespaces (or a specific one). */
export async function listHelmReleases(namespace?: string): Promise<HelmRelease[]> {
  try {
    return await invoke<HelmRelease[]>('list_helm_releases', { namespace: namespace ?? null });
  } catch {
    return [];
  }
}

/** Get all revisions of a specific Helm release, sorted by revision number. */
export async function getHelmReleaseHistory(namespace: string, name: string): Promise<HelmRelease[]> {
  try {
    return await invoke<HelmRelease[]>('get_helm_release_history', { namespace, name });
  } catch {
    return [];
  }
}

/** Get user-supplied values for the latest revision of a Helm release.
 *  Sensitive keys are redacted by default; pass `reveal: true` to see raw values. */
export async function getHelmReleaseValues(namespace: string, name: string, reveal = false): Promise<string> {
  return invoke<string>('get_helm_release_values', { namespace, name, reveal });
}

/** Roll back a Helm release to a specific revision using the helm CLI. */
export async function helmRollback(namespace: string, name: string, revision: number): Promise<string> {
  return invoke<string>('helm_rollback', { namespace, name, revision });
}

/** Fetch node-level CPU/memory metrics with allocatable percentages. */
export async function getNodeMetrics(): Promise<NodeMetricsData[]> {
  try {
    return await invoke<NodeMetricsData[]>('get_node_metrics');
  } catch {
    return [];
  }
}
