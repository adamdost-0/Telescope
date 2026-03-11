<script lang="ts">
  import { onMount } from 'svelte';
  import { listContexts } from '$lib/api';
  import type { ClusterContext } from '$lib/tauri-commands';

  let contexts: ClusterContext[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);

  onMount(async () => {
    try {
      contexts = await listContexts();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load clusters';
    } finally {
      loading = false;
    }
  });
</script>

<h1>Telescope</h1>

<nav aria-label="resources">
  <a href="/pods">Pods</a>
</nav>

<section aria-label="clusters">
  <h2>Clusters</h2>

  {#if loading}
    <p role="status">Loading clusters…</p>
  {:else if error}
    <p role="alert">{error}</p>
  {:else if contexts.length === 0}
    <p>No clusters found. Check your kubeconfig at <code>~/.kube/config</code></p>
  {:else}
    <ul>
      {#each contexts as ctx (ctx.name)}
        <li data-testid="cluster-item">
          <a href="/pods">
            <strong>{ctx.name}</strong>
            {#if ctx.cluster_server}
              <span style="color: #888; font-size: 0.85em"> — {ctx.cluster_server}</span>
            {/if}
            {#if ctx.is_active}
              <span style="color: #66bb6a;"> ●</span>
            {/if}
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>
