<script lang="ts">
  import { isConnected } from '$lib/stores';
</script>

<div class="resource-page">
  <header>
    <h1>CronJobs</h1>
  </header>

  {#if !$isConnected}
    <div class="not-connected">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else}
    <div class="coming-soon">
      <p class="icon">🕐</p>
      <h2>Not Yet Watched</h2>
      <p class="detail">CronJobs are not currently synced by the resource watcher.</p>
      <p class="hint">This resource kind will be available in a future milestone.</p>
    </div>
  {/if}
</div>

<style>
  .resource-page { padding: 1rem; color: #e0e0e0; background: #0f0f23; min-height: 100vh; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  h1 { font-size: 1.5rem; margin: 0; }
  .not-connected, .coming-soon { text-align: center; padding: 3rem 1rem; color: #757575; }
  .not-connected p, .coming-soon p { margin: 0.25rem 0; font-size: 1.125rem; }
  .hint { font-size: 0.875rem; color: #616161; }
  .coming-soon .icon { font-size: 3rem; margin-bottom: 0.5rem; }
  .coming-soon h2 { color: #9e9e9e; font-size: 1.25rem; margin: 0.5rem 0; }
  .coming-soon .detail { color: #8b949e; font-size: 0.95rem; }
</style>
