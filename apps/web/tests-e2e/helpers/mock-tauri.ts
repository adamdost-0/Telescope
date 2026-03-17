import type { Page } from '@playwright/test';
import type { AksNodePool, PoolUpgradeProfile, ClusterContext } from '../../src/lib/tauri-commands';

export interface MockTauriScenario {
  contexts?: ClusterContext[];
  namespaces?: string[];
  preferences?: Record<string, string | null>;
  pools?: AksNodePool[];
  upgradeProfiles?: Record<string, PoolUpgradeProfile>;
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

export async function installMockTauri(page: Page, scenario: MockTauriScenario = {}): Promise<void> {
  await page.addInitScript((input: MockTauriScenario) => {
    const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value));

    const state = {
      contexts: clone(input.contexts ?? []),
      namespaces: clone(input.namespaces ?? []),
      preferences: clone(input.preferences ?? {}),
      pools: clone(input.pools ?? []),
      upgradeProfiles: clone(input.upgradeProfiles ?? {}),
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
              return { state: 'Ready' };
            case 'get_preference':
              return state.preferences[String(args.key)] ?? null;
            case 'check_metrics_available':
              return false;
            case 'get_node_metrics':
            case 'get_resources':
              return [];
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
    namespaces: scenario.namespaces ?? ['default', 'kube-system'],
    preferences: scenario.preferences ?? {
      auto_refresh_interval: '300',
      default_namespace: 'default',
      production_patterns: null,
    },
    pools: scenario.pools ?? defaultPools,
    upgradeProfiles: scenario.upgradeProfiles ?? defaultUpgradeProfiles,
  });
}
