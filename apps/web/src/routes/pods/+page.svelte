<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getPods } from '$lib/api';
  import { selectedNamespace, isConnected } from '$lib/stores';
  import PodTable from '$lib/components/PodTable.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  let pods: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let refreshTimer: ReturnType<typeof setInterval> | null = null;

  async function loadPods() {
    if (!$isConnected) {
      loading = false;
      pods = [];
      return;
    }

    loading = true;
    error = null;
    try {
      pods = await getPods($selectedNamespace);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load pods';
      pods = [];
    } finally {
      loading = false;
    }
  }

  // React to namespace or connection changes from the header
  $effect(() => {
    void $selectedNamespace;
    void $isConnected;
    loadPods();
  });

  onMount(() => {
    refreshTimer = setInterval(loadPods, 3000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });
</script>

<div class="pods-page">
  <header>
    <h1>Pods</h1>
    <div class="controls">
      <span class="ns-label">Namespace: <strong>{$selectedNamespace}</strong></span>
      <button type="button" onclick={loadPods}>Refresh</button>
    </div>
  </header>

  {#if !$isConnected && !loading}
    <div class="not-connected">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if loading}
    <p role="status">Loading pods…</p>
  {:else if error}
    <p role="alert" class="error">Error: {error}</p>
    <button type="button" onclick={loadPods}>Retry</button>
  {:else}
    <p class="count">{pods.length} pod{pods.length !== 1 ? 's' : ''}</p>
    <PodTable {pods} />
  {/if}
</div>

<style>
  .pods-page {
    padding: 1rem;
    color: #e0e0e0;
    background: #0f0f23;
    min-height: 100vh;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  h1 {
    font-size: 1.5rem;
    margin: 0;
  }
  .controls {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }
  button {
    background: #1a73e8;
    color: white;
    border: none;
    padding: 0.375rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
  }
  button:hover { background: #1565c0; }
  .count {
    color: #9e9e9e;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
  }
  .error { color: #ef5350; }
  .not-connected {
    text-align: center;
    padding: 3rem 1rem;
    color: #757575;
  }
  .not-connected p {
    margin: 0.25rem 0;
    font-size: 1.125rem;
  }
  .not-connected .hint {
    font-size: 0.875rem;
    color: #616161;
  }
</style>
