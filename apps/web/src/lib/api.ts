/**
 * Tauri-only API layer for desktop IPC.
 * All calls go through Tauri invoke — no HTTP fallback.
 */
import {
  type ClusterContext,
  type ClusterInfo,
  type CrdInfo,
  type HelmRelease,
  type ResourceEntry,
  type ConnectionState,
  type PodMetrics,
  type NodeMetricsData,
  type AksIdentityInfo,
  type AksNodePool,
  type AksClusterDetail,
  type AksUpgradeProfile,
  type AksMaintenanceConfig,
  type PoolUpgradeProfile,
} from './tauri-commands';

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
  try {
    const result = await tauriInvoke<T>(command, args);
    return result;
  } catch (e) {
    console.error(`[telescope] invoke ${command} FAILED:`, e);
    throw e;
  }
}

// ── API error notification ───────────────────────────────────────────────

type ApiErrorListener = (error: { command: string; message: string }) => void;
const errorListeners: ApiErrorListener[] = [];

/** Subscribe to API errors that are caught and suppressed by helpers. Returns an unsubscribe function. */
export function onApiError(listener: ApiErrorListener): () => void {
  errorListeners.push(listener);
  return () => {
    const idx = errorListeners.indexOf(listener);
    if (idx >= 0) errorListeners.splice(idx, 1);
  };
}

