<script lang="ts">
  import { onMount } from 'svelte';
  import { getPods } from '$lib/api';
  import PodTable from '$lib/components/PodTable.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  let pods: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let namespace = $state('default');

  async function loadPods() {
    loading = true;
    error = null;
    try {
      pods = await getPods(namespace);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load pods';
      pods = [];
    } finally {
      loading = false;
    }
  }

  onMount(loadPods);

  function handleNamespaceChange(e: Event) {
    const target = e.target as HTMLInputElement;
    namespace = target.value;
    loadPods();
  }
</script>

<div class="pods-page">
  <header>
    <h1>Pods</h1>
    <div class="controls">
      <label>
        Namespace:
        <input type="text" value={namespace} onchange={handleNamespaceChange} />
      </label>
      <button type="button" onclick={loadPods}>Refresh</button>
    </div>
  </header>

  {#if loading}
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
  input {
    background: #1a1a2e;
    border: 1px solid #3a3a5e;
    color: #e0e0e0;
    padding: 0.375rem 0.5rem;
    border-radius: 4px;
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
</style>
