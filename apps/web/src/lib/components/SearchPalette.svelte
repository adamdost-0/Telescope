<script lang="ts">
  import { goto } from '$app/navigation';
  import { searchResources } from '$lib/api';
  import type { ResourceEntry } from '$lib/tauri-commands';

  let { open = $bindable(false) }: { open: boolean } = $props();

  let query = $state('');
  let results = $state<ResourceEntry[]>([]);
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  const KIND_ICONS: Record<string, string> = {
    Pod: '📦',
    Deployment: '🚀',
    StatefulSet: '🗄️',
    DaemonSet: '🔄',
    Service: '🌐',
    ConfigMap: '📋',
    Job: '⚙️',
    CronJob: '🕐',
    Node: '🖥️',
    Event: '⚡',
  };

  /** Map a GVK string like "apps/v1/Deployment" → its kind name. */
  function kindFromGvk(gvk: string): string {
    const parts = gvk.split('/');
    return parts[parts.length - 1];
  }

  function iconForGvk(gvk: string): string {
    return KIND_ICONS[kindFromGvk(gvk)] ?? '📄';
  }

  /** Navigate to the appropriate detail page for a resource entry. */
  function routeForEntry(entry: ResourceEntry): string {
    const kind = kindFromGvk(entry.gvk).toLowerCase();
    const ns = entry.namespace || 'default';

    switch (kind) {
      case 'pod':
        return `/pods/${ns}/${entry.name}`;
      case 'node':
        return `/nodes/${entry.name}`;
      case 'event':
        return `/events`;
      case 'deployment':
        return `/resources/deployments/${ns}/${entry.name}`;
      case 'statefulset':
        return `/resources/statefulsets/${ns}/${entry.name}`;
      case 'daemonset':
        return `/resources/daemonsets/${ns}/${entry.name}`;
      case 'job':
        return `/resources/jobs/${ns}/${entry.name}`;
      case 'cronjob':
        return `/resources/cronjobs/${ns}/${entry.name}`;
      case 'service':
        return `/resources/services/${ns}/${entry.name}`;
      case 'configmap':
        return `/resources/configmaps/${ns}/${entry.name}`;
      default:
        return `/resources/${kind}/${ns}/${entry.name}`;
    }
  }

  /** Group results by kind for display. */
  let grouped = $derived.by(() => {
    const groups = new Map<string, ResourceEntry[]>();
    for (const entry of results) {
      const kind = kindFromGvk(entry.gvk);
      if (!groups.has(kind)) groups.set(kind, []);
      groups.get(kind)!.push(entry);
    }
    return groups;
  });

  /** Flat list for arrow-key navigation (preserves render order). */
  let flatResults = $derived.by(() => {
    const flat: ResourceEntry[] = [];
    for (const entries of grouped.values()) {
      flat.push(...entries);
    }
    return flat;
  });

  function close() {
    open = false;
    query = '';
    results = [];
    selectedIndex = 0;
  }

  function handleOverlayClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      close();
    }
  }

  function selectEntry(entry: ResourceEntry) {
    close();
    goto(routeForEntry(entry));
  }

  function handleInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      if (query.trim().length === 0) {
        results = [];
        selectedIndex = 0;
        return;
      }
      results = await searchResources(query.trim());
      selectedIndex = 0;
    }, 200);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (flatResults.length > 0) {
        selectedIndex = (selectedIndex + 1) % flatResults.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (flatResults.length > 0) {
        selectedIndex = (selectedIndex - 1 + flatResults.length) % flatResults.length;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (flatResults.length > 0 && flatResults[selectedIndex]) {
        selectEntry(flatResults[selectedIndex]);
      }
    }
  }

  $effect(() => {
    if (open && inputEl) {
      inputEl.focus();
    }
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" role="presentation" onkeydown={handleKeydown} onclick={handleOverlayClick}>
    <div class="palette" role="dialog" aria-modal="true" aria-label="Search resources" tabindex="-1">
      <div class="search-bar">
        <span class="search-icon">🔍</span>
        <input
          bind:this={inputEl}
          bind:value={query}
          oninput={handleInput}
          placeholder="Search resources… (name or kind)"
          type="text"
          spellcheck="false"
          autocomplete="off"
          aria-label="Search resources"
        />
        <kbd class="hint">ESC</kbd>
      </div>

      {#if flatResults.length > 0}
        <div class="results" role="listbox">
          {#each grouped as [kind, entries]}
            <div class="group-header">{iconForGvk(entries[0].gvk)} {kind}</div>
            {#each entries as entry}
              {@const idx = flatResults.indexOf(entry)}
              <button
                class="result-item"
                class:selected={idx === selectedIndex}
                role="option"
                aria-selected={idx === selectedIndex}
                onclick={() => selectEntry(entry)}
                onmouseenter={() => (selectedIndex = idx)}
              >
                <span class="entry-name">{entry.name}</span>
                {#if entry.namespace}
                  <span class="entry-ns">{entry.namespace}</span>
                {/if}
              </button>
            {/each}
          {/each}
        </div>
      {:else if query.trim().length > 0}
        <div class="empty">No results for "{query}"</div>
      {:else}
        <div class="empty">Type to search pods, deployments, services…</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 100;
    display: flex;
    justify-content: center;
    padding-top: 15vh;
  }
  .palette {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 12px;
    width: min(560px, 90vw);
    max-height: 420px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }
  .search-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #21262d;
  }
  .search-icon {
    font-size: 1rem;
    opacity: 0.6;
  }
  input {
    flex: 1;
    background: none;
    border: none;
    color: #e0e0e0;
    font-size: 1rem;
    outline: none;
    font-family: inherit;
  }
  input::placeholder {
    color: #484f58;
  }
  .hint {
    background: #21262d;
    color: #8b949e;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    font-size: 0.7rem;
    font-family: inherit;
    border: 1px solid #30363d;
  }
  .results {
    overflow-y: auto;
    padding: 0.25rem 0;
  }
  .group-header {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #484f58;
    padding: 0.5rem 1rem 0.25rem;
  }
  .result-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 1rem;
    background: none;
    border: none;
    color: #c9d1d9;
    font-size: 0.875rem;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
  }
  .result-item:hover,
  .result-item.selected {
    background: #1f2937;
  }
  .entry-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry-ns {
    font-size: 0.75rem;
    color: #8b949e;
    background: #21262d;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .empty {
    padding: 2rem 1rem;
    text-align: center;
    color: #484f58;
    font-size: 0.875rem;
  }
</style>