function notifyApiError(command: string, error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[telescope] ${command} failed:`, message);
  for (const listener of errorListeners) {
    listener({ command, message });
  }
}

const AZURE_CLOUD_STORAGE_KEY = 'telescope-azure-cloud';
const AZURE_CLOUD_SELECTION_STORAGE_KEY = 'telescope-azure-cloud-selection';

export function isTauriDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** Fetch counts for all major resource types. */
export async function getResourceCounts(): Promise<[string, number][]> {
  try {
    return await invoke<[string, number][]>('get_resource_counts');
  } catch (e) {
    notifyApiError('get_resource_counts', e);
    return [];
  }
}

/** Search across all cached resource types by name or GVK substring match (max 20 results). */
export async function searchResources(query: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('search_resources', { query });
  } catch (e) {
    notifyApiError('search_resources', e);
    return [];
  }
}

export async function listContexts(): Promise<ClusterContext[]> {
  try {
    return await invoke<ClusterContext[]>('list_contexts');
  } catch (e) {
    notifyApiError('list_contexts', e);
    return [];
  }
}

export async function activeContext(): Promise<string | null> {
  try {
    return await invoke<string>('active_context');
  } catch (e) {
    notifyApiError('active_context', e);
    return null;
  }
}

export async function getConnectionState(): Promise<ConnectionState> {
  try {
    return await invoke<ConnectionState>('get_connection_state');
  } catch (e) {
    notifyApiError('get_connection_state', e);
    return { state: 'Disconnected' };
  }
}

/** Fetch cluster version, auth, and AKS info for the connected context. */
export async function getClusterInfo(): Promise<ClusterInfo | null> {
  try {
    return await invoke<ClusterInfo>('get_cluster_info');
  } catch (e) {
    notifyApiError('get_cluster_info', e);
    return null;
  }
}

export async function getPods(namespace?: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_pods', { namespace: namespace ?? null });
  } catch (e) {
    notifyApiError('get_pods', e);
    return [];
  }
}

/** Fetch resources of any GVK from the SQLite store. */
export async function getResources(gvk: string, namespace?: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_resources', { gvk, namespace: namespace ?? null });
  } catch (e) {
    notifyApiError('get_resources', e);
    return [];
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
  } catch (e) {
    notifyApiError('list_dynamic_resources', e);
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
  } catch (e) {
    notifyApiError('get_dynamic_resource', e);
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
  } catch (e) {
    notifyApiError('get_resource', e);
    return null;
  }
}

/** Fetch secrets on demand without reading from the shared resource cache. */
export async function getSecrets(namespace: string): Promise<ResourceEntry[]> {
  try {
    return await invoke<ResourceEntry[]>('get_secrets', { namespace });
  } catch (e) {
    notifyApiError('get_secrets', e);
    return [];
  }
}

/** Fetch one secret on demand without reading from the shared resource cache. */
export async function getSecret(namespace: string, name: string): Promise<ResourceEntry | null> {
  try {
    return await invoke<ResourceEntry | null>('get_secret', { namespace, name });
  } catch (e) {
    notifyApiError('get_secret', e);
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
  } catch (e) {
    notifyApiError('get_events', e);
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
  } catch (e) {
    notifyApiError('list_namespaces', e);
    return ['default'];
  }
}

export async function createNamespace(name: string): Promise<string> {
  return invoke<string>('create_namespace', { name });
}

export async function deleteNamespace(name: string): Promise<string> {
  return invoke<string>('delete_namespace', { name });
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
  } catch (e) {
    notifyApiError('get_pod_logs', e);
    return '';
  }
}

/** List container names for a pod (init containers prefixed with "init:"). */
export async function listContainers(namespace: string, pod: string): Promise<string[]> {
  try {
    return await invoke<string[]>('list_containers', { namespace, pod });
  } catch (e) {
    notifyApiError('list_containers', e);
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
interface ExecResult {
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
interface ApplyResult {
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
  } catch (e) {
    notifyApiError('get_pod_metrics', e);
    return [];
  }
}

/** Check whether the metrics-server API is reachable on the cluster. */
export async function checkMetricsAvailable(): Promise<boolean> {
  try {
    return await invoke<boolean>('check_metrics_available');
  } catch (e) {
    notifyApiError('check_metrics_available', e);
    return false;
  }
}

/** List Helm releases across all namespaces (or a specific one). */
export async function listHelmReleases(namespace?: string): Promise<HelmRelease[]> {
  try {
    return await invoke<HelmRelease[]>('list_helm_releases', { namespace: namespace ?? null });
  } catch (e) {
    notifyApiError('list_helm_releases', e);
    return [];
  }
}

/** Get all revisions of a specific Helm release, sorted by revision number. */
export async function getHelmReleaseHistory(namespace: string, name: string): Promise<HelmRelease[]> {
  try {
    return await invoke<HelmRelease[]>('get_helm_release_history', { namespace, name });
  } catch (e) {
    notifyApiError('get_helm_release_history', e);
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
  } catch (e) {
    notifyApiError('list_crds', e);
    return [];
  }
}

/** Fetch node-level CPU/memory metrics with allocatable percentages. */
export async function getNodeMetrics(): Promise<NodeMetricsData[]> {
  try {
    return await invoke<NodeMetricsData[]>('get_node_metrics');
  } catch (e) {
    notifyApiError('get_node_metrics', e);
    return [];
  }
}

// ── User preferences ─────────────────────────────────────────────────────

/** Read a single user preference by key. */
export async function getPreference(key: string): Promise<string | null> {
  try {
    return await invoke<string | null>('get_preference', { key });
  } catch (e) {
    notifyApiError('get_preference', e);
    return null;
  }
}

/** Write a single user preference. */
export async function setPreference(key: string, value: string): Promise<void> {
  await invoke<void>('set_preference', { key, value });
}

// ── AKS identity resolution ─────────────────────────────────────────────

/** Resolve AKS resource identity (subscription, RG, cluster name) for the active context. */
export async function resolveAksIdentity(): Promise<AksIdentityInfo | null> {
  try {
    return await invoke<AksIdentityInfo | null>('resolve_aks_identity');
  } catch (e) {
    notifyApiError('resolve_aks_identity', e);
    return null;
  }
}

/** List authoritative AKS node pools from the Azure ARM API. */
export async function listAksNodePools(): Promise<AksNodePool[]> {
  try {
    return await invoke<AksNodePool[]>('list_aks_node_pools');
  } catch (e) {
    notifyApiError('list_aks_node_pools', e);
    return [];
  }
}

/** Scale an AKS node pool to a target node count. */
export async function scaleAksNodePool(poolName: string, count: number): Promise<AksNodePool> {
  return invoke<AksNodePool>('scale_aks_node_pool', { poolName, count });
}

/** Update autoscaler settings on an AKS node pool. */
export async function updateAksAutoscaler(
  poolName: string,
  enabled: boolean,
  min: number | null,
  max: number | null,
): Promise<AksNodePool> {
  return invoke<AksNodePool>('update_aks_autoscaler', { poolName, enabled, min, max });
}

/** Create node pool config for the create API. */
export interface CreateNodePoolConfig {
  name: string;
  vmSize: string;
  count: number;
  osType?: string;
  mode?: string;
  orchestratorVersion?: string;
  enableAutoScaling?: boolean;
  minCount?: number;
  maxCount?: number;
  availabilityZones?: string[];
  maxPods?: number;
  nodeLabels?: Record<string, string>;
  nodeTaints?: string[];
}

/** Create a new AKS node pool via the Azure ARM API. */
export async function createAksNodePool(config: CreateNodePoolConfig): Promise<AksNodePool> {
  return invoke<AksNodePool>('create_aks_node_pool', { config });
}

/** Delete an AKS node pool. */
export async function deleteAksNodePool(poolName: string): Promise<void> {
  return invoke<void>('delete_aks_node_pool', { poolName });
}

/** Fetch comprehensive AKS cluster details from the Azure ARM API. */
export async function getAksClusterDetail(): Promise<AksClusterDetail | null> {
  try {
    return await invoke<AksClusterDetail | null>('get_aks_cluster_detail');
  } catch (e) {
    notifyApiError('get_aks_cluster_detail', e);
    return null;
  }
}

/** Start the active AKS cluster via Azure ARM. */
export async function startAksCluster(): Promise<void> {
  await invoke<void>('start_aks_cluster');
}

/** Stop the active AKS cluster via Azure ARM. */
export async function stopAksCluster(): Promise<void> {
  await invoke<void>('stop_aks_cluster');
}

/** Fetch available control plane upgrades for the active AKS cluster. */
export async function getAksUpgradeProfile(): Promise<AksUpgradeProfile> {
  return invoke<AksUpgradeProfile>('get_aks_upgrade_profile');
}

/** Trigger a control plane upgrade for the active AKS cluster. */
export async function upgradeAksCluster(targetVersion: string): Promise<void> {
  await invoke<void>('upgrade_aks_cluster', { targetVersion });
}

/** Fetch available upgrades for an AKS node pool. */
export async function getPoolUpgradeProfile(pool: string): Promise<PoolUpgradeProfile> {
  return invoke<PoolUpgradeProfile>('get_pool_upgrade_profile', { pool });
}

/** Upgrade an AKS node pool Kubernetes version. */
export async function upgradePoolVersion(pool: string, version: string): Promise<void> {
  await invoke<void>('upgrade_pool_version', { pool, version });
}

/** Upgrade an AKS node pool node image. */
export async function upgradePoolNodeImage(pool: string): Promise<void> {
  await invoke<void>('upgrade_pool_node_image', { pool });
}

/** Fetch AKS maintenance configurations from the Azure ARM API. */
export async function listAksMaintenanceConfigs(): Promise<AksMaintenanceConfig[]> {
  try {
    return await invoke<AksMaintenanceConfig[]>('list_aks_maintenance_configs');
  } catch (e) {
    notifyApiError('list_aks_maintenance_configs', e);
    return [];
  }
}

/** Get the effective Azure cloud, using desktop auto-detection when available. */
export async function getAzureCloud(): Promise<string> {
  try {
    const cloud = await invoke<string>('get_azure_cloud');
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(AZURE_CLOUD_STORAGE_KEY, cloud);
    }
    return cloud;
  } catch {
    if (typeof localStorage !== 'undefined') {
      return localStorage.getItem(AZURE_CLOUD_STORAGE_KEY) ?? 'Commercial';
    }
    return 'Commercial';
  }
}

/** Persist the preferred Azure cloud selection. */
export async function setAzureCloud(cloud: string): Promise<void> {
  const tauriDesktop = isTauriDesktop();

  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(AZURE_CLOUD_SELECTION_STORAGE_KEY, cloud);
    if (cloud !== 'auto') {
      localStorage.setItem(AZURE_CLOUD_STORAGE_KEY, cloud);
    }
  }

  try {
    await invoke<void>('set_azure_cloud', { cloud });
  } catch (error) {
    if (tauriDesktop || typeof localStorage === 'undefined') {
      throw error;
    }
  }
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
