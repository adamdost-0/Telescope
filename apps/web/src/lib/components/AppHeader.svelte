<script lang="ts">
  import ConnectionStatus from './ConnectionStatus.svelte';
  import ContextSwitcher from './ContextSwitcher.svelte';
</script>

<header class="app-header">
  <div class="brand">
    <a href="/">🔭 Telescope</a>
  </div>
  <nav>
    <a href="/clusters">Clusters</a>
    <a href="/pods">Pods</a>
    <a href="/explore">Explore</a>
  </nav>
  <div class="context-area">
    <ConnectionStatus />
    <ContextSwitcher />
  </div>
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 1rem;
    background: #0d1117;
    border-bottom: 1px solid #21262d;
    color: #e0e0e0;
  }
  .brand a {
    font-weight: 700;
    font-size: 1.125rem;
    color: #58a6ff;
    text-decoration: none;
  }
  nav {
    display: flex;
    gap: 0.75rem;
  }
  nav a {
    color: #8b949e;
    text-decoration: none;
    font-size: 0.875rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
  }
  nav a:hover {
    color: #e0e0e0;
    background: #161b22;
  }
  .context-area {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
</style>
