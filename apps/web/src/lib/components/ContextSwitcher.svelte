<script lang="ts">
  import { onMount } from 'svelte';
  import { listContexts, connectToContext, listNamespaces, setNamespace } from '$lib/api';
  import type { ClusterContext } from '$lib/tauri-commands';

  let contexts: ClusterContext[] = $state([]);
  let selected: string | null = $state(null);
  let loading = $state(true);
  let connecting = $state(false);
  let error: string | null = $state(null);

  // Namespace state
  let namespaces: string[] = $state(['default']);
  let selectedNamespace = $state('default');
  let namespacesLoading = $state(false);

  onMount(async () => {
    try {
      contexts = await listContexts();
      const active = contexts.find(c => c.is_active);
      selected = active?.name ?? contexts[0]?.name ?? null;
    } catch {
      contexts = [];
    } finally {
      loading = false;
    }
  });

  async function handleContextChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    selected = target.value;
    connecting = true;
    error = null;

    try {
      await connectToContext(selected);
      // After connecting, load namespaces
      namespacesLoading = true;
      namespaces = await listNamespaces();
      selectedNamespace = namespaces.includes('default') ? 'default' : namespaces[0] ?? 'default';
      await setNamespace(selectedNamespace);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Connection failed';
    } finally {
      connecting = false;
      namespacesLoading = false;
    }
  }

  async function handleNamespaceChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    selectedNamespace = target.value;
    try {
      await setNamespace(selectedNamespace);
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
    <label>
      <span class="label-text">Context:</span>
      <select value={selected} onchange={handleContextChange} disabled={connecting}>
        {#each contexts as ctx (ctx.name)}
          <option value={ctx.name}>
            {ctx.name}
            {#if ctx.namespace}({ctx.namespace}){/if}
          </option>
        {/each}
      </select>
    </label>

    {#if connecting}
      <span class="connecting">Connecting…</span>
    {/if}

    {#if !connecting && namespaces.length > 1}
      <label>
        <span class="label-text">Namespace:</span>
        <select value={selectedNamespace} onchange={handleNamespaceChange} disabled={namespacesLoading}>
          {#each namespaces as ns (ns)}
            <option value={ns}>{ns}</option>
          {/each}
        </select>
      </label>
    {/if}

    {#if selected && !connecting}
      {@const ctx = contexts.find(c => c.name === selected)}
      {#if ctx?.cluster_server}
        <span class="server" title={ctx.cluster_server}>
          {ctx.cluster_server}
        </span>
      {/if}
    {/if}

    {#if error}
      <span class="error" title={error}>⚠ {error}</span>
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
