import type { Page } from '@playwright/test';
import type {
  AiInsightsConnectionTestResult,
  AiInsightsResponse,
  AiInsightsScope,
  AksNodePool,
  PoolUpgradeProfile,
  ClusterContext,
  ConnectionState,
  ResourceEntry,
  HelmRelease,
} from '../../src/lib/tauri-commands';

interface AiInsightsHistoryEntry {
  createdAt: string;
  scope: AiInsightsScope;
  response: AiInsightsResponse;
}

export interface MockTauriScenario {
  contexts?: ClusterContext[];
  connectionState?: ConnectionState;
  namespaces?: string[];
  preferences?: Record<string, string | null>;
  pools?: AksNodePool[];
  upgradeProfiles?: Record<string, PoolUpgradeProfile>;
  resources?: Record<string, ResourceEntry[]>;
  helmReleases?: HelmRelease[];
  aiInsightsConnectionResult?: AiInsightsConnectionTestResult;
  aiInsightsGenerateResponse?: AiInsightsResponse;
  aiInsightsHistory?: AiInsightsHistoryEntry[];
  commandErrors?: Record<string, string | { message: string; times?: number }>;
  commandDelays?: Record<string, number>;
}

const defaultContexts: ClusterContext[] = [
  {
    name: 'aks-dev',
    cluster_server: 'https://demo.hcp.eastus.azmk8s.io',
    namespace: 'default',
    is_active: true,
    auth_type: 'exec',
  },
];

const defaultPools: AksNodePool[] = [
  {
    name: 'systempool',
    vmSize: 'Standard_DS2_v2',
    count: 1,
    osType: 'Linux',
    osDiskSizeGb: 128,
    mode: 'System',
    orchestratorVersion: '1.29.3',
    enableAutoScaling: false,
    minCount: null,
    maxCount: null,
    availabilityZones: ['1', '2'],
    nodeLabels: { role: 'system' },
    nodeTaints: ['CriticalAddonsOnly=true:NoSchedule'],
    provisioningState: 'Succeeded',
    powerState: { code: 'Running' },
    maxPods: 110,
    nodeImageVersion: 'AKSUbuntu-2204gen2containerd-2024.09.01',
    vnetSubnetId: '/subscriptions/mock/resourceGroups/mock/providers/Microsoft.Network/virtualNetworks/mock/subnets/system',
  },
  {
    name: 'gpunp',
    vmSize: 'Standard_NC6s_v3',
    count: 2,
    osType: 'Linux',
    osDiskSizeGb: 256,
    mode: 'User',
    orchestratorVersion: '1.29.3',
    enableAutoScaling: true,
    minCount: 1,
    maxCount: 4,
    availabilityZones: ['1'],
    nodeLabels: { workload: 'gpu' },
    nodeTaints: ['sku=gpu:NoSchedule'],
    provisioningState: 'Succeeded',
    powerState: { code: 'Running' },
    maxPods: 60,
    nodeImageVersion: 'AKSUbuntu-2204gen2containerd-2024.09.01',
    vnetSubnetId: '/subscriptions/mock/resourceGroups/mock/providers/Microsoft.Network/virtualNetworks/mock/subnets/gpu',
  },
];

const defaultUpgradeProfiles: Record<string, PoolUpgradeProfile> = {
  systempool: {
    currentVersion: '1.29.3',
    latestNodeImageVersion: 'AKSUbuntu-2204gen2containerd-2024.10.15',
    upgrades: [{ kubernetesVersion: '1.29.5', isPreview: false }],
  },
  gpunp: {
    currentVersion: '1.29.3',
    latestNodeImageVersion: 'AKSUbuntu-2204gen2containerd-2024.10.15',
    upgrades: [
      { kubernetesVersion: '1.29.5', isPreview: false },
      { kubernetesVersion: '1.30.0', isPreview: true },
    ],
  },
};

