<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listCrds } from '$lib/api';
  import { isConnected } from '$lib/stores';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import type { CrdInfo } from '$lib/tauri-commands';

  let crds: CrdInfo[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let refreshing = $state(false);
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;
  let search = $state('');

  let filtered = $derived(
    search
      ? crds.filter(
          (c) =>
            c.name.toLowerCase().includes(search.toLowerCase()) ||
            c.group.toLowerCase().includes(search.toLowerCase()) ||
            c.kind.toLowerCase().includes(search.toLowerCase()),
        )
      : crds,
  );

  function formatTimestamp(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    return `Updated ${Math.floor(seconds / 60)}m ago`;
  }

  async function loadCrds() {
    if (!$isConnected) { loading = false; crds = []; return; }
    const isInitial = crds.length === 0 && !lastUpdated;
    if (isInitial) loading = true; else refreshing = true;
    error = null;
    try {
      crds = await listCrds();
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load CRDs';
      crds = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  $effect(() => {
    void $isConnected;
    loadCrds();
  });

  onMount(() => {
    refreshTimer = setInterval(loadCrds, 10000);
    timestampTimer = setInterval(() => { lastUpdatedText = formatTimestamp(); }, 1000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="resource-page">
  <header>
    <h1>Custom Resource Definitions</h1>
    <div class="controls">
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" onclick={loadCrds} disabled={refreshing} class:spinning={refreshing} aria-label="Refresh CRDs">
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
    <LoadingSkeleton rows={8} columns={6} />
  {:else if error}
    <div role="alert" aria-live="polite" class="error-container">
      <p class="error">Failed to load CRDs. Check cluster connection and try again.</p>
      {#if error !== 'Failed to load CRDs'}
        <p class="error-detail">{error}</p>
      {/if}
      <button type="button" onclick={loadCrds}>Retry</button>
    </div>
  {:else}
    <div class="search-bar">
      <input type="text" placeholder="Filter CRDs by name, group, or kind…" bind:value={search} />
    </div>
    <p class="count">{filtered.length} of {crds.length} CRDs</p>

    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Group</th>
            <th>Kind</th>
            <th>Version</th>
            <th>Scope</th>
            <th>Short Names</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as crd (crd.name)}
            <tr>
              <td>
                <a href="/crds/{encodeURIComponent(crd.group)}/{encodeURIComponent(crd.kind)}?version={encodeURIComponent(crd.version)}&plural={encodeURIComponent(crd.plural)}&scope={encodeURIComponent(crd.scope)}">
                  {crd.name}
                </a>
              </td>
              <td>{crd.group}</td>
              <td><code>{crd.kind}</code></td>
              <td>{crd.version}</td>
              <td><span class="badge" class:namespaced={crd.scope === '"Namespaced"' || crd.scope === 'Namespaced'}>{crd.scope.replace(/"/g, '')}</span></td>
              <td>{crd.short_names.length ? crd.short_names.join(', ') : '—'}</td>
            </tr>
          {/each}
          {#if filtered.length === 0}
            <tr><td colspan="6" class="empty">No CRDs match the filter.</td></tr>
          {/if}
        </tbody>
      </table>
    </div>
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

  .search-bar { margin-bottom: 0.75rem; }
  .search-bar input {
    width: 100%;
    max-width: 400px;
    padding: 0.5rem 0.75rem;
    border: 1px solid #30363d;
    border-radius: 6px;
    background: #161b22;
    color: #e0e0e0;
    font-size: 0.875rem;
  }
  .search-bar input::placeholder { color: #484f58; }
  .search-bar input:focus { outline: none; border-color: #58a6ff; }

  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  thead th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #30363d;
    color: #8b949e;
    font-weight: 600;
    white-space: nowrap;
  }
  tbody td {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #21262d;
    white-space: nowrap;
  }
  tbody tr:hover { background: #161b22; }
  a { color: #58a6ff; text-decoration: none; }
  a:hover { text-decoration: underline; }
  code { background: #21262d; padding: 0.1rem 0.35rem; border-radius: 3px; font-size: 0.8rem; }
  .badge {
    display: inline-block;
    padding: 0.1rem 0.5rem;
    border-radius: 10px;
    font-size: 0.75rem;
    background: #30363d;
    color: #8b949e;
  }
  .badge.namespaced { background: #1a3a2a; color: #3fb950; }
  .empty { text-align: center; color: #757575; padding: 2rem !important; }
</style>
