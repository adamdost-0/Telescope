<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getEvents } from '$lib/api';
  import { namespaces, isConnected } from '$lib/stores';
  import EventsTable from '$lib/components/EventsTable.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  let events: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let filterType = $state('all');
  let filterNamespace = $state('__all__');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;

  let filteredEvents = $derived.by(() => {
    let result = events;
    if (filterType !== 'all') {
      result = result.filter((e) => {
        try {
          return JSON.parse(e.content).type === filterType;
        } catch {
          return false;
        }
      });
    }
    if (filterNamespace !== '__all__') {
      result = result.filter((e) => e.namespace === filterNamespace);
    }
    return result;
  });

  /** Unique namespaces present in the current events list. */
  let eventNamespaces = $derived(
    [...new Set(events.map((e) => e.namespace).filter(Boolean))].sort()
  );

  async function loadEvents() {
    if (!$isConnected) {
      loading = false;
      events = [];
      return;
    }
    loading = events.length === 0;
    error = null;
    try {
      events = await getEvents(null);
      lastUpdated = new Date();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load events';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Re-load when connection state changes
    const _conn = $isConnected;
    loadEvents();
  });

  onMount(() => {
    refreshTimer = setInterval(loadEvents, 3000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });
</script>

<div class="events-page">
  <header>
    <h1>Events</h1>
    <div class="controls">
      <select
        value={filterNamespace}
        onchange={(e) => (filterNamespace = (e.target as HTMLSelectElement).value)}
        aria-label="Filter by namespace"
      >
        <option value="__all__">All Namespaces</option>
        {#each eventNamespaces as ns}
          <option value={ns}>{ns}</option>
        {/each}
      </select>
      <select
        value={filterType}
        onchange={(e) => (filterType = (e.target as HTMLSelectElement).value)}
        aria-label="Filter by event type"
      >
        <option value="all">All Types</option>
        <option value="Warning">⚠ Warnings</option>
        <option value="Normal">✓ Normal</option>
      </select>
      {#if lastUpdated}
        <span class="timestamp">
          Updated {Math.floor((Date.now() - lastUpdated.getTime()) / 1000)}s ago
        </span>
      {/if}
      <span class="count">{filteredEvents.length} events</span>
    </div>
  </header>

  {#if !$isConnected}
    <p class="not-connected">🔌 Connect to a cluster to view events.</p>
  {:else if loading}
    <LoadingSkeleton rows={10} columns={5} />
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {:else}
    <EventsTable events={filteredEvents} />
  {/if}
</div>

<style>
  .events-page {
    padding: 1.5rem;
    color: #e0e0e0;
    max-width: 1200px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.25rem;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  h1 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
    color: #e0e0e0;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.875rem;
  }

  select {
    background: #161b22;
    color: #e0e0e0;
    border: 1px solid #21262d;
    border-radius: 4px;
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
    cursor: pointer;
  }
  select:focus {
    outline: none;
    border-color: #58a6ff;
  }

  .timestamp {
    color: #6e7681;
    font-size: 0.75rem;
  }

  .count {
    background: #1a1a2e;
    color: #8b949e;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    border: 1px solid #21262d;
  }

  .not-connected {
    color: #8b949e;
    font-size: 0.9rem;
    padding: 2rem;
    text-align: center;
    background: #161b22;
    border: 1px dashed #21262d;
    border-radius: 6px;
  }

  .error {
    color: #ef5350;
    background: rgba(239, 83, 80, 0.1);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    border: 1px solid rgba(239, 83, 80, 0.3);
  }
</style>
