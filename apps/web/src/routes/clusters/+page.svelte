<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { listContexts, connectToContext, listNamespaces, setNamespace } from '$lib/api';
  import { getPreferredNamespace } from '$lib/preferences';
  import {
    selectedContext,
    selectedNamespace,
    namespaces,
    connectionState,
  } from '$lib/stores';
  import type { ClusterContext } from '$lib/tauri-commands';

  let loading = $state(true);
  let error: string | null = $state(null);
  let contexts: ClusterContext[] = $state([]);
  let connectingTo: string | null = $state(null);

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
    // Don't reconnect if already connected to this context
    if (name === $selectedContext) {
      void goto('/pods');
      return;
    }

    connectingTo = name;
    error = null;
    connectionState.set({ state: 'Connecting' });

    try {
      await connectToContext(name);
      selectedContext.set(name);
      connectionState.set({ state: 'Ready' });

      const nsList = await listNamespaces();
      namespaces.set(nsList);
      const ns = await getPreferredNamespace(nsList);
      selectedNamespace.set(ns);
      await setNamespace(ns);

      void goto('/pods');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to connect';
      connectionState.set({ state: 'Error', detail: { message: error } });
      connectingTo = null;
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
        <button type="button" onclick={() => selectContext(ctx.name)} disabled={connectingTo !== null}>
          <strong>{ctx.name}</strong>
          {#if ctx.auth_type === 'exec'}
            <span style="font-size: 0.7rem; padding: 0.125rem 0.375rem; border-radius: 3px; background: #2d2040; color: #ce93d8; border: 1px solid #4a2d6e;">🔑 Exec</span>
          {:else if ctx.auth_type === 'token'}
            <span style="font-size: 0.7rem; padding: 0.125rem 0.375rem; border-radius: 3px; background: #1a2e3e; color: #81d4fa; border: 1px solid #2a4a5e;">🎫 Token</span>
          {:else if ctx.auth_type === 'certificate'}
            <span style="font-size: 0.7rem; padding: 0.125rem 0.375rem; border-radius: 3px; background: #1e3a2a; color: #a5d6a7; border: 1px solid #2e5a3a;">📜 Cert</span>
          {/if}
          {#if ctx.cluster_server}
            <span style="color: #888; font-size: 0.85em"> — {ctx.cluster_server}</span>
          {/if}
          {#if ctx.name === $selectedContext}
            <span style="color: #66bb6a;"> Connected ✓</span>
          {:else if ctx.is_active}
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
