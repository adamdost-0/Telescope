<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getPods, getPodMetrics } from '$lib/api';
  import { selectedNamespace, isConnected } from '$lib/stores';
  import PodTable from '$lib/components/PodTable.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import type { ResourceEntry, PodMetrics } from '$lib/tauri-commands';

  let pods: ResourceEntry[] = $state([]);
  let metrics: PodMetrics[] = $state([]);
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
    const minutes = Math.floor(seconds / 60);
    return `Updated ${minutes}m ago`;
  }

  async function loadPods() {
    if (!$isConnected) {
      loading = false;
      pods = [];
      metrics = [];
      return;
    }

    const isInitial = pods.length === 0 && !lastUpdated;
    if (isInitial) {
      loading = true;
    } else {
      refreshing = true;
    }
    error = null;
    try {
      const [podResult, metricsResult] = await Promise.all([
        getPods($selectedNamespace),
        getPodMetrics($selectedNamespace),
      ]);
      pods = podResult;
      metrics = metricsResult;
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load pods';
      pods = [];
      metrics = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  $effect(() => {
    void $selectedNamespace;
    void $isConnected;
    loadPods();
  });

  onMount(() => {
    refreshTimer = setInterval(loadPods, 3000);
    timestampTimer = setInterval(() => {
      lastUpdatedText = formatTimestamp();
    }, 1000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="pods-page">
  <header>
    <h1>Pods</h1>
    <div class="controls">
      <span class="ns-label">Namespace: <strong>{$selectedNamespace}</strong></span>
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button
        type="button"
        onclick={loadPods}
        disabled={refreshing}
        class:spinning={refreshing}
        aria-label="Refresh pods"
      >
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
    <LoadingSkeleton rows={5} columns={5} />
  {:else if error}
    <div role="alert" aria-live="polite" class="error-container">
      <p class="error">Failed to load pods. Check cluster connection and try again.</p>
      {#if error !== 'Failed to load pods'}
        <p class="error-detail">{error}</p>
      {/if}
      <button type="button" onclick={loadPods}>Retry</button>
    </div>
  {:else}
    <p class="count">{pods.length} pod{pods.length !== 1 ? 's' : ''}</p>
    <PodTable {pods} {metrics} />
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
  button:hover { background: #1565c0; }
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
  .error-container {
    padding: 1.5rem;
    text-align: center;
  }
  .error { color: #ef5350; }
  .error-detail {
    color: #9e9e9e;
    font-size: 0.875rem;
    margin-top: 0.25rem;
  }
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