const defaultResources: Record<string, ResourceEntry[]> = {
  'v1/Pod': [
    {
      gvk: 'v1/Pod',
      namespace: 'default',
      name: 'nginx-abc123',
      resource_version: '1001',
      content: JSON.stringify({
        apiVersion: 'v1', kind: 'Pod',
        metadata: { name: 'nginx-abc123', namespace: 'default', creationTimestamp: '2026-01-01T00:00:00Z', labels: { app: 'nginx' } },
        spec: { nodeName: 'node-1', restartPolicy: 'Always', serviceAccountName: 'default', containers: [{ name: 'nginx', image: 'nginx:1.25', ports: [{ containerPort: 80, protocol: 'TCP' }] }] },
        status: { phase: 'Running', podIP: '10.0.0.1', hostIP: '192.168.1.1', qosClass: 'BestEffort' },
      }),
      updated_at: '2026-01-01T00:00:00Z',
    },
    {
      gvk: 'v1/Pod',
      namespace: 'default',
      name: 'redis-def456',
      resource_version: '1002',
      content: JSON.stringify({
        apiVersion: 'v1', kind: 'Pod',
        metadata: { name: 'redis-def456', namespace: 'default', creationTimestamp: '2026-01-02T00:00:00Z', labels: { app: 'redis' } },
        spec: { nodeName: 'node-1', restartPolicy: 'Always', serviceAccountName: 'default', containers: [{ name: 'redis', image: 'redis:7', ports: [{ containerPort: 6379, protocol: 'TCP' }] }] },
        status: { phase: 'Running', podIP: '10.0.0.2', hostIP: '192.168.1.1', qosClass: 'BestEffort' },
      }),
      updated_at: '2026-01-02T00:00:00Z',
    },
  ],
  'apps/v1/Deployment': [
    {
      gvk: 'apps/v1/Deployment',
      namespace: 'default',
      name: 'nginx-deploy',
      resource_version: '2001',
      content: JSON.stringify({
        apiVersion: 'apps/v1', kind: 'Deployment',
        metadata: { name: 'nginx-deploy', namespace: 'default', creationTimestamp: '2026-01-01T00:00:00Z' },
        spec: { replicas: 2, selector: { matchLabels: { app: 'nginx' } }, template: { spec: { containers: [{ name: 'nginx', image: 'nginx:1.25' }] } } },
        status: { readyReplicas: 2, availableReplicas: 2, updatedReplicas: 2 },
      }),
      updated_at: '2026-01-01T00:00:00Z',
    },
  ],
  'apps/v1/ReplicaSet': [
    {
      gvk: 'apps/v1/ReplicaSet',
      namespace: 'default',
      name: 'nginx-deploy-7f8f9c5c6f',
      resource_version: '2101',
      content: JSON.stringify({
        apiVersion: 'apps/v1', kind: 'ReplicaSet',
        metadata: { name: 'nginx-deploy-7f8f9c5c6f', namespace: 'default', creationTimestamp: '2026-01-03T00:00:00Z', labels: { app: 'nginx' } },
        spec: { replicas: 2, selector: { matchLabels: { app: 'nginx' } } },
        status: { replicas: 2, readyReplicas: 2, availableReplicas: 2 },
      }),
      updated_at: '2026-01-03T00:00:00Z',
    },
  ],
  'rbac.authorization.k8s.io/v1/ClusterRole': [
    {
      gvk: 'rbac.authorization.k8s.io/v1/ClusterRole',
      namespace: '',
      name: 'view',
      resource_version: '4101',
      content: JSON.stringify({
        apiVersion: 'rbac.authorization.k8s.io/v1', kind: 'ClusterRole',
        metadata: { name: 'view', creationTimestamp: '2026-01-01T00:00:00Z' },
        rules: [{ apiGroups: [''], resources: ['pods'], verbs: ['get', 'list', 'watch'] }],
      }),
      updated_at: '2026-01-01T00:00:00Z',
    },
  ],
  'rbac.authorization.k8s.io/v1/ClusterRoleBinding': [
    {
      gvk: 'rbac.authorization.k8s.io/v1/ClusterRoleBinding',
      namespace: '',
      name: 'viewers-binding',
      resource_version: '4201',
      content: JSON.stringify({
        apiVersion: 'rbac.authorization.k8s.io/v1', kind: 'ClusterRoleBinding',
        metadata: { name: 'viewers-binding', creationTimestamp: '2026-01-01T00:00:00Z' },
        roleRef: { apiGroup: 'rbac.authorization.k8s.io', kind: 'ClusterRole', name: 'view' },
        subjects: [{ kind: 'ServiceAccount', name: 'default', namespace: 'default' }],
      }),
      updated_at: '2026-01-01T00:00:00Z',
    },
  ],
  'v1/Node': [
    {
      gvk: 'v1/Node',
      namespace: '',
      name: 'node-1',
      resource_version: '3001',
      content: JSON.stringify({
        apiVersion: 'v1', kind: 'Node',
        metadata: { name: 'node-1', labels: { 'node-role.kubernetes.io/control-plane': '' }, creationTimestamp: '2026-01-01T00:00:00Z' },
        spec: { unschedulable: false },
        status: {
          conditions: [{ type: 'Ready', status: 'True', lastTransitionTime: '2026-01-01T00:00:00Z' }],
          capacity: { cpu: '4', memory: '16Gi', pods: '110' },
          allocatable: { cpu: '3800m', memory: '15Gi', pods: '110' },
          nodeInfo: { kubeletVersion: 'v1.29.3', osImage: 'Ubuntu 22.04', containerRuntimeVersion: 'containerd://1.7.0', architecture: 'amd64' },
        },
      }),
      updated_at: '2026-01-01T00:00:00Z',
    },
  ],
};

