<script lang="ts">
  import ContextSwitcher from './ContextSwitcher.svelte';
  import ConnectionStatus from './ConnectionStatus.svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import ClusterVitals from './ClusterVitals.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { IconTelescope } from '@tabler/icons-svelte';
  import { isProduction, isConnected } from '$lib/stores';

  let { onhelp }: { onhelp?: () => void } = $props();
</script>

{#if $isProduction}
  <div class="prod-banner" role="status" data-testid="prod-banner">
    <Icon name="prod-warning" size={16} aria-hidden="true" />
    <span class="prod-banner-text">PRODUCTION</span>
  </div>
{/if}

<header class="app-header">
  <div class="brand" aria-label="Telescope">
    <span class="brand-icon" aria-hidden="true">
      <IconTelescope />
    </span>
    <h1 class="brand-text" role="heading" aria-level="1" data-testid="app-brand">Telescope</h1>
  </div>
  <div class="spacer"></div>
  <ClusterVitals visible={$isConnected} />
  <div class="context-area">
    <ConnectionStatus />
    <ContextSwitcher />
    <ThemeToggle />
    <button class="help-btn" onclick={() => onhelp?.()} title="Keyboard shortcuts (?)">?</button>
  </div>
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 1rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
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
    color: var(--accent);
    margin: 0;
  }
  .spacer { flex: 1; }
  .context-area {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .prod-banner {
    background: var(--danger-bg);
    color: #fff;
    text-align: center;
    font-weight: 700;
    font-size: 0.8rem;
    padding: 0.25rem 0;
    letter-spacing: 0.05em;
    width: 100%;
  }
  .help-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    width: 28px;
    height: 28px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: inherit;
  }
  .help-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
