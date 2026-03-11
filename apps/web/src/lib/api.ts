/**
 * Unified API layer that works in both Tauri (desktop) and browser (web) contexts.
 * Desktop: Uses Tauri invoke for IPC.
 * Web: Falls back to HTTP fetch against the engine API.
 */
import {
  isTauri,
  type ClusterContext,
  type ResourceEntry,
  type ConnectionState,
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
      })) as unknown as T;
    }
    case 'get_connection_state':
      return { state: 'Disconnected' } as unknown as T;
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
    default:
      throw new Error(`Command "${command}" not available in web mode`);
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

/** Fetch events for a given namespace, optionally filtered by involved object name. */
export async function getEvents(namespace: string, involvedObject?: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_events', {
      namespace,
      involvedObject: involvedObject ?? null,
    });
  } catch {
    return [];
  }
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