const defaultHelmReleases: HelmRelease[] = [
  {
    name: 'ingress-nginx',
    namespace: 'default',
    chart: 'ingress-nginx-4.8.3',
    app_version: '1.9.4',
    revision: 1,
    status: 'deployed',
    updated: '2026-01-01T12:00:00Z',
  },
];

const defaultInsightsResponse: AiInsightsResponse = {
  summary: 'Cluster is healthy with one medium risk.',
  risks: [{ title: 'Node pressure', detail: 'One node reports memory pressure.', impact: 'medium' }],
  observations: [{ area: 'Workloads', detail: 'All tracked deployments report ready replicas.' }],
  recommendations: [{ action: 'Investigate node pressure', rationale: 'Prevent potential evictions.', confidence: 0.81 }],
  references: [{ kind: 'Node', name: 'node-1', namespace: null }],
};

const defaultInsightsConnectionResult: AiInsightsConnectionTestResult = {
  normalizedEndpoint: 'https://example.openai.azure.com/',
  chatCompletionsUrl: 'https://example.openai.azure.com/openai/deployments/insights/chat/completions?api-version=2024-10-21',
  model: 'gpt-4.1',
};

const defaultInsightsHistory: AiInsightsHistoryEntry[] = [
  {
    createdAt: '2026-01-03T12:00:00Z',
    scope: { kind: 'cluster' },
    response: defaultInsightsResponse,
  },
];

