<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import SearchPalette from '$lib/components/SearchPalette.svelte';

  let { children }: { children: Snippet } = $props();
  let searchOpen = $state(false);

  onMount(() => {
    function handleKeydown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        searchOpen = !searchOpen;
      }
    }
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="app-shell">
  <AppHeader />
  <div class="app-body">
    <Sidebar />
    <main>
      {@render children()}
    </main>
  </div>
</div>

<SearchPalette bind:open={searchOpen} />

<style>
  :global(:root), :global([data-theme="dark"]) {
    --bg-primary: #0f0f23;
    --bg-secondary: #0d1117;
    --bg-tertiary: #161b22;
    --bg-hover: #1f2937;
    --border: #21262d;
    --text-primary: #e0e0e0;
    --text-secondary: #8b949e;
    --text-muted: #484f58;
    --accent: #58a6ff;
    --success: #66bb6a;
    --warning: #ffa726;
    --error: #ef5350;
    --danger-bg: #d32f2f;
  }

  :global([data-theme="light"]) {
    --bg-primary: #ffffff;
    --bg-secondary: #f6f8fa;
    --bg-tertiary: #f0f2f5;
    --bg-hover: #e8eaed;
    --border: #d0d7de;
    --text-primary: #1f2328;
    --text-secondary: #656d76;
    --text-muted: #8c959f;
    --accent: #0969da;
    --success: #1a7f37;
    --warning: #9a6700;
    --error: #cf222e;
    --danger-bg: #cf222e;
  }

  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  :global(*:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
  .app-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  main {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }
</style>
