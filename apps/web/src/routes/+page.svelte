<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { listContexts, connectToContext, listNamespaces, setNamespace } from '$lib/api';
  import {
    selectedContext,
    selectedNamespace,
    namespaces,
    connectionState,
  } from '$lib/stores';
  import type { ClusterContext } from '$lib/tauri-commands';

  let contexts: ClusterContext[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let connectingTo: string | null = $state(null);

  onMount(async () => {
    try {
      contexts = await listContexts();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load clusters';
    } finally {
      loading = false;
    }
  });

  async function handleConnect(contextName: string) {
    connectingTo = contextName;
    error = null;
    connectionState.set({ state: 'Connecting' });

    try {
      await connectToContext(contextName);
      selectedContext.set(contextName);
      connectionState.set({ state: 'Ready' });

      const nsList = await listNamespaces();
      namespaces.set(nsList);
      const ns = nsList.includes('default') ? 'default' : nsList[0] ?? 'default';
      selectedNamespace.set(ns);
      await setNamespace(ns);

      goto('/pods');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Connection failed';
      connectionState.set({ state: 'Error', detail: { message: error } });
      connectingTo = null;
    }
  }
</script>

<h1>Telescope</h1>

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
          <button type="button" onclick={() => handleConnect(ctx.name)} disabled={connectingTo !== null}>
            <strong>{ctx.name}</strong>
            {#if ctx.cluster_server}
              <span style="color: #888; font-size: 0.85em"> — {ctx.cluster_server}</span>
            {/if}
            {#if ctx.is_active}
              <span style="color: #66bb6a;"> ●</span>
            {/if}
            {#if connectingTo === ctx.name}
              <span style="color: #42a5f5;"> connecting…</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  button {
    background: none;
    border: 1px solid #2a2a3e;
    color: inherit;
    padding: 0.75rem 1rem;
    width: 100%;
    text-align: left;
    cursor: pointer;
    border-radius: 6px;
    font-size: inherit;
  }
  button:hover { background: #16213e; border-color: #3a3a5e; }
  button:disabled { opacity: 0.6; cursor: wait; }
  ul { list-style: none; padding: 0; }
  li { margin-bottom: 0.5rem; }
</style>