export async function installMockTauri(page: Page, scenario: MockTauriScenario = {}): Promise<void> {
  await page.addInitScript((input: MockTauriScenario) => {
    const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value));

    const state = {
      contexts: clone(input.contexts ?? []),
      connectionState: clone(input.connectionState ?? { state: 'Ready' }),
      namespaces: clone(input.namespaces ?? []),
      preferences: clone(input.preferences ?? {}),
      pools: clone(input.pools ?? []),
      upgradeProfiles: clone(input.upgradeProfiles ?? {}),
      resources: clone(input.resources ?? {}),
      helmReleases: clone(input.helmReleases ?? []),
      aiInsightsConnectionResult: clone(input.aiInsightsConnectionResult ?? defaultInsightsConnectionResult),
      aiInsightsGenerateResponse: clone(input.aiInsightsGenerateResponse ?? defaultInsightsResponse),
      aiInsightsHistory: clone(input.aiInsightsHistory ?? defaultInsightsHistory),
      commandErrors: clone(input.commandErrors ?? {}),
      commandDelays: clone(input.commandDelays ?? {}),
      calls: [] as Array<{ cmd: string; args: Record<string, unknown> }>,
    };

    const listeners = new Map<number, unknown>();
    let nextCallbackId = 1;
    let nextEventListenerId = 1;

    const defaultPoolShape = {
      vmSize: 'Standard_DS2_v2',
      count: 1,
      osType: 'Linux',
      osDiskSizeGb: 128,
      mode: 'User',
      orchestratorVersion: '1.29.3',
      enableAutoScaling: false,
      minCount: null,
      maxCount: null,
      availabilityZones: null,
      nodeLabels: null,
      nodeTaints: null,
      provisioningState: 'Succeeded',
      powerState: { code: 'Running' },
      maxPods: 110,
      nodeImageVersion: 'AKSUbuntu-2204gen2containerd-2024.09.01',
      vnetSubnetId: '/subscriptions/mock/resourceGroups/mock/providers/Microsoft.Network/virtualNetworks/mock/subnets/default',
    };

    const findPoolIndex = (poolName: unknown): number => {
      const index = state.pools.findIndex((pool) => pool.name === poolName);
      if (index === -1) {
        throw new Error(`Unknown node pool: ${String(poolName)}`);
      }
      return index;
    };

    Object.defineProperty(window, '__TEST_TAURI__', {
      configurable: true,
      value: {
        get calls() {
          return clone(state.calls);
        },
        get pools() {
          return clone(state.pools);
        },
        get resources() {
          return clone(state.resources);
        },
        get aiInsightsHistory() {
          return clone(state.aiInsightsHistory);
        },
      },
    });

    Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
      configurable: true,
      value: {
        unregisterListener: (_event: string, eventId: number) => {
          listeners.delete(eventId);
        },
      },
    });

    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {
        transformCallback: (callback: unknown) => {
          const id = nextCallbackId++;
          listeners.set(id, callback);
          return id;
        },
        invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
          state.calls.push({ cmd, args: clone(args) });

          const commandDelayMs = Number(state.commandDelays[cmd] ?? 0);
          if (commandDelayMs > 0) {
            await new Promise((resolve) => setTimeout(resolve, commandDelayMs));
          }

          const configuredError = state.commandErrors[cmd];
          if (configuredError) {
            const normalized = typeof configuredError === 'string'
              ? { message: configuredError, times: Number.POSITIVE_INFINITY }
              : {
                  message: configuredError.message,
                  times: configuredError.times ?? Number.POSITIVE_INFINITY,
                };

            if (normalized.times > 0) {
              if (typeof configuredError !== 'string' && Number.isFinite(normalized.times)) {
                const nextTimes = normalized.times - 1;
                if (nextTimes > 0) {
                  state.commandErrors[cmd] = { ...configuredError, times: nextTimes };
                } else {
                  delete state.commandErrors[cmd];
                }
              }
              throw new Error(normalized.message);
            }
          }

          switch (cmd) {
            case 'list_contexts':
              return clone(state.contexts);
            case 'connect_to_context':
            case 'disconnect':
            case 'set_namespace':
              return null;
            case 'list_namespaces':
              return clone(state.namespaces);
            case 'get_connection_state':
              return clone(state.connectionState);
            case 'get_preference':
              return state.preferences[String(args.key)] ?? null;
            case 'check_metrics_available':
              return false;
            case 'get_node_metrics':
              return [];
            case 'get_pod_metrics':
              return [];
            case 'get_resources': {
              const gvk = String(args.gvk);
              const ns = args.namespace as string | null;
              const entries = state.resources[gvk] ?? [];
              return clone(ns ? entries.filter(e => e.namespace === ns) : entries);
            }
            case 'get_resource': {
              const gvk = String(args.gvk);
              const ns = String(args.namespace ?? '');
              const name = String(args.name);
              const entries = state.resources[gvk] ?? [];
              return clone(entries.find(e => e.name === name && e.namespace === ns) ?? null);
            }
            case 'get_pods': {
              const ns = args.namespace as string | null;
              const pods = state.resources['v1/Pod'] ?? [];
              return clone(ns ? pods.filter(p => p.namespace === ns) : pods);
            }
            case 'get_events':
              return [];
            case 'get_resource_counts': {
              const counts: [string, number][] = [];
              for (const [gvk, entries] of Object.entries(state.resources)) {
                counts.push([gvk, entries.length]);
              }
              return counts;
            }
            case 'search_resources': {
              const query = String(args.query ?? '').toLowerCase();
              const all: Array<Record<string, unknown>> = [];
              for (const entries of Object.values(state.resources)) {
                for (const e of entries) {
                  if (e.name.toLowerCase().includes(query) || e.gvk.toLowerCase().includes(query)) {
                    all.push(clone(e));
                  }
                }
              }
              return all.slice(0, 20);
            }
            case 'set_preference':
              state.preferences[String(args.key)] = String(args.value);
              return null;
            case 'test_ai_insights_connection':
              return clone(state.aiInsightsConnectionResult);
            case 'generate_ai_insights': {
              const response = clone(state.aiInsightsGenerateResponse);
              const historyEntry = {
                createdAt: new Date().toISOString(),
                scope: { kind: 'cluster' as const },
                response,
              };
              state.aiInsightsHistory.unshift(historyEntry);
              state.aiInsightsHistory = state.aiInsightsHistory.slice(0, 3);
              return response;
            }
            case 'list_ai_insights_history':
              return clone(state.aiInsightsHistory);
            case 'clear_ai_insights_history':
              state.aiInsightsHistory = [];
              return null;
            case 'active_context': {
              const active = state.contexts.find(c => c.is_active);
              return active ? active.name : null;
            }
            case 'get_cluster_info':
              return null;
            case 'list_containers':
              return ['main'];
            case 'get_pod_logs':
              return '';
            case 'start_log_stream':
              return null;
            case 'list_crds':
              return [];
            case 'list_helm_releases': {
              const ns = args.namespace as string | null;
              const releases = state.helmReleases;
              return clone(ns ? releases.filter(r => r.namespace === ns) : releases);
            }
            case 'get_helm_release_history': {
              const releaseName = String(args.name);
              const ns = String(args.namespace);
              return clone(state.helmReleases.filter(r => r.name === releaseName && r.namespace === ns));
            }
            case 'get_helm_release_values':
              return '';
            case 'helm_uninstall': {
              const releaseName = String(args.name);
              const ns = String(args.namespace);
              const releaseIndex = state.helmReleases.findIndex((release) =>
                release.name === releaseName && release.namespace === ns
              );
              if (releaseIndex >= 0) {
                state.helmReleases.splice(releaseIndex, 1);
              }
              return `Uninstalled Helm release ${releaseName} from namespace ${ns}`;
            }
            case 'apply_resource':
              return { success: true, message: 'Applied successfully (mock)' };
            case 'delete_resource':
              return 'Deleted (mock)';
            case 'rollout_status':
              return { desired: 1, ready: 1, updated: 1, available: 1, is_complete: true, message: 'deployment "mock" successfully rolled out' };
            case 'rollout_restart':
              return 'Rollout restart initiated (mock)';
            case 'scale_resource':
              return 'Scaled (mock)';
            case 'start_port_forward':
              return 8080;
            case 'resolve_aks_identity':
              return null;
            case 'get_azure_cloud':
              return 'Commercial';
            case 'set_azure_cloud':
              return null;
            case 'plugin:event|listen':
              return nextEventListenerId++;
            case 'plugin:event|unlisten':
              return null;
            case 'list_aks_node_pools':
              return clone(state.pools);
            case 'get_pool_upgrade_profile': {
              const profile = state.upgradeProfiles[String(args.pool)];
              if (!profile) {
                throw new Error(`No upgrade profile registered for ${String(args.pool)}`);
              }
              return clone(profile);
            }
            case 'scale_aks_node_pool': {
              const index = findPoolIndex(args.poolName);
              const count = Number(args.count);
              state.pools[index] = { ...state.pools[index], count };
              return clone(state.pools[index]);
            }
            case 'update_aks_autoscaler': {
              const index = findPoolIndex(args.poolName);
              const enabled = Boolean(args.enabled);
              state.pools[index] = {
                ...state.pools[index],
                enableAutoScaling: enabled,
                minCount: enabled ? Number(args.min) : null,
                maxCount: enabled ? Number(args.max) : null,
              };
              return clone(state.pools[index]);
            }
            case 'create_aks_node_pool': {
              const config = (args.config ?? {}) as Record<string, unknown>;
              const createdPool = {
                name: String(config.name),
                ...defaultPoolShape,
                vmSize: String(config.vmSize ?? defaultPoolShape.vmSize),
                count: Number(config.count ?? defaultPoolShape.count),
                osType: String(config.osType ?? defaultPoolShape.osType),
                mode: String(config.mode ?? defaultPoolShape.mode),
                orchestratorVersion: String(config.orchestratorVersion ?? defaultPoolShape.orchestratorVersion),
                enableAutoScaling: Boolean(config.enableAutoScaling ?? false),
                minCount: config.enableAutoScaling ? Number(config.minCount ?? 1) : null,
                maxCount: config.enableAutoScaling ? Number(config.maxCount ?? 5) : null,
                availabilityZones: (config.availabilityZones as string[] | undefined) ?? null,
                maxPods: Number(config.maxPods ?? defaultPoolShape.maxPods),
                nodeLabels: (config.nodeLabels as Record<string, string> | undefined) ?? null,
                nodeTaints: (config.nodeTaints as string[] | undefined) ?? null,
              };
              state.pools.push(createdPool);
              state.upgradeProfiles[createdPool.name] = {
                currentVersion: createdPool.orchestratorVersion,
                latestNodeImageVersion: 'AKSUbuntu-2204gen2containerd-2024.10.15',
                upgrades: [{ kubernetesVersion: '1.29.5', isPreview: false }],
              };
              return clone(createdPool);
            }
            case 'delete_aks_node_pool': {
              const index = findPoolIndex(args.poolName);
              state.pools.splice(index, 1);
              return null;
            }
            case 'upgrade_pool_version': {
              const index = findPoolIndex(args.pool);
              const version = String(args.version);
              state.pools[index] = { ...state.pools[index], orchestratorVersion: version };
              if (state.upgradeProfiles[state.pools[index].name]) {
                state.upgradeProfiles[state.pools[index].name] = {
                  ...state.upgradeProfiles[state.pools[index].name],
                  currentVersion: version,
                };
              }
              return null;
            }
            case 'upgrade_pool_node_image': {
              const index = findPoolIndex(args.pool);
              const nextImage = state.upgradeProfiles[state.pools[index].name]?.latestNodeImageVersion ?? 'AKSUbuntu-2204gen2containerd-2024.10.15';
              state.pools[index] = { ...state.pools[index], nodeImageVersion: nextImage };
              return null;
            }
            default:
              throw new Error(`Unhandled mock Tauri command: ${cmd}`);
          }
        },
      },
    });
  }, {
    contexts: scenario.contexts ?? defaultContexts,
    connectionState: scenario.connectionState ?? { state: 'Ready' },
    namespaces: scenario.namespaces ?? ['default', 'kube-system'],
    preferences: scenario.preferences ?? {
      auto_refresh_interval: '300',
      default_namespace: 'default',
      production_patterns: null,
    },
    pools: scenario.pools ?? defaultPools,
    upgradeProfiles: scenario.upgradeProfiles ?? defaultUpgradeProfiles,
    resources: scenario.resources ?? defaultResources,
    helmReleases: scenario.helmReleases ?? defaultHelmReleases,
    aiInsightsConnectionResult: scenario.aiInsightsConnectionResult ?? defaultInsightsConnectionResult,
    aiInsightsGenerateResponse: scenario.aiInsightsGenerateResponse ?? defaultInsightsResponse,
    aiInsightsHistory: scenario.aiInsightsHistory ?? defaultInsightsHistory,
    commandErrors: scenario.commandErrors ?? {},
    commandDelays: scenario.commandDelays ?? {},
  });
}
