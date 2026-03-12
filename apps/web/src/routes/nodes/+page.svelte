<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getResources, getNodeMetrics, checkMetricsAvailable } from '$lib/api';
  import { isConnected } from '$lib/stores';
  import ResourceTable from '$lib/components/ResourceTable.svelte';
  import NodePoolHeader from '$lib/components/NodePoolHeader.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import type { ResourceEntry, NodeMetricsData } from '$lib/tauri-commands';

  const GVK = 'v1/Node';
  const PAGE_TITLE = 'Nodes';
  const OTHER_POOL = 'Other';

  interface NodePool {
    name: string;
    vmSize: string;
    osType: string;
    mode: string;
    nodes: ResourceEntry[];
    readyCount: number;
  }

  function formatAge(timestamp: string): string {
    const created = new Date(timestamp);
    const now = new Date();
    const diffSec = Math.floor((now.getTime() - created.getTime()) / 1000);
    if (diffSec < 60) return `${diffSec}s`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h`;
    return `${Math.floor(diffHours / 24)}d`;
  }

  function getNodeStatus(obj: any): string {
    const conditions = obj?.status?.conditions ?? [];
    const ready = conditions.find((c: any) => c.type === 'Ready');
    if (!ready) return 'Unknown';
    return ready.status === 'True' ? 'Ready' : 'NotReady';
  }

  function getNodeRoles(obj: any): string {
    const labels = obj?.metadata?.labels ?? {};
    const roles: string[] = [];
    for (const key of Object.keys(labels)) {
      if (key.startsWith('node-role.kubernetes.io/')) {
        roles.push(key.replace('node-role.kubernetes.io/', ''));
      }
    }
    return roles.length > 0 ? roles.join(', ') : '<none>';
  }

  function getPoolName(obj: any): string | null {
    const labels = obj?.metadata?.labels ?? {};
    return labels['agentpool'] ?? labels['kubernetes.azure.com/agentpool'] ?? null;
  }

  function getPoolVmSize(obj: any): string {
    return obj?.metadata?.labels?.['node.kubernetes.io/instance-type'] ?? '';
  }

  function getPoolOsType(obj: any): string {
    return obj?.metadata?.labels?.['kubernetes.azure.com/os-type']
      ?? obj?.metadata?.labels?.['beta.kubernetes.io/os']
      ?? obj?.status?.nodeInfo?.operatingSystem
      ?? '';
  }

  function getPoolMode(obj: any): string {
    const mode = obj?.metadata?.labels?.['kubernetes.azure.com/mode'] ?? '';
    return mode ? mode.charAt(0).toUpperCase() + mode.slice(1) : '';
  }

  function groupNodesByPool(entries: ResourceEntry[]): { pools: NodePool[]; hasAksLabels: boolean } {
    const poolMap = new Map<string, { nodes: ResourceEntry[]; objs: any[] }>();
    let aksCount = 0;

    for (const entry of entries) {
      let obj: any;
      try { obj = JSON.parse(entry.content); } catch { obj = {}; }
      const pool = getPoolName(obj);
      if (pool) aksCount++;
      const key = pool ?? OTHER_POOL;
      if (!poolMap.has(key)) poolMap.set(key, { nodes: [], objs: [] });
      const group = poolMap.get(key)!;
      group.nodes.push(entry);
      group.objs.push(obj);
    }

    if (aksCount === 0) return { pools: [], hasAksLabels: false };

    const pools: NodePool[] = [];
    for (const [name, { nodes, objs }] of poolMap) {
      const readyCount = objs.filter(o => getNodeStatus(o) === 'Ready').length;
      // Derive pool-level metadata from the first node in the group
      const first = objs[0];
      pools.push({
        name,
        vmSize: getPoolVmSize(first),
        osType: getPoolOsType(first),
        mode: getPoolMode(first),
        nodes,
        readyCount,
      });
    }

    // Sort: named pools alphabetically, "Other" last
    pools.sort((a, b) => {
      if (a.name === OTHER_POOL) return 1;
      if (b.name === OTHER_POOL) return -1;
      return a.name.localeCompare(b.name);
    });

    return { pools, hasAksLabels: true };
  }

  function usageColor(pct: number): string {
    if (pct < 70) return '#66bb6a';
    if (pct < 90) return '#ffa726';
    return '#ef5350';
  }

  const columns = [
    { key: 'name', label: 'Name', extract: (c: any) => c?.metadata?.name ?? '', width: '20%' },
    { key: 'status', label: 'Status', extract: (c: any) => getNodeStatus(c) },
    { key: 'roles', label: 'Roles', extract: (c: any) => getNodeRoles(c) },
    { key: 'version', label: 'Version', extract: (c: any) => c?.status?.nodeInfo?.kubeletVersion ?? '' },
    { key: 'cpu', label: 'CPU', extract: (c: any) => c?.status?.capacity?.cpu ?? '' },
    { key: 'memory', label: 'Memory', extract: (c: any) => c?.status?.capacity?.memory ?? '' },
    { key: 'cpu_used', label: 'CPU Used', extract: (c: any) => {
      const m = metricsMap[c?.metadata?.name ?? ''];
      if (!m) return '—';
      return `${m.cpu_millicores}m (${m.cpu_percent}%)`;
    }, colorFn: (c: any) => {
      const m = metricsMap[c?.metadata?.name ?? ''];
      return m ? usageColor(m.cpu_percent) : null;
    }},
    { key: 'mem_used', label: 'Mem Used', extract: (c: any) => {
      const m = metricsMap[c?.metadata?.name ?? ''];
      if (!m) return '—';
      const mib = Math.round(m.memory_bytes / (1024 * 1024));
      return `${mib}Mi (${m.memory_percent}%)`;
    }, colorFn: (c: any) => {
      const m = metricsMap[c?.metadata?.name ?? ''];
      return m ? usageColor(m.memory_percent) : null;
    }},
    { key: 'age', label: 'Age', extract: (c: any) => {
      const ts = c?.metadata?.creationTimestamp;
      return ts ? formatAge(ts) : 'Unknown';
    }},
  ];

  let resources: ResourceEntry[] = $state([]);
  let nodeMetrics: NodeMetricsData[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let collapsedPools: Record<string, boolean> = $state({});
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;

  let metricsMap: Record<string, NodeMetricsData> = $derived(
    Object.fromEntries(nodeMetrics.map(m => [m.name, m]))
  );

  let grouped = $derived(groupNodesByPool(resources));

  function togglePool(name: string) {
    collapsedPools = { ...collapsedPools, [name]: !collapsedPools[name] };
  }

  function formatTimestamp(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    return `Updated ${Math.floor(seconds / 60)}m ago`;
  }

  async function loadResources(userInitiated = false) {
    if (!$isConnected) { loading = false; resources = []; nodeMetrics = []; return; }
    const isInitial = resources.length === 0 && !lastUpdated;
    if (isInitial) loading = true; else if (userInitiated) refreshing = true;
    error = null;
    try {
      const [nodes, available] = await Promise.all([
        getResources(GVK, null as unknown as string),
        checkMetricsAvailable(),
      ]);
      resources = nodes;
      if (available) {
        nodeMetrics = await getNodeMetrics();
      }
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : `Failed to load ${PAGE_TITLE.toLowerCase()}`;
      resources = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  $effect(() => {
    void $isConnected;
    loadResources();
  });

  onMount(() => {
    refreshTimer = setInterval(loadResources, 10_000);
    timestampTimer = setInterval(() => { lastUpdatedText = formatTimestamp(); }, 1000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="resource-page">
  <header>
    <h1>{PAGE_TITLE}</h1>
    <div class="controls">
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" onclick={() => loadResources(true)} disabled={refreshing} class:spinning={refreshing} aria-label="Refresh {PAGE_TITLE.toLowerCase()}">
        <span class="refresh-icon">↻</span> Refresh
      </button>
    </div>
  </header>

  {#if !$isConnected && !loading}
    <div class="not-connected">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if loading}
    <LoadingSkeleton rows={8} columns={columns.length} />
  {:else if error}
    <div role="alert" aria-live="polite" class="error-container">
      <p class="error">Failed to load {PAGE_TITLE.toLowerCase()}. Check cluster connection and try again.</p>
      {#if error !== `Failed to load ${PAGE_TITLE.toLowerCase()}`}
        <p class="error-detail">{error}</p>
      {/if}
      <button type="button" onclick={loadResources}>Retry</button>
    </div>
  {:else}
    <p class="count">{resources.length} {PAGE_TITLE.toLowerCase()}</p>
    {#if grouped.hasAksLabels}
      <div class="pool-list">
        {#each grouped.pools as pool (pool.name)}
          <section class="pool-section">
            <NodePoolHeader
              poolName={pool.name}
              nodeCount={pool.nodes.length}
              readyCount={pool.readyCount}
              vmSize={pool.vmSize}
              osType={pool.osType}
              mode={pool.mode}
              collapsed={!!collapsedPools[pool.name]}
              onToggle={() => togglePool(pool.name)}
            />
            {#if !collapsedPools[pool.name]}
              <div class="pool-nodes">
                <ResourceTable resources={pool.nodes} {columns} emptyMessage="No nodes in this pool." hrefFn={(entry) => `/nodes/${entry.name}`} />
              </div>
            {/if}
          </section>
        {/each}
      </div>
    {:else}
      <ResourceTable {resources} {columns} emptyMessage="No nodes found." hrefFn={(entry) => `/nodes/${entry.name}`} />
    {/if}
  {/if}
</div>

<style>
  .resource-page { padding: 1rem; color: #e0e0e0; background: #0f0f23; min-height: 100vh; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  h1 { font-size: 1.5rem; margin: 0; }
  .controls { display: flex; gap: 0.75rem; align-items: center; }
  .last-updated { color: #757575; font-size: 0.75rem; }
  button { background: #1a73e8; color: white; border: none; padding: 0.375rem 0.75rem; border-radius: 4px; cursor: pointer; display: inline-flex; align-items: center; gap: 0.25rem; }
  button:hover { background: #1565c0; }
  button:disabled { opacity: 0.6; cursor: not-allowed; }
  .refresh-icon { display: inline-block; }
  .spinning .refresh-icon { animation: spin 1s linear infinite; }
  @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
  .count { color: #9e9e9e; margin-bottom: 0.5rem; font-size: 0.875rem; }
  .error-container { padding: 1.5rem; text-align: center; }
  .error { color: #ef5350; }
  .error-detail { color: #9e9e9e; font-size: 0.875rem; margin-top: 0.25rem; }
  .not-connected { text-align: center; padding: 3rem 1rem; color: #757575; }
  .not-connected p { margin: 0.25rem 0; font-size: 1.125rem; }
  .not-connected .hint { font-size: 0.875rem; color: #616161; }
  .pool-list { display: flex; flex-direction: column; gap: 0.75rem; }
  .pool-section { display: flex; flex-direction: column; }
  .pool-nodes { margin-top: 0.25rem; margin-left: 0.5rem; border-left: 2px solid #2a2a4a; padding-left: 0.5rem; }
</style>
