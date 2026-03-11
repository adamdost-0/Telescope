<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { listContexts, connectToContext } from '$lib/api';
  import type { ClusterContext } from '$lib/tauri-commands';

  let loading = $state(true);
  let error: string | null = $state(null);
  let contexts: ClusterContext[] = $state([]);

  async function loadContexts() {
    loading = true;
    error = null;

    try {
      contexts = await listContexts();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Unknown error';
      contexts = [];
    } finally {
      loading = false;
    }
  }

  onMount(loadContexts);

  async function selectContext(name: string) {
    try {
      await connectToContext(name);
      void goto('/pods');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to connect';
    }
  }
</script>

<h1>Clusters</h1>

{#if loading}
  <p role="status">Loading clusters…</p>
{:else if error}
  <p role="alert">Failed to load clusters: {error}</p>
  <button type="button" onclick={loadContexts}>Retry</button>
{:else if contexts.length === 0}
  <p>No clusters found. Check your kubeconfig at <code>~/.kube/config</code></p>
{:else}
  <ul>
    {#each contexts as ctx (ctx.name)}
      <li>
        <button type="button" onclick={() => selectContext(ctx.name)}>
          <strong>{ctx.name}</strong>
          {#if ctx.cluster_server}
            <span style="color: #888; font-size: 0.85em"> — {ctx.cluster_server}</span>
          {/if}
          {#if ctx.is_active}
            <span style="color: #66bb6a;"> ●</span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>
{/if}
