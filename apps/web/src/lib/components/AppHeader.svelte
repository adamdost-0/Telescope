<script lang="ts">
  import ContextSwitcher from './ContextSwitcher.svelte';
  import ConnectionStatus from './ConnectionStatus.svelte';
  import { isProduction } from '$lib/stores';
</script>

{#if $isProduction}
  <div class="prod-banner" role="status">⚠️ PRODUCTION</div>
{/if}

<header class="app-header">
  <div class="brand">
    <span class="brand-icon">🔭</span>
    <span class="brand-text">Telescope</span>
  </div>
  <div class="spacer"></div>
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
    background: #010409;
    border-bottom: 1px solid #21262d;
    color: #e0e0e0;
    height: 48px;
    z-index: 10;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .brand-icon { font-size: 1.25rem; }
  .brand-text {
    font-weight: 700;
    font-size: 1rem;
    color: #58a6ff;
  }
  .spacer { flex: 1; }
  .context-area {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .prod-banner {
    background: #d32f2f;
    color: #fff;
    text-align: center;
    font-weight: 700;
    font-size: 0.8rem;
    padding: 0.25rem 0;
    letter-spacing: 0.05em;
    width: 100%;
  }
</style>
