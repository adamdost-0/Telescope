<script lang="ts">
  import { goto } from '$app/navigation';
  import { searchResources, setPreference } from '$lib/api';
  import { kindFromGvk, routeForSearchEntry } from '$lib/resource-routing';
  import type { ResourceEntry } from '$lib/tauri-commands';

  let { open = $bindable(false) }: { open: boolean } = $props();

  let query = $state('');
  let results = $state<ResourceEntry[]>([]);
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  // Scope mode: 'all', 'commands', 'resources', or 'navigation'
  type ScopeMode = 'all' | 'commands' | 'resources' | 'navigation';
  let scopeMode = $state<ScopeMode>('all');

  // Command actions available globally
  const COMMANDS: Record<string, { icon: string; label: string; action: () => void }> = {
    reload: {
      icon: '🔄',
      label: 'Reload Resources',
      action: () => {
        // Placeholder: would trigger a refresh from backend
      },
    },
    theme: {
      icon: '🎨',
      label: 'Toggle Theme',
      action: () => {
        const current = document.documentElement.getAttribute('data-theme') ?? 'dark';
        const next = current === 'dark' ? 'light' : 'dark';
        document.documentElement.setAttribute('data-theme', next);
        setPreference('theme', next);
      },
    },
    settings: {
      icon: '⚙️',
      label: 'Settings',
      action: () => {
        goto('/settings');
      },
    },
  };

  // Navigation pages available
  const PAGES: Record<
    string,
    { icon: string; label: string; route: string; description?: string }
  > = {
    overview: { icon: '📊', label: 'Overview', route: '/overview' },
    pods: { icon: '📦', label: 'Pods', route: '/pods' },
    nodes: { icon: '🖥️', label: 'Nodes', route: '/nodes' },
    nodepools: { icon: '☁️', label: 'Node Pools', route: '/azure/node-pools' },
    deployments: { icon: '🚀', label: 'Deployments', route: '/resources/deployments' },
    replicasets: { icon: '🧬', label: 'ReplicaSets', route: '/resources/replicasets' },
    services: { icon: '🌐', label: 'Services', route: '/resources/services' },
    configmaps: { icon: '📋', label: 'ConfigMaps', route: '/resources/configmaps' },
    clusterroles: { icon: '🧾', label: 'ClusterRoles', route: '/resources/clusterroles' },
    clusterrolebindings: { icon: '🔗', label: 'ClusterRoleBindings', route: '/resources/clusterrolebindings' },
    helm: { icon: '⛵', label: 'Helm Releases', route: '/helm' },
    events: { icon: '⚡', label: 'Events', route: '/events' },
    namespaces: { icon: '📁', label: 'Namespaces', route: '/namespaces' },
    crds: { icon: '🔷', label: 'CRDs', route: '/crds' },
    create: { icon: '✨', label: 'Create Resource', route: '/create' },
    settings: { icon: '⚙️', label: 'Settings', route: '/settings' },
  };

  const KIND_ICONS: Record<string, string> = {
    Pod: '📦',
    Deployment: '🚀',
    StatefulSet: '🗄️',
    DaemonSet: '🔄',
    Service: '🌐',
    Ingress: '🚪',
    ConfigMap: '📋',
    PersistentVolumeClaim: '💾',
    Job: '⚙️',
    CronJob: '🕐',
    Node: '🖥️',
    Event: '⚡',
  };

  function iconForGvk(gvk: string): string {
    return KIND_ICONS[kindFromGvk(gvk)] ?? '📄';
  }

  /** Extract scope prefix from query (>, @, /) and return [mode, cleanQuery]. */
  function detectScope(q: string): [ScopeMode, string] {
    if (q.startsWith('> ')) {
      return ['commands', q.slice(2).trim()];
    } else if (q.startsWith('@ ')) {
      return ['resources', q.slice(2).trim()];
    } else if (q.startsWith('/ ')) {
      return ['navigation', q.slice(2).trim()];
    }
    return ['all', q.trim()];
  }

  /** Filter K8s resources by kind (for @ scope). */
  function isResourceType(entry: ResourceEntry, filterText?: string): boolean {
    // Common K8s resource kinds
    const resourceKinds = new Set([
      'Pod', 'Deployment', 'StatefulSet', 'DaemonSet', 'ReplicaSet',
      'Service', 'Ingress', 'NetworkPolicy', 'EndpointSlice',
      'ConfigMap', 'Secret', 'ResourceQuota', 'LimitRange',
      'PersistentVolume', 'PersistentVolumeClaim', 'StorageClass',
      'Job', 'CronJob',
      'Role', 'ClusterRole', 'RoleBinding', 'ClusterRoleBinding', 'ServiceAccount',
      'HorizontalPodAutoscaler', 'PriorityClass', 'PodDisruptionBudget',
      'ValidatingWebhookConfiguration', 'MutatingWebhookConfiguration',
      'Node', 'Event', 'Namespace',
    ]);
    const kind = kindFromGvk(entry.gvk);
    if (!resourceKinds.has(kind)) return false;
    if (!filterText || filterText.length === 0) return true;
    return (
      entry.name.toLowerCase().includes(filterText.toLowerCase()) ||
      kind.toLowerCase().includes(filterText.toLowerCase())
    );
  }

  /** Filter command results. */
  function filterCommands(filterText?: string): Array<[string, typeof COMMANDS[string]]> {
    const entries = Object.entries(COMMANDS);
    if (!filterText || filterText.length === 0) return entries;
    const lower = filterText.toLowerCase();
    return entries.filter(
      ([_, cmd]) =>
        cmd.label.toLowerCase().includes(lower) || cmd.action.toString().includes(lower)
    );
  }

  /** Filter page results. */
  function filterPages(filterText?: string): Array<[string, typeof PAGES[string]]> {
    const entries = Object.entries(PAGES);
    if (!filterText || filterText.length === 0) return entries;
    const lower = filterText.toLowerCase();
    return entries.filter(
      ([_, page]) =>
        page.label.toLowerCase().includes(lower) ||
        page.route.toLowerCase().includes(lower)
    );
  }

  /** Navigate to the appropriate detail page for a resource entry. */
  function routeForEntry(entry: ResourceEntry): string {
    return routeForSearchEntry(entry);
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

  /** Flat list for command results. */
  let flatCommandResults = $derived.by(() => {
    const [, filterText] = detectScope(query);
    return filterCommands(filterText);
  });

  /** Flat list for page results. */
  let flatPageResults = $derived.by(() => {
    const [, filterText] = detectScope(query);
    return filterPages(filterText);
  });

  /** All results in flat form (depends on scope). */
  let allFlatResults = $derived.by(() => {
    if (scopeMode === 'commands') {
      return flatCommandResults.map(([key, cmd]) => ({
        type: 'command' as const,
        id: key,
        icon: cmd.icon,
        label: cmd.label,
        data: cmd,
      }));
    } else if (scopeMode === 'navigation') {
      return flatPageResults.map(([key, page]) => ({
        type: 'page' as const,
        id: page.route,
        icon: page.icon,
        label: page.label,
        description: page.description,
        data: page,
      }));
    } else if (scopeMode === 'resources') {
      const [, filterText] = detectScope(query);
      return results
        .filter((entry) => isResourceType(entry, filterText))
        .map((entry, idx) => ({
          type: 'resource' as const,
          id: `${entry.gvk}/${entry.namespace}/${entry.name}`,
          icon: iconForGvk(entry.gvk),
          label: entry.name,
          namespace: entry.namespace,
          entry,
          idx,
        }));
    } else {
      // 'all' mode: show a mix
      const [, filterText] = detectScope(query);
      const resourceResults = results
        .filter((entry) => isResourceType(entry, filterText))
        .map((entry, idx) => ({
          type: 'resource' as const,
          id: `${entry.gvk}/${entry.namespace}/${entry.name}`,
          icon: iconForGvk(entry.gvk),
          label: entry.name,
          namespace: entry.namespace,
          entry,
          idx,
        }));
      // Add top commands and pages if no filter text
      const commandResults =
        !filterText || filterText.length === 0
          ? filterCommands(filterText)
              .slice(0, 2)
              .map(([key, cmd]) => ({
                type: 'command' as const,
                id: key,
                icon: cmd.icon,
                label: cmd.label,
                data: cmd,
              }))
          : [];
      const pageResults =
        !filterText || filterText.length === 0
          ? filterPages(filterText)
              .slice(0, 2)
              .map(([_, page]) => ({
                type: 'page' as const,
                id: page.route,
                icon: page.icon,
                label: page.label,
                data: page,
              }))
          : [];
      return [...commandResults, ...pageResults, ...resourceResults];
    }
  });

  function close() {
    open = false;
    query = '';
    results = [];
    selectedIndex = 0;
    scopeMode = 'all';
  }

  function handleOverlayClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      close();
    }
  }

  function selectCommand(commandId: string) {
    const cmd = COMMANDS[commandId];
    if (cmd) {
      cmd.action();
    }
    close();
  }

  function selectPage(pageRoute: string) {
    close();
    goto(pageRoute);
  }

  function selectEntry(entry: ResourceEntry) {
    close();
    goto(routeForEntry(entry));
  }

  function handleSelectedItem() {
    if (allFlatResults.length > 0 && selectedIndex < allFlatResults.length) {
      const item = allFlatResults[selectedIndex];
      if (item.type === 'command') {
        selectCommand(item.id);
      } else if (item.type === 'page') {
        selectPage(item.id);
      } else if (item.type === 'resource' && item.entry) {
        selectEntry(item.entry);
      }
    }
  }

  function handleInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      const [mode, cleanQuery] = detectScope(query);
      scopeMode = mode;

      if (cleanQuery.length === 0 && mode !== 'commands' && mode !== 'navigation') {
        results = [];
        selectedIndex = 0;
        return;
      }

      // Fetch K8s resources if not purely command/navigation scope
      if (mode === 'resources' || mode === 'all') {
        results = await searchResources(cleanQuery);
      } else {
        results = [];
      }

      selectedIndex = 0;
    }, 200);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (allFlatResults.length > 0) {
        selectedIndex = (selectedIndex + 1) % allFlatResults.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (allFlatResults.length > 0) {
        selectedIndex = (selectedIndex - 1 + allFlatResults.length) % allFlatResults.length;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      handleSelectedItem();
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
          placeholder="Search… (> commands, @ resources, / pages)"
          type="text"
          spellcheck="false"
          autocomplete="off"
          aria-label="Search resources, commands, and pages"
        />
        {#if scopeMode !== 'all'}
          <span class="scope-badge">
            {#if scopeMode === 'commands'}
              > Commands
            {:else if scopeMode === 'resources'}
              @ Resources
            {:else if scopeMode === 'navigation'}
              / Pages
            {/if}
          </span>
        {/if}
        <kbd class="hint">ESC</kbd>
      </div>

      {#if allFlatResults.length > 0}
        <div class="results" role="listbox">
          {#each allFlatResults as item, idx}
            {#if item.type === 'command'}
              <button
                class="result-item"
                class:selected={idx === selectedIndex}
                role="option"
                aria-selected={idx === selectedIndex}
                onclick={() => selectCommand(item.id)}
                onmouseenter={() => (selectedIndex = idx)}
              >
                <span class="item-icon">{item.icon}</span>
                <span class="item-name">{item.label}</span>
              </button>
            {:else if item.type === 'page'}
              <button
                class="result-item"
                class:selected={idx === selectedIndex}
                role="option"
                aria-selected={idx === selectedIndex}
                onclick={() => selectPage(item.id)}
                onmouseenter={() => (selectedIndex = idx)}
              >
                <span class="item-icon">{item.icon}</span>
                <span class="item-name">{item.label}</span>
              </button>
            {:else if item.type === 'resource'}
              <button
                class="result-item"
                class:selected={idx === selectedIndex}
                role="option"
                aria-selected={idx === selectedIndex}
                onclick={() => selectEntry(item.entry)}
                onmouseenter={() => (selectedIndex = idx)}
              >
                <span class="item-icon">{item.icon}</span>
                <span class="entry-name">{item.label}</span>
                {#if item.namespace}
                  <span class="entry-ns">{item.namespace}</span>
                {/if}
              </button>
            {/if}
          {/each}
        </div>
      {:else if query.trim().length > 0}
        <div class="empty">
          {#if scopeMode === 'commands'}
            No commands match "{query.slice(2).trim()}"
          {:else if scopeMode === 'resources'}
            No resources match "{query.slice(2).trim()}"
          {:else if scopeMode === 'navigation'}
            No pages match "{query.slice(2).trim()}"
          {:else}
            No results for "{query}"
          {/if}
        </div>
      {:else}
        <div class="empty">
          <div class="help-text">Type to search:</div>
          <div class="help-examples">
            <div class="help-item"><kbd>></kbd> Commands (reload, theme, settings)</div>
            <div class="help-item"><kbd>@</kbd> Resources (pods, deployments, services…)</div>
            <div class="help-item"><kbd>/</kbd> Pages (overview, nodes, helm…)</div>
          </div>
        </div>
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
  .scope-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: rgba(88, 166, 255, 0.15);
    color: #58a6ff;
    padding: 0.25rem 0.6rem;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    flex-shrink: 0;
    border: 1px solid rgba(88, 166, 255, 0.25);
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
  .result-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.65rem 1rem;
    background: none;
    border: none;
    color: #c9d1d9;
    font-size: 0.875rem;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    transition: background-color 0.15s ease;
  }
  .result-item:hover,
  .result-item.selected {
    background: #1f2937;
  }
  .item-icon {
    font-size: 1.1rem;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
  }
  .item-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .help-text {
    color: #8b949e;
    font-weight: 500;
    margin-bottom: 0.5rem;
  }
  .help-examples {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: center;
  }
  .help-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: #6e7681;
  }
  .help-item kbd {
    background: #21262d;
    color: #79c0ff;
    padding: 0.2rem 0.35rem;
    border-radius: 3px;
    border: 1px solid #30363d;
    font-family: monospace;
    font-size: 0.7rem;
    font-weight: 600;
  }
</style>
