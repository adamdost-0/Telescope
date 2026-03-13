import { env as publicEnv } from '$env/dynamic/public';
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
    ? ((window as Window & { __TELESCOPE_HUB_URL__?: string }).__TELESCOPE_HUB_URL__ ??
      publicEnv.PUBLIC_ENGINE_HTTP_BASE ??
      'http://localhost:3001')
    : (publicEnv.PUBLIC_ENGINE_HTTP_BASE ?? 'http://localhost:3001');

type HubErrorResponse = { error?: string };
const CLUSTER_SCOPED_NAMESPACE = '_cluster';

async function readHubError(res: Response): Promise<Error> {
  let message = `Request failed (${res.status})`;
  try {
    const payload = (await res.json()) as HubErrorResponse;
    if (payload?.error) {
      message = payload.error;
    }
  } catch {
    // Ignore malformed error bodies and fall back to the status-based message.
  }
  return new Error(message);
}

async function expectJson<T>(res: Response): Promise<T> {
  if (!res.ok) {
    throw await readHubError(res);
  }
  return (await res.json()) as T;
}

async function expectOk(res: Response): Promise<void> {
  if (!res.ok) {
    throw await readHubError(res);
  }
}

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
    case 'count_resources': {
      const params = new URLSearchParams();
      if (args?.gvk) params.set('gvk', args.gvk as string);
      if (args?.namespace) params.set('namespace', args.namespace as string);
      const res = await fetch(`${base}/resources?${params}`);
      if (!res.ok) {
        throw new Error(`Failed to count resources (${res.status})`);
      }
      const resources = (await res.json()) as ResourceEntry[];
      return resources.length as T;
    }
    case 'list_dynamic_resources': {
      const params = new URLSearchParams();
      if (args?.namespace) params.set('namespace', args.namespace as string);
      const res = await fetch(
        `${base}/dynamic/${encodeURIComponent(args?.group as string)}/${encodeURIComponent(args?.version as string)}/${encodeURIComponent(args?.plural as string)}?${params}`
      );
      return await expectJson<T>(res);
    }
    case 'get_dynamic_resource': {
      const namespace = typeof args?.namespace === 'string' && args.namespace.length > 0
        ? encodeURIComponent(args.namespace as string)
        : CLUSTER_SCOPED_NAMESPACE;
      const res = await fetch(
        `${base}/dynamic/${encodeURIComponent(args?.group as string)}/${encodeURIComponent(args?.version as string)}/${encodeURIComponent(args?.plural as string)}/${namespace}/${encodeURIComponent(args?.name as string)}`
      );
      return await expectJson<T>(res);
    }
    case 'get_resource': {
      const params = new URLSearchParams();
      if (args?.gvk) params.set('gvk', args.gvk as string);
      if (args?.namespace) params.set('namespace', args.namespace as string);
      const res = await fetch(`${base}/resources?${params}`);
      if (!res.ok) {
        throw new Error(`Failed to load resource (${res.status})`);
      }
      const resources = (await res.json()) as ResourceEntry[];
      return (resources.find((resource) => resource.name === args?.name) ?? null) as T;
    }
    case 'get_secrets': {
      const params = new URLSearchParams();
      if (args?.namespace) params.set('namespace', args.namespace as string);
      const res = await fetch(`${base}/secrets?${params}`);
      if (!res.ok) {
        throw new Error(`Failed to load secrets (${res.status})`);
      }
      return (await res.json()) as T;
    }
    case 'get_secret': {
      const res = await fetch(
        `${base}/secrets/${encodeURIComponent(args?.namespace as string)}/${encodeURIComponent(args?.name as string)}`
      );
      if (res.status === 404) {
        return null as T;
      }
      if (!res.ok) {
        throw new Error(`Failed to load secret (${res.status})`);
      }
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
      return await expectJson<T>(res);
    }
    case 'create_namespace': {
      const res = await fetch(`${base}/namespaces/create`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: args?.name })
      });
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'delete_namespace': {
      const res = await fetch(`${base}/namespaces/${encodeURIComponent(args?.name as string)}`, {
        method: 'DELETE'
      });
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'get_namespace': {
      const res = await fetch(`${base}/namespace`);
      return await expectJson<T>(res);
    }
    case 'get_pod_logs': {
      const params = new URLSearchParams();
      if (args?.container) params.set('container', args.container as string);
      if (args?.tailLines) params.set('tail', String(args.tailLines));
      if (args?.previous) params.set('previous', 'true');
      const res = await fetch(`${base}/pods/${args?.namespace}/${args?.pod}/logs?${params}`);
      if (!res.ok) {
        throw new Error(`Failed to load pod logs (${res.status})`);
      }
      const data = (await res.json()) as { logs?: string };
      return (data.logs ?? '') as T;
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
      const res = await fetch(`${base}/metrics/available`);
      return await expectJson<T>(res);
    }
    case 'list_crds': {
      const res = await fetch(`${base}/crds`);
      return (await res.json()) as T;
    }
    case 'get_resource_counts': {
      const res = await fetch(`${base}/resource-counts`);
      return await expectJson<T>(res);
    }
    case 'set_namespace': {
      const res = await fetch(`${base}/namespace`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ namespace: args?.namespace })
      });
      await expectOk(res);
      return undefined as T;
    }
    case 'scale_resource': {
      const res = await fetch(`${base}/resources/scale`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          gvk: args?.gvk,
          namespace: args?.namespace,
          name: args?.name,
          replicas: args?.replicas
        })
      });
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'delete_resource': {
      const res = await fetch(`${base}/resources/delete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          gvk: args?.gvk,
          namespace: args?.namespace,
          name: args?.name
        })
      });
      const data = await expectJson<{ success: boolean; message: string }>(res);
      if (!data.success) {
        throw new Error(data.message);
      }
      return data.message as T;
    }
    case 'apply_resource': {
      const res = await fetch(`${base}/resources/apply`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          manifest: args?.json_content,
          dry_run: args?.dry_run
        })
      });
      return await expectJson<T>(res);
    }
    case 'apply_dynamic_resource': {
      const res = await fetch(`${base}/dynamic/apply`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          group: args?.group,
          version: args?.version,
          kind: args?.kind,
          plural: args?.plural,
          namespace: args?.namespace,
          manifest: args?.manifest,
          dry_run: args?.dry_run,
        })
      });
      return await expectJson<T>(res);
    }
    case 'delete_dynamic_resource': {
      const res = await fetch(`${base}/dynamic/delete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          group: args?.group,
          version: args?.version,
          kind: args?.kind,
          plural: args?.plural,
          namespace: args?.namespace,
          name: args?.name,
        })
      });
      const data = await expectJson<{ success: boolean; message: string }>(res);
      if (!data.success) {
        throw new Error(data.message);
      }
      return data.message as T;
    }
    case 'rollout_restart': {
      const res = await fetch(`${base}/rollout/restart`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          gvk: args?.gvk,
          namespace: args?.namespace,
          name: args?.name
        })
      });
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'rollout_status': {
      const params = new URLSearchParams({
        gvk: String(args?.gvk ?? ''),
        namespace: String(args?.namespace ?? ''),
        name: String(args?.name ?? '')
      });
      const res = await fetch(`${base}/rollout/status?${params}`);
      return await expectJson<T>(res);
    }
    case 'start_port_forward': {
      const res = await fetch(`${base}/port-forward`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          namespace: args?.namespace,
          pod: args?.pod,
          local_port: args?.localPort,
          remote_port: args?.remotePort
        })
      });
      const data = await expectJson<{ local_port: number }>(res);
      return data.local_port as T;
    }
    case 'exec_command': {
      const res = await fetch(`${base}/exec`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          namespace: args?.namespace,
          pod: args?.pod,
          container: args?.container,
          command: args?.command
        })
      });
      return await expectJson<T>(res);
    }
    case 'helm_rollback': {
      const res = await fetch(
        `${base}/helm/releases/${encodeURIComponent(args?.namespace as string)}/${encodeURIComponent(args?.name as string)}/rollback`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ revision: args?.revision })
        }
      );
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'get_helm_release_history': {
      const res = await fetch(
        `${base}/helm/releases/${encodeURIComponent(args?.namespace as string)}/${encodeURIComponent(args?.name as string)}/history`
      );
      return await expectJson<T>(res);
    }
    case 'get_helm_release_values': {
      const params = new URLSearchParams({ reveal: String(Boolean(args?.reveal)) });
      const res = await fetch(
        `${base}/helm/releases/${encodeURIComponent(args?.namespace as string)}/${encodeURIComponent(args?.name as string)}/values?${params}`
      );
      return await expectJson<T>(res);
    }
    case 'list_containers': {
      const res = await fetch(
        `${base}/containers/${encodeURIComponent(args?.namespace as string)}/${encodeURIComponent(args?.pod as string)}`
      );
      return await expectJson<T>(res);
    }
    case 'start_log_stream':
      return undefined as unknown as T;
    case 'active_context': {
      const res = await fetch(`${base}/active-context`);
      return await expectJson<T>(res);
    }
    case 'get_node_metrics': {
      const res = await fetch(`${base}/metrics/nodes`);
      return await expectJson<T>(res);
    }
    case 'get_preference': {
      const res = await fetch(`${base}/preferences/${encodeURIComponent(args?.key as string)}`);
      return await expectJson<T>(res);
    }
    case 'set_preference': {
      const res = await fetch(`${base}/preferences/${encodeURIComponent(args?.key as string)}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ value: args?.value })
      });
      await expectOk(res);
      return undefined as T;
    }
    case 'cordon_node': {
      const res = await fetch(
        `${base}/nodes/${encodeURIComponent(args?.name as string)}/cordon`,
        { method: 'POST' }
      );
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'uncordon_node': {
      const res = await fetch(
        `${base}/nodes/${encodeURIComponent(args?.name as string)}/uncordon`,
        { method: 'POST' }
      );
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'drain_node': {
      const res = await fetch(
        `${base}/nodes/${encodeURIComponent(args?.name as string)}/drain`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            grace_period: args?.grace_period ?? 30,
            ignore_daemonsets: args?.ignore_daemonsets ?? true,
            force: args?.force ?? false
          })
        }
      );
      return await expectJson<T>(res);
    }
    case 'add_node_taint': {
      const res = await fetch(
        `${base}/nodes/${encodeURIComponent(args?.name as string)}/taints`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ key: args?.key, value: args?.value, effect: args?.effect })
        }
      );
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }
    case 'remove_node_taint': {
      const res = await fetch(
        `${base}/nodes/${encodeURIComponent(args?.name as string)}/taints/${encodeURIComponent(args?.key as string)}`,
        { method: 'DELETE' }
      );
      const data = await expectJson<{ message: string }>(res);
      return data.message as T;
    }

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

/** Count resources for a specific GVK, optionally scoped to a namespace. */
export async function countResources(gvk: string, namespace?: string): Promise<number> {
  try {
    return await invoke<number>('count_resources', { gvk, namespace: namespace ?? null });
  } catch {
    return 0;
  }
}

export async function listDynamicResources(
  group: string,
  version: string,
  plural: string,
  namespace?: string | null,
): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('list_dynamic_resources', {
      group,
      version,
      plural,
      namespace: namespace ?? null,
    });
  } catch {
    return [];
  }
}

export async function getDynamicResource(
  group: string,
  version: string,
  plural: string,
  namespace: string | null,
  name: string,
): Promise<ResourceEntry | null> {
  try {
    return await invoke<ResourceEntry | null>('get_dynamic_resource', {
      group,
      version,
      plural,
      namespace,
      name,
    });
  } catch {
    return null;
  }
}

/** Fetch a single resource by GVK, namespace, and name. */
export async function getResource(
  gvk: string,
  namespace: string,
  name: string,
): Promise<ResourceEntry | null> {
  try {
    return await invoke<ResourceEntry | null>('get_resource', { gvk, namespace, name });
  } catch {
    return null;
  }
}

/** Fetch secrets on demand without reading from the shared resource cache. */
export async function getSecrets(namespace: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_secrets', { namespace });
  } catch {
    return [];
  }
}

/** Fetch one secret on demand without reading from the shared resource cache. */
export async function getSecret(namespace: string, name: string): Promise<ResourceEntry | null> {
  try {
    return await invoke<ResourceEntry | null>('get_secret', { namespace, name });
  } catch {
    return null;
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

export async function createNamespace(name: string): Promise<string> {
  return invoke<string>('create_namespace', { name });
}

export async function deleteNamespace(name: string): Promise<string> {
  return invoke<string>('delete_namespace', { name });
}

/** Get the current active namespace. */
export async function getNamespace(): Promise<string> {
  try {
    return await invoke<string>('get_namespace');
  } catch {
    return 'default';
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

export async function applyDynamicResource(
  group: string,
  version: string,
  kind: string,
  plural: string,
  namespace: string | null,
  manifest: string,
  dryRun = false,
): Promise<ApplyResult> {
  return invoke<ApplyResult>('apply_dynamic_resource', {
    group,
    version,
    kind,
    plural,
    namespace,
    manifest,
    dry_run: dryRun,
  });
}

export async function deleteDynamicResource(
  group: string,
  version: string,
  kind: string,
  plural: string,
  namespace: string | null,
  name: string,
): Promise<string> {
  return invoke<string>('delete_dynamic_resource', {
    group,
    version,
    kind,
    plural,
    namespace,
    name,
  });
}

/** Apply (create or update) a Kubernetes resource from a JSON/YAML manifest. */
export async function applyResource(manifest: string, dryRun = false): Promise<ApplyResult> {
  return invoke<ApplyResult>('apply_resource', { json_content: manifest, dry_run: dryRun });
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

// ── Node operations ──────────────────────────────────────────────────────

/** Cordon a node (mark as unschedulable). */
export async function cordonNode(name: string): Promise<string> {
  return invoke<string>('cordon_node', { name });
}

/** Uncordon a node (mark as schedulable). */
export async function uncordonNode(name: string): Promise<string> {
  return invoke<string>('uncordon_node', { name });
}

/** Drain options for drainNode(). */
export interface DrainOptions {
  grace_period?: number;
  ignore_daemonsets?: boolean;
  force?: boolean;
}

/** Result of a drain operation. */
export interface DrainResult {
  success: boolean;
  message: string;
  evicted_pods: string[];
  skipped_pods: string[];
}

/** Drain a node: cordon then evict eligible pods. */
export async function drainNode(name: string, options?: DrainOptions): Promise<DrainResult> {
  return invoke<DrainResult>('drain_node', {
    name,
    grace_period: options?.grace_period ?? 30,
    ignore_daemonsets: options?.ignore_daemonsets ?? true,
    force: options?.force ?? false
  });
}

/** Add a taint to a node. */
export async function addNodeTaint(name: string, key: string, value: string, effect: string): Promise<string> {
  return invoke<string>('add_node_taint', { name, key, value, effect });
}

/** Remove a taint from a node by key. */
export async function removeNodeTaint(name: string, key: string): Promise<string> {
  return invoke<string>('remove_node_taint', { name, key });
}
