<script lang="ts">
  import { onMount } from 'svelte';
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
  let connecting = $state(false);
  let error: string | null = $state(null);
  let namespacesLoading = $state(false);

  onMount(async () => {
    try {
      contexts = await listContexts();
      const active = contexts.find(c => c.is_active);
      const initial = active?.name ?? contexts[0]?.name ?? null;

      if (initial) {
        selectedContext.set(initial);
        connecting = true;
        try {
          await connectToContext(initial);
          connectionState.set({ state: 'Ready' });
          namespacesLoading = true;
          const nsList = await listNamespaces();
          namespaces.set(nsList);
          const ns = nsList.includes('default') ? 'default' : nsList[0] ?? 'default';
          selectedNamespace.set(ns);
          await setNamespace(ns);
        } catch (err) {
          error = err instanceof Error ? err.message : 'Auto-connect failed';
          connectionState.set({ state: 'Error', detail: { message: error ?? 'Auto-connect failed' } });
        } finally {
          connecting = false;
          namespacesLoading = false;
        }
      }
    } catch {
      contexts = [];
    } finally {
      loading = false;
    }
  });

  async function handleContextChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    const name = target.value;
    selectedContext.set(name);
    connecting = true;
    error = null;
    connectionState.set({ state: 'Connecting' });

    try {
      await connectToContext(name);
      connectionState.set({ state: 'Ready' });
      namespacesLoading = true;
      const nsList = await listNamespaces();
      namespaces.set(nsList);
      const ns = nsList.includes('default') ? 'default' : nsList[0] ?? 'default';
      selectedNamespace.set(ns);
      await setNamespace(ns);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Connection failed. Verify the cluster is reachable and kubeconfig is valid.';
      connectionState.set({ state: 'Error', detail: { message: error ?? 'Connection failed' } });
    } finally {
      connecting = false;
      namespacesLoading = false;
    }
  }

  async function handleNamespaceChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    selectedNamespace.set(target.value);
    try {
      await setNamespace(target.value);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to switch namespace';
    }
  }
</script>

<div class="context-switcher">
  {#if loading}
    <span class="loading">Loading contexts…</span>
  {:else if contexts.length === 0}
    <span class="empty">No contexts found</span>
  {:else}
    <label for="context-select">
      <span class="label-text">Context:</span>
    </label>
    <select
      id="context-select"
      value={$selectedContext}
      onchange={handleContextChange}
      disabled={connecting}
      aria-busy={connecting}
    >
      {#each contexts as ctx (ctx.name)}
        <option value={ctx.name}>
          {ctx.name}
          {#if ctx.namespace}({ctx.namespace}){/if}
        </option>
      {/each}
    </select>

    {#if connecting}
      <span class="connecting">Connecting…</span>
    {/if}

    {#if !connecting && $namespaces.length > 1}
      <label for="namespace-select">
        <span class="label-text">Namespace:</span>
      </label>
      <select
        id="namespace-select"
        value={$selectedNamespace}
        onchange={handleNamespaceChange}
        disabled={namespacesLoading}
        aria-busy={namespacesLoading}
      >
        {#each $namespaces as ns (ns)}
          <option value={ns}>{ns}</option>
        {/each}
      </select>
    {/if}

    {#if $selectedContext && !connecting}
      {@const ctx = contexts.find(c => c.name === $selectedContext)}
      {#if ctx?.cluster_server}
        <span class="server" title={ctx.cluster_server}>
          {ctx.cluster_server}
        </span>
      {/if}
    {/if}

    {#if error}
      <span class="error" role="alert" aria-live="polite" title={error}>⚠ {error}</span>
    {/if}
  {/if}
</div>

<style>
  .context-switcher {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.875rem;
  }
  label {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }
  .label-text {
    color: #9e9e9e;
    white-space: nowrap;
  }
  select {
    background: #1a1a2e;
    color: #e0e0e0;
    border: 1px solid #3a3a5e;
    padding: 0.375rem 0.5rem;
    border-radius: 4px;
    max-width: 250px;
  }
  select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .server {
    color: #757575;
    font-size: 0.75rem;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .connecting {
    color: #42a5f5;
    font-size: 0.75rem;
    animation: pulse 1.5s infinite;
  }
  .error {
    color: #ef5350;
    font-size: 0.75rem;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .loading, .empty {
    color: #757575;
    font-style: italic;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
</style>
