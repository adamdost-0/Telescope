<script lang="ts">
  import { onMount } from 'svelte';
  import { listContexts } from '$lib/api';
  import type { ClusterContext } from '$lib/tauri-commands';

  let contexts: ClusterContext[] = $state([]);
  let selected: string | null = $state(null);
  let loading = $state(true);

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

  function handleChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    selected = target.value;
    // TODO: In Phase 3, this will call set_context and restart watchers
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
      <select value={selected} onchange={handleChange}>
        {#each contexts as ctx (ctx.name)}
          <option value={ctx.name}>
            {ctx.name}
            {#if ctx.namespace}({ctx.namespace}){/if}
          </option>
        {/each}
      </select>
    </label>
    {#if selected}
      {@const ctx = contexts.find(c => c.name === selected)}
      {#if ctx?.cluster_server}
        <span class="server" title={ctx.cluster_server}>
          {ctx.cluster_server}
        </span>
      {/if}
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
  .server {
    color: #757575;
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
</style>
