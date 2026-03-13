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
  subscription_id: string | null;
  resource_group: string | null;
  azure_resource_id: string | null;
}

export interface AksIdentityInfo {
  subscription_id: string;
  resource_group: string;
  cluster_name: string;
  arm_resource_id: string;
}

export interface PowerState {
  code: string | null;
}

export interface AksNodePool {
  name: string;
  vm_size: string | null;
  count: number | null;
  os_type: string | null;
  os_disk_size_gb: number | null;
  mode: string | null;
  orchestrator_version: string | null;
  enable_auto_scaling: boolean | null;
  min_count: number | null;
  max_count: number | null;
  availability_zones: string[] | null;
  node_labels: unknown;
  node_taints: string[] | null;
  provisioning_state: string | null;
  power_state: PowerState | null;
  max_pods: number | null;
  node_image_version: string | null;
  vnet_subnet_id: string | null;
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

export interface CrdInfo {
  /** Full CRD name, e.g. "certificates.cert-manager.io" */
  name: string;
  /** API group, e.g. "cert-manager.io" */
  group: string;
  /** Kind, e.g. "Certificate" */
  kind: string;
  /** Served version, e.g. "v1" */
  version: string;
  /** "Namespaced" or "Cluster" */
  scope: string;
  /** Plural resource name, e.g. "certificates" */
  plural: string;
  /** Short names for kubectl */
  short_names: string[];
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

export interface AksClusterDetail {
  kubernetesVersion: string | null;
  provisioningState: string | null;
  powerState: { code: string | null } | null;
  fqdn: string | null;
  dnsPrefix: string | null;
  sku: { name: string | null; tier: string | null } | null;
  networkProfile: {
    networkPlugin: string | null;
    networkPolicy: string | null;
    serviceCidr: string | null;
    podCidr: string | null;
    dnsServiceIp: string | null;
    outboundType: string | null;
    loadBalancerSku: string | null;
  } | null;
  apiServerAccessProfile: {
    authorizedIpRanges: string[] | null;
    enablePrivateCluster: boolean | null;
  } | null;
  identity: {
    type_: string | null;
    principalId: string | null;
    tenantId: string | null;
  } | null;
  addonProfiles: Record<string, unknown> | null;
  autoUpgradeProfile: {
    upgradeChannel: string | null;
    nodeOsUpgradeChannel: string | null;
  } | null;
  oidcIssuerProfile: {
    enabled: boolean | null;
    issuerUrl: string | null;
  } | null;
  securityProfile: {
    workloadIdentity: { enabled: boolean | null } | null;
  } | null;
}

/**
 * Check if running inside Tauri (desktop) or browser (web).
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
