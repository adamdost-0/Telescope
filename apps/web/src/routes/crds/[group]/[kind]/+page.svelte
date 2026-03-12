<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import { getResources } from '$lib/api';
  import { selectedNamespace, isConnected } from '$lib/stores';
  import ResourceTable from '$lib/components/ResourceTable.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  const group = $derived(decodeURIComponent(page.params.group));
  const kind = $derived(decodeURIComponent(page.params.kind));

  // Read query params for version, plural, and scope
  const version = $derived(page.url.searchParams.get('version') ?? 'v1');
  const plural = $derived(page.url.searchParams.get('plural') ?? kind.toLowerCase() + 's');
  const scope = $derived(page.url.searchParams.get('scope') ?? 'Namespaced');
  const isNamespaced = $derived(scope === 'Namespaced' || scope === '"Namespaced"');

  const gvk = $derived(`${group}/${version}/${kind}`);

  const columns = [
    { key: 'name', label: 'Name', extract: (c: any) => c?.metadata?.name ?? '', width: '30%' },
    { key: 'namespace', label: 'Namespace', extract: (c: any) => c?.metadata?.namespace ?? '—' },
    { key: 'age', label: 'Age', extract: (c: any) => {
      const ts = c?.metadata?.creationTimestamp;
      if (!ts) return 'Unknown';
      const diffSec = Math.floor((Date.now() - new Date(ts).getTime()) / 1000);
      if (diffSec < 60) return `${diffSec}s`;
      const diffMin = Math.floor(diffSec / 60);
      if (diffMin < 60) return `${diffMin}m`;
      const diffHours = Math.floor(diffMin / 60);
      if (diffHours < 24) return `${diffHours}h`;
      return `${Math.floor(diffHours / 24)}d`;
    }},
  ];

  let resources: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;

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
      const ns = isNamespaced ? $selectedNamespace : undefined;
      resources = await getResources(gvk, ns);
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : `Failed to load ${kind} resources`;
      resources = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  $effect(() => {
    void gvk;
    void $selectedNamespace;
    void $isConnected;
    loadResources();
  });

  onMount(() => {
    refreshTimer = setInterval(loadResources, 3000);
    timestampTimer = setInterval(() => { lastUpdatedText = formatTimestamp(); }, 1000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="resource-page">
  <header>
    <div>
      <h1>{kind}</h1>
      <p class="subtitle">{group}/{version} · {scope.replace(/"/g, '')}</p>
    </div>
    <div class="controls">
      {#if isNamespaced}
        <span class="ns-label">Namespace: <strong>{$selectedNamespace}</strong></span>
      {/if}
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" onclick={loadResources} disabled={refreshing} class:spinning={refreshing} aria-label="Refresh {kind}">
        <span class="refresh-icon">↻</span> Refresh
      </button>
    </div>
  </header>

  <nav class="breadcrumb">
    <a href="/crds">CRDs</a> <span>›</span> <span>{kind}</span>
  </nav>

  {#if !$isConnected && !loading}
    <div class="not-connected">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if loading}
    <LoadingSkeleton rows={8} columns={columns.length} />
  {:else if error}
    <div role="alert" aria-live="polite" class="error-container">
      <p class="error">Failed to load {kind} resources.</p>
      {#if error !== `Failed to load ${kind} resources`}
        <p class="error-detail">{error}</p>
      {/if}
      <button type="button" onclick={loadResources}>Retry</button>
    </div>
  {:else}
    <p class="count">{resources.length} {kind.toLowerCase()}{resources.length !== 1 ? 's' : ''}</p>
    <ResourceTable
      {resources}
      {columns}
      emptyMessage="No {kind} resources found."
      hrefFn={(entry) => `/resources/${plural}/${entry.namespace}/${entry.name}`}
    />
  {/if}
</div>

<style>
  .resource-page { padding: 1rem; color: #e0e0e0; background: #0f0f23; min-height: 100vh; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
  h1 { font-size: 1.5rem; margin: 0; }
  .subtitle { margin: 0.125rem 0 0; font-size: 0.8rem; color: #8b949e; }
  .controls { display: flex; gap: 0.75rem; align-items: center; }
  .last-updated { color: #757575; font-size: 0.75rem; }
  .ns-label { color: #9e9e9e; font-size: 0.85rem; }
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
  .breadcrumb { margin-bottom: 1rem; font-size: 0.8rem; color: #8b949e; display: flex; gap: 0.35rem; align-items: center; }
  .breadcrumb a { color: #58a6ff; text-decoration: none; }
  .breadcrumb a:hover { text-decoration: underline; }
</style>
