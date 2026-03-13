<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { listAksNodePools } from '$lib/api';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import { getAutoRefreshIntervalMs } from '$lib/preferences';
  import { isAks, isConnected } from '$lib/stores';
  import type { AksNodePool } from '$lib/tauri-commands';

  const PAGE_TITLE = 'Node Pools';

  let pools: AksNodePool[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error: string | null = $state(null);
  let filterQuery = $state('');
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;
  let expandedPools: Record<string, boolean> = $state({});
  let destroyed = false;

  function formatTimestamp(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    return `Updated ${Math.floor(seconds / 60)}m ago`;
  }

  function togglePool(name: string) {
    expandedPools = { ...expandedPools, [name]: !expandedPools[name] };
  }

  function poolAnchorId(name: string): string {
    return `pool-${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  }

  function formatAutoscaler(pool: AksNodePool): string {
    if (!pool.enable_auto_scaling) return 'Off';
    const min = pool.min_count ?? '—';
    const max = pool.max_count ?? '—';
    return `${min}-${max}`;
  }

  function formatZones(pool: AksNodePool): string {
    return pool.availability_zones?.length ? pool.availability_zones.join(', ') : '—';
  }

  function formatStatus(pool: AksNodePool): string {
    return pool.provisioning_state ?? pool.power_state?.code ?? 'Unknown';
  }

  function statusVariant(status: string): string {
    const normalized = status.toLowerCase();
    if (normalized === 'succeeded' || normalized === 'running') return 'success';
    if (normalized === 'updating' || normalized === 'creating' || normalized === 'stopping') {
      return 'warning';
    }
    if (normalized === 'failed' || normalized === 'error') return 'danger';
    return 'neutral';
  }

  let filteredPools = $derived.by(() => {
    if (!filterQuery) return pools;
    const query = filterQuery.toLowerCase();
    return pools.filter((pool) =>
      [
        pool.name,
        pool.mode ?? '',
        pool.vm_size ?? '',
        pool.os_type ?? '',
        pool.orchestrator_version ?? '',
        pool.node_image_version ?? '',
        pool.provisioning_state ?? '',
        pool.power_state?.code ?? '',
      ].some((value) => value.toLowerCase().includes(query))
    );
  });

  async function loadPools(userInitiated = false) {
    if (!$isConnected || !$isAks) {
      pools = [];
      error = null;
      loading = false;
      refreshing = false;
      return;
    }

    const isInitial = pools.length === 0 && !lastUpdated;
    if (isInitial) {
      loading = true;
    } else if (userInitiated) {
      refreshing = true;
    }

    error = null;

    try {
      pools = await listAksNodePools();
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load node pools';
      pools = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  $effect(() => {
    void $isConnected;
    void $isAks;
    loadPools();
  });

  onMount(() => {
    void (async () => {
      const refreshIntervalMs = await getAutoRefreshIntervalMs(10_000);
      if (!destroyed) {
        refreshTimer = setInterval(() => {
          void loadPools();
        }, refreshIntervalMs);
      }
    })();
    timestampTimer = setInterval(() => {
      lastUpdatedText = formatTimestamp();
    }, 1000);
  });

  onDestroy(() => {
    destroyed = true;
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="resource-page">
  <header>
    <div>
      <h1>{PAGE_TITLE}</h1>
      <p class="subtitle">Authoritative AKS node pool data from the Azure ARM API.</p>
    </div>
    <div class="controls">
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" onclick={() => loadPools(true)} disabled={refreshing} class:spinning={refreshing}>
        <span class="refresh-icon">↻</span>
        Refresh
      </button>
    </div>
  </header>

  {#if !$isConnected && !loading}
    <div class="state-card">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if !$isAks && !loading}
    <div class="state-card">
      <p>Azure node pool data requires an AKS cluster.</p>
    </div>
  {:else if loading}
    <LoadingSkeleton rows={6} columns={10} />
  {:else if error}
    <div role="alert" class="state-card error-card">
      <p>Failed to load node pools.</p>
      <p class="hint">{error}</p>
      <button type="button" onclick={() => loadPools(true)}>Retry</button>
    </div>
  {:else}
    <FilterBar query={filterQuery} onfilter={(query) => (filterQuery = query)} />
    <p class="count">
      {filterQuery ? `${filteredPools.length} of ${pools.length}` : pools.length}
      node pools
    </p>

    {#if filteredPools.length === 0}
      <div class="state-card">
        <p>No node pools found.</p>
      </div>
    {:else}
      <div class="table-wrapper">
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Mode</th>
              <th>VM Size</th>
              <th>Count</th>
              <th>Autoscaler</th>
              <th>K8s Version</th>
              <th>Node Image</th>
              <th>OS</th>
              <th>Zones</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredPools as pool (pool.name)}
              <tr id={poolAnchorId(pool.name)}>
                <td>
                  <a
                    href={`#${poolAnchorId(pool.name)}`}
                    class="pool-link"
                    onclick={() => togglePool(pool.name)}
                  >
                    {expandedPools[pool.name] ? '▾' : '▸'} {pool.name}
                  </a>
                </td>
                <td>{pool.mode ?? '—'}</td>
                <td>{pool.vm_size ?? '—'}</td>
                <td>{pool.count ?? '—'}</td>
                <td>{formatAutoscaler(pool)}</td>
                <td>{pool.orchestrator_version ?? '—'}</td>
                <td>{pool.node_image_version ?? '—'}</td>
                <td>{pool.os_type ?? '—'}</td>
                <td>{formatZones(pool)}</td>
                <td>
                  <span class={`status-badge ${statusVariant(formatStatus(pool))}`}>
                    {formatStatus(pool)}
                  </span>
                </td>
              </tr>
              {#if expandedPools[pool.name]}
                <tr class="details-row">
                  <td colspan="10">
                    <div class="details-grid">
                      <div><span class="details-label">OS Disk</span><span>{pool.os_disk_size_gb ?? '—'} GiB</span></div>
                      <div><span class="details-label">Max Pods</span><span>{pool.max_pods ?? '—'}</span></div>
                      <div><span class="details-label">Power State</span><span>{pool.power_state?.code ?? '—'}</span></div>
                      <div><span class="details-label">Subnet</span><span class="break">{pool.vnet_subnet_id ?? '—'}</span></div>
                      <div><span class="details-label">Taints</span><span>{pool.node_taints?.join(', ') ?? '—'}</span></div>
                      <div><span class="details-label">Labels</span><span class="break">{pool.node_labels ? JSON.stringify(pool.node_labels) : '—'}</span></div>
                    </div>
                  </td>
                </tr>
              {/if}
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<style>
  .resource-page {
    padding: 1rem;
    color: #e0e0e0;
    background: #0f0f23;
    min-height: 100vh;
  }

  header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  h1 {
    font-size: 1.5rem;
    margin: 0;
  }

  .subtitle {
    margin: 0.25rem 0 0;
    color: #9e9e9e;
  }

  .controls {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }

  .last-updated {
    color: #757575;
    font-size: 0.75rem;
  }

  button {
    background: #1a73e8;
    color: white;
    border: none;
    padding: 0.375rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }

  button:hover {
    background: #1565c0;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .refresh-icon {
    display: inline-block;
  }

  .spinning .refresh-icon {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .count {
    color: #9e9e9e;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
  }

  .state-card {
    padding: 1.5rem;
    text-align: center;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
  }

  .hint {
    color: #9e9e9e;
    font-size: 0.875rem;
  }

  .error-card p:first-child {
    color: #ef5350;
  }

  .table-wrapper {
    overflow-x: auto;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    text-align: left;
    padding: 0.75rem;
    border-bottom: 1px solid #21262d;
    vertical-align: top;
  }

  th {
    color: #9e9e9e;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  tbody tr:hover {
    background: rgba(255, 255, 255, 0.02);
  }

  .pool-link {
    color: #8ab4f8;
    text-decoration: none;
    font-weight: 500;
  }

  .pool-link:hover {
    text-decoration: underline;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.8rem;
    font-weight: 600;
    border: 1px solid transparent;
  }

  .status-badge.success {
    color: #66bb6a;
    background: rgba(102, 187, 106, 0.12);
    border-color: rgba(102, 187, 106, 0.35);
  }

  .status-badge.warning {
    color: #ffa726;
    background: rgba(255, 167, 38, 0.12);
    border-color: rgba(255, 167, 38, 0.35);
  }

  .status-badge.danger {
    color: #ef5350;
    background: rgba(239, 83, 80, 0.12);
    border-color: rgba(239, 83, 80, 0.35);
  }

  .status-badge.neutral {
    color: #c5c5c5;
    background: rgba(197, 197, 197, 0.1);
    border-color: rgba(197, 197, 197, 0.2);
  }

  .details-row td {
    background: #121826;
  }

  .details-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.75rem 1rem;
  }

  .details-grid > div {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .details-label {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: #9e9e9e;
  }

  .break {
    overflow-wrap: anywhere;
  }
</style>
