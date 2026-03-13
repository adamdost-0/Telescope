<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { getPodLogs, listContainers, startLogStream } from '$lib/api';

  let { namespace, pod }: { namespace: string; pod: string } = $props();

  let logs = $state('');
  let containers: string[] = $state([]);
  let selectedContainer: string | null = $state(null);
  let loading = $state(true);
  let streaming = $state(false);
  let error: string | null = $state(null);
  let autoScroll = $state(true);
  let showPrevious = $state(false);
  let searchQuery = $state('');
  let tailLines = $state(500);

  let logContainer: HTMLElement | undefined = $state();
  let cleanup: (() => void) | null = null;

  let filteredLogs = $derived.by(() => {
    if (!searchQuery.trim()) return logs;
    const query = searchQuery.toLowerCase();
    return logs.split('\n').filter(line => line.toLowerCase().includes(query)).join('\n');
  });

  let lineCount = $derived(logs.split('\n').filter(l => l.length > 0).length);

  onMount(async () => {
    containers = await listContainers(namespace, pod);
    if (containers.length > 0) {
      selectedContainer = containers.find(c => !c.startsWith('init:')) ?? containers[0];
    }
    await fetchLogs();
  });

  onDestroy(() => {
    cleanup?.();
  });

  async function fetchLogs() {
    loading = true;
    error = null;
    streaming = false;
    cleanup?.();
    cleanup = null;

    try {
      const result = await getPodLogs(namespace, pod, selectedContainer ?? undefined, showPrevious, tailLines);
      logs = result;
      await scrollToBottom();

      if (!showPrevious) {
        await startStreaming();
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to fetch logs';
      logs = '';
    } finally {
      loading = false;
    }
  }

  async function startStreaming() {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      streaming = true;

      await startLogStream(namespace, pod, selectedContainer ?? undefined, 0);

      const unlisten = await listen<{ lines: string; is_complete: boolean }>('log-chunk', (event) => {
        if (event.payload.is_complete) {
          streaming = false;
          return;
        }
        logs += event.payload.lines;
        if (autoScroll) {
          tick().then(scrollToBottom);
        }
      });

      cleanup = () => {
        unlisten();
        streaming = false;
      };
    } catch {
      streaming = false;
    }
  }

  async function scrollToBottom() {
    await tick();
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  }

  function handleContainerChange(e: Event) {
    selectedContainer = (e.target as HTMLSelectElement).value;
    fetchLogs();
  }

  function togglePrevious() {
    showPrevious = !showPrevious;
    fetchLogs();
  }

  function handleSearch(e: Event) {
    searchQuery = (e.target as HTMLInputElement).value;
  }
</script>

<div class="log-viewer">
  <div class="log-toolbar">
    {#if containers.length > 1}
      <label>
        <span class="sr-only">Container</span>
        <select value={selectedContainer} onchange={handleContainerChange}>
          {#each containers as c}
            <option value={c}>{c}</option>
          {/each}
        </select>
      </label>
    {:else if containers.length === 1}
      <span class="container-name">{containers[0]}</span>
    {/if}

    <input
      type="text"
      placeholder="Search logs..."
      value={searchQuery}
      oninput={handleSearch}
      class="search-input"
      aria-label="Search logs"
    />

    <label class="toggle-label">
      <input type="checkbox" checked={showPrevious} onchange={togglePrevious} />
      Previous
    </label>

    <label class="toggle-label">
      <input type="checkbox" bind:checked={autoScroll} />
      Auto-scroll
    </label>

    <button onclick={fetchLogs} class="btn" disabled={loading}>
      {loading ? '↻' : '⟳'} Refresh
    </button>

    <span class="meta">
      {lineCount} lines
      {#if streaming}
        <span class="streaming-dot">● streaming</span>
      {/if}
    </span>
  </div>

  {#if error}
    <div class="log-error" role="alert">{error}</div>
  {:else if loading && !logs}
    <div class="log-loading">Loading logs…</div>
  {:else}
    <pre class="log-output" bind:this={logContainer}><code>{filteredLogs}</code></pre>
  {/if}
</div>

<style>
  .log-viewer {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 280px);
    min-height: 300px;
  }
  .log-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px 6px 0 0;
    flex-wrap: wrap;
  }
  select, .search-input {
    background: #0d1117;
    border: 1px solid #30363d;
    color: #e0e0e0;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .search-input {
    flex: 1;
    min-width: 120px;
    max-width: 250px;
  }
  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8rem;
    color: #8b949e;
    cursor: pointer;
    white-space: nowrap;
  }
  .toggle-label input { accent-color: #58a6ff; }
  .btn {
    background: #21262d;
    border: 1px solid #30363d;
    color: #e0e0e0;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .btn:hover { background: #30363d; }
  .btn:disabled { opacity: 0.5; cursor: wait; }
  .meta {
    margin-left: auto;
    color: #484f58;
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .streaming-dot {
    color: #3fb950;
    animation: pulse 1.5s infinite;
  }
  .container-name {
    color: #8b949e;
    font-size: 0.8rem;
    padding: 0.25rem 0.5rem;
    background: #0d1117;
    border-radius: 4px;
  }
  .log-output {
    flex: 1;
    overflow: auto;
    margin: 0;
    padding: 0.75rem;
    background: #0d1117;
    border: 1px solid #21262d;
    border-top: none;
    border-radius: 0 0 6px 6px;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 0.8rem;
    line-height: 1.5;
    color: #c9d1d9;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .log-error {
    padding: 1rem;
    color: #f85149;
    background: #0d1117;
    border: 1px solid #21262d;
    border-top: none;
    border-radius: 0 0 6px 6px;
  }
  .log-loading {
    padding: 2rem;
    text-align: center;
    color: #484f58;
    background: #0d1117;
    border: 1px solid #21262d;
    border-top: none;
    border-radius: 0 0 6px 6px;
  }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0,0,0,0); }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
</style>
