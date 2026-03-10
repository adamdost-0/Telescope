<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import type { Cluster } from '$lib/engine';

  let loading = true;
  let error: string | null = null;
  let clusters: Cluster[] = [];

  async function loadClusters() {
    loading = true;
    error = null;

    try {
      const res = await fetch('/api/clusters', {
        headers: { accept: 'application/json' }
      });

      if (!res.ok) throw new Error(`Request failed (${res.status})`);

      const data = (await res.json()) as { clusters?: Cluster[] };
      clusters = data.clusters ?? [];
    } catch (e) {
      error = e instanceof Error ? e.message : 'Unknown error';
      clusters = [];
    } finally {
      loading = false;
    }
  }

  onMount(loadClusters);

  function selectCluster(id: string) {
    void goto(`/explore?cluster=${encodeURIComponent(id)}`);
  }
</script>

<h1>Clusters</h1>

{#if loading}
  <p role="status">Loading clusters…</p>
{:else if error}
  <p role="alert">Failed to load clusters: {error}</p>
  <button type="button" on:click={loadClusters}>Retry</button>
{:else if clusters.length === 0}
  <p>No clusters found.</p>
{:else}
  <ul>
    {#each clusters as c (c.id)}
      <li>
        <button type="button" on:click={() => selectCluster(c.id)}>
          {c.name} ({c.id})
        </button>
      </li>
    {/each}
  </ul>
{/if}
