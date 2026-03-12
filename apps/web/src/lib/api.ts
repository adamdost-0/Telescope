/**
 * Unified API layer that works in both Tauri (desktop) and browser (web) contexts.
 * Desktop: Uses Tauri invoke for IPC.
 * Web: Falls back to HTTP fetch against the engine API.
 */
import {
  isTauri,
  type ClusterContext,
  type ClusterInfo,
  type CrdInfo,
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

const HUB_URL =
  typeof window !== 'undefined'
    ? (window as any).__TELESCOPE_HUB_URL__ || 'http://localhost:3001'
    : 'http://localhost:3001';

async function webFallback<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const base = `${HUB_URL}/api/v1`;

  switch (command) {
    // ── Read operations (hub-backed) ──────────────────────────────────────
    case 'list_contexts': {
      const res = await fetch(`${base}/contexts`);
      return (await res.json()) as T;
    }
    case 'connect_to_context': {
      const res = await fetch(`${base}/connect`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ contextName: args?.contextName }),
      });
      return (await res.json()) as T;
    }
    case 'disconnect': {
      await fetch(`${base}/disconnect`, { method: 'POST' });
      return undefined as T;
    }
    case 'get_connection_state': {
      const res = await fetch(`${base}/connection-state`);
      return (await res.json()) as T;
    }
    case 'get_pods': {
      const ns = args?.namespace ? `?namespace=${args.namespace}` : '';
      const res = await fetch(`${base}/pods${ns}`);
      return (await res.json()) as T;
    }
    case 'get_resources': {
      const params = new URLSearchParams();
      if (args?.gvk) params.set('gvk', args.gvk as string);
      if (args?.namespace) params.set('namespace', args.namespace as string);
      const res = await fetch(`${base}/resources?${params}`);
      return (await res.json()) as T;
    }
    case 'get_events': {
      const params = new URLSearchParams();
      if (args?.namespace) params.set('namespace', args.namespace as string);
      if (args?.involvedObject) params.set('involved_object', args.involvedObject as string);
      const res = await fetch(`${base}/events?${params}`);
      return (await res.json()) as T;
    }
    case 'list_namespaces': {
      const res = await fetch(`${base}/namespaces`);
      return (await res.json()) as T;
    }
    case 'get_pod_logs': {
      const params = new URLSearchParams();
      if (args?.container) params.set('container', args.container as string);
      if (args?.tailLines) params.set('tail', String(args.tailLines));
      if (args?.previous) params.set('previous', 'true');
      const res = await fetch(`${base}/pods/${args?.namespace}/${args?.pod}/logs?${params}`);
      return (await res.text()) as T;
    }
    case 'get_cluster_info': {
      const res = await fetch(`${base}/cluster-info`);
      return (await res.json()) as T;
    }
    case 'search_resources': {
      const res = await fetch(`${base}/search?q=${encodeURIComponent(args?.query as string)}`);
      return (await res.json()) as T;
    }
    case 'list_helm_releases': {
      const ns = args?.namespace ? `?namespace=${args.namespace}` : '';
      const res = await fetch(`${base}/helm/releases${ns}`);
      return (await res.json()) as T;
    }
    case 'get_pod_metrics': {
      const ns = args?.namespace ? `?namespace=${args.namespace}` : '';
      const res = await fetch(`${base}/metrics/pods${ns}`);
      return (await res.json()) as T;
    }
    case 'check_metrics_available': {
      try {
        const res = await fetch(`${base}/metrics/pods`);
        return res.ok as unknown as T;
      } catch {
        return false as unknown as T;
      }
    }
    case 'list_crds': {
      const res = await fetch(`${base}/crds`);
      return (await res.json()) as T;
    }
    case 'get_resource_counts':
      // Hub doesn't have this endpoint yet — compute client-side
      return [] as unknown as T;

    // ── Write operations (deferred to next iteration) ─────────────────────
    case 'set_namespace':
    case 'scale_resource':
    case 'delete_resource':
    case 'apply_resource':
    case 'rollout_restart':
    case 'rollout_status':
    case 'start_port_forward':
    case 'exec_command':
    case 'helm_rollback':
    case 'get_helm_release_history':
    case 'get_helm_release_values':
    case 'list_containers':
    case 'start_log_stream':
    case 'active_context':
    case 'get_node_metrics':
    case 'get_preference':
    case 'set_preference':
      return undefined as unknown as T;

    default:
      console.warn(`[telescope] No hub mapping for command: ${command}`);
      return undefined as unknown as T;
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

/** Search across all cached resource types by name or GVK substring match (max 20 results). */
export async function searchResources(query: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('search_resources', { query });
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

/** Restart a Deployment, StatefulSet, or DaemonSet rollout. */
export async function rolloutRestart(gvk: string, namespace: string, name: string): Promise<string> {
  return invoke<string>('rollout_restart', { gvk, namespace, name });
}

/** Get rollout status for a Deployment or StatefulSet. */
export async function rolloutStatus(gvk: string, namespace: string, name: string): Promise<RolloutStatus> {
  return invoke<RolloutStatus>('rollout_status', { gvk, namespace, name });
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

/** List all Custom Resource Definitions installed on the cluster. */
export async function listCrds(): Promise<CrdInfo[]> {
  try {
    return await invoke<CrdInfo[]>('list_crds');
  } catch {
    return [];
  }
}

/** Fetch node-level CPU/memory metrics with allocatable percentages. */
export async function getNodeMetrics(): Promise<NodeMetricsData[]> {
  try {
    return await invoke<NodeMetricsData[]>('get_node_metrics');
  } catch {
    return [];
  }
}

// ── User preferences ─────────────────────────────────────────────────────

/** Read a single user preference by key. */
export async function getPreference(key: string): Promise<string | null> {
  try {
    return await invoke<string | null>('get_preference', { key });
  } catch {
    return null;
  }
}

/** Write a single user preference. */
export async function setPreference(key: string, value: string): Promise<void> {
  await invoke<void>('set_preference', { key, value });
}
