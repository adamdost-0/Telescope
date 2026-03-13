<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getResources } from '$lib/api';
  import { getAutoRefreshIntervalMs } from '$lib/preferences';
  import { selectedNamespace, isConnected } from '$lib/stores';
  import ResourceTable from '$lib/components/ResourceTable.svelte';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  const GVK_ROLE_BINDING = 'rbac.authorization.k8s.io/v1/RoleBinding';
  const GVK_CLUSTER_ROLE_BINDING = 'rbac.authorization.k8s.io/v1/ClusterRoleBinding';

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

  let activeTab: 'rolebindings' | 'clusterrolebindings' = $state('rolebindings');

  const rbColumns = [
    { key: 'name', label: 'Name', extract: (c: any) => c?.metadata?.name ?? '', width: '30%' },
    { key: 'namespace', label: 'Namespace', extract: (c: any) => c?.metadata?.namespace ?? '' },
    { key: 'roleRef', label: 'Role Ref', extract: (c: any) => {
      const ref = c?.roleRef;
      return ref ? `${ref.kind}/${ref.name}` : 'N/A';
    }},
    { key: 'subjects', label: 'Subjects', extract: (c: any) => String((c?.subjects ?? []).length) },
    { key: 'age', label: 'Age', extract: (c: any) => {
      const ts = c?.metadata?.creationTimestamp;
      return ts ? formatAge(ts) : 'Unknown';
    }},
  ];

  const crbColumns = [
    { key: 'name', label: 'Name', extract: (c: any) => c?.metadata?.name ?? '', width: '35%' },
    { key: 'roleRef', label: 'Role Ref', extract: (c: any) => {
      const ref = c?.roleRef;
      return ref ? `${ref.kind}/${ref.name}` : 'N/A';
    }},
    { key: 'subjects', label: 'Subjects', extract: (c: any) => String((c?.subjects ?? []).length) },
    { key: 'age', label: 'Age', extract: (c: any) => {
      const ts = c?.metadata?.creationTimestamp;
      return ts ? formatAge(ts) : 'Unknown';
    }},
  ];

  let resources: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let filterQuery = $state('');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;

  let columns = $derived(activeTab === 'rolebindings' ? rbColumns : crbColumns);

  let filtered = $derived.by(() => {
    if (!filterQuery) return resources;
    const q = filterQuery.toLowerCase();
    return resources.filter(r => r.name.toLowerCase().includes(q));
  });

  function formatTimestamp(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    return `Updated ${Math.floor(seconds / 60)}m ago`;
  }

  async function loadResources() {
    if (!$isConnected) { loading = false; resources = []; return; }
    const isInitial = resources.length === 0 && !lastUpdated;
    if (isInitial) loading = true; else refreshing = true;
    error = null;
    try {
      if (activeTab === 'rolebindings') {
        resources = await getResources(GVK_ROLE_BINDING, $selectedNamespace);
      } else {
        resources = await getResources(GVK_CLUSTER_ROLE_BINDING);
      }
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load bindings';
      resources = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  function switchTab(tab: 'rolebindings' | 'clusterrolebindings') {
    activeTab = tab;
    resources = [];
    lastUpdated = null;
    filterQuery = '';
    loadResources();
  }

  $effect(() => {
    void $selectedNamespace;
    void $isConnected;
    loadResources();
  });

  let destroyed = false;

  onMount(() => {
    void (async () => {
      const refreshIntervalMs = await getAutoRefreshIntervalMs(3000);
      if (!destroyed) {
        refreshTimer = setInterval(loadResources, refreshIntervalMs);
      }
    })();
    timestampTimer = setInterval(() => { lastUpdatedText = formatTimestamp(); }, 1000);
  });

  onDestroy(() => {
    destroyed = true;
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="resource-page">
  <header>
    <h1>RoleBindings &amp; ClusterRoleBindings</h1>
    <div class="controls">
      {#if activeTab === 'rolebindings'}
        <span class="ns-label">Namespace: <strong>{$selectedNamespace}</strong></span>
      {/if}
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" onclick={loadResources} disabled={refreshing} class:spinning={refreshing} aria-label="Refresh">
        <span class="refresh-icon">↻</span> Refresh
      </button>
    </div>
  </header>

  <div class="tabs">
    <button type="button" class:active={activeTab === 'rolebindings'} onclick={() => switchTab('rolebindings')}>RoleBindings</button>
    <button type="button" class:active={activeTab === 'clusterrolebindings'} onclick={() => switchTab('clusterrolebindings')}>ClusterRoleBindings</button>
  </div>

  {#if !$isConnected && !loading}
    <div class="not-connected">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if loading}
    <LoadingSkeleton rows={8} columns={columns.length} />
  {:else if error}
    <div role="alert" aria-live="polite" class="error-container">
      <p class="error">Failed to load bindings. Check cluster connection and try again.</p>
      {#if error !== 'Failed to load bindings'}
        <p class="error-detail">{error}</p>
      {/if}
      <button type="button" onclick={loadResources}>Retry</button>
    </div>
  {:else}
    <FilterBar query={filterQuery} onfilter={(q) => filterQuery = q} />
    <p class="count">{filterQuery ? `${filtered.length} of ${resources.length}` : resources.length} {activeTab === 'rolebindings' ? 'role bindings' : 'cluster role bindings'}</p>
    <ResourceTable resources={filtered} {columns} emptyMessage={activeTab === 'rolebindings' ? 'No role bindings found in this namespace.' : 'No cluster role bindings found.'} hrefFn={(entry) => activeTab === 'rolebindings' ? `/resources/rolebindings/${entry.namespace}/${entry.name}` : `/resources/clusterrolebindings/_cluster/${entry.name}`} />
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
  .tabs { display: flex; gap: 0.25rem; margin-bottom: 1rem; }
  .tabs button { background: #1e1e3a; color: #9e9e9e; border: 1px solid #333; padding: 0.5rem 1rem; border-radius: 4px 4px 0 0; }
  .tabs button.active { background: #1a73e8; color: white; border-color: #1a73e8; }
  .tabs button:hover:not(.active) { background: #2a2a4a; }
</style>
