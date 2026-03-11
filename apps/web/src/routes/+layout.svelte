<script lang="ts">
  import type { Snippet } from 'svelte';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';

  let { children }: { children: Snippet } = $props();
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

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #0f0f23;
    color: #e0e0e0;
  }
  :global(*:focus-visible) {
    outline: 2px solid #58a6ff;
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
