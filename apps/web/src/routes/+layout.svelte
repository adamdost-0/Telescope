<script lang="ts">
  import type { Snippet } from 'svelte';
  import { goto } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import SearchPalette from '$lib/components/SearchPalette.svelte';
  import ShortcutHelp from '$lib/components/ShortcutHelp.svelte';

  let { children }: { children: Snippet } = $props();
  let searchOpen = $state(false);
  let showHelp = $state(false);
  let gPrefix = $state(false);
  let gTimer: ReturnType<typeof setTimeout> | undefined;

  function handleGlobalKeydown(e: KeyboardEvent) {
    // Ctrl/Cmd+K: search palette
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      searchOpen = !searchOpen;
      return;
    }

    // Skip remaining shortcuts when typing in form elements
    const tag = (e.target as HTMLElement)?.tagName;
    if (['INPUT', 'TEXTAREA', 'SELECT'].includes(tag)) return;

    if (e.key === '?' && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      showHelp = !showHelp;
      return;
    }

    if (e.key === 'Escape') {
      if (showHelp) {
        showHelp = false;
      }
      // search palette handles its own Escape
      return;
    }

    // "g" prefix navigation
    if (e.key === 'g' && !e.ctrlKey && !e.metaKey && !gPrefix) {
      gPrefix = true;
      clearTimeout(gTimer);
      gTimer = setTimeout(() => { gPrefix = false; }, 1000);
      return;
    }

    if (gPrefix) {
      gPrefix = false;
      clearTimeout(gTimer);
      const routes: Record<string, string> = {
        'o': '/overview',
        'i': '/insights',
        'p': '/pods',
        'd': '/resources/deployments',
        's': '/resources/services',
        'n': '/nodes',
        'e': '/events',
        'h': '/helm',
        'c': '/crds',
        't': '/settings',
      };
      if (routes[e.key]) {
        e.preventDefault();
        goto(routes[e.key]);
      }
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="app-shell">
  <AppHeader onhelp={() => { showHelp = true; }} />
  <div class="app-body">
    <Sidebar />
    <main>
      {@render children()}
    </main>
  </div>
</div>

<SearchPalette bind:open={searchOpen} />
<ShortcutHelp open={showHelp} onclose={() => { showHelp = false; }} />

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
