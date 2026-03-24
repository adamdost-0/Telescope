<script lang="ts">
  import { IconChevronRight } from '@tabler/icons-svelte';

  interface Props {
    poolName: string;
    nodeCount: number;
    readyCount: number;
    vmSize: string;
    osType: string;
    mode: string;
    collapsed: boolean;
    onToggle: () => void;
  }

  let { poolName, nodeCount, readyCount, vmSize, osType, mode, collapsed, onToggle }: Props = $props();

  let healthClass = $derived(
    readyCount === nodeCount ? 'healthy' : readyCount === 0 ? 'unhealthy' : 'degraded'
  );
</script>

<button type="button" class="pool-header" onclick={onToggle} aria-expanded={!collapsed}>
  <span class="chevron" class:collapsed aria-hidden="true">
    <IconChevronRight size={14} />
  </span>
  <span class="pool-name">{poolName}</span>
  <span class="pool-badges">
    <span class="badge health {healthClass}">{readyCount}/{nodeCount} Ready</span>
    {#if mode}
      <span class="badge mode" class:system={mode === 'System'} class:user={mode === 'User'}>{mode}</span>
    {/if}
    {#if vmSize}
      <span class="badge vm">{vmSize}</span>
    {/if}
    {#if osType}
      <span class="badge os">{osType}</span>
    {/if}
    <span class="badge count">{nodeCount} node{nodeCount !== 1 ? 's' : ''}</span>
  </span>
</button>

<style>
  .pool-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    background: #1a1a2e;
    border: 1px solid #2a2a4a;
    border-radius: 6px;
    padding: 0.625rem 0.75rem;
    cursor: pointer;
    color: #e0e0e0;
    font-size: 0.9375rem;
    transition: background 0.15s;
  }
  .pool-header:hover { background: #22223a; }

  .chevron {
    display: inline-block;
    transition: transform 0.2s;
    font-size: 0.625rem;
    color: #9e9e9e;
  }
  .chevron:not(.collapsed) { transform: rotate(90deg); }

  .pool-name { font-weight: 600; }

  .pool-badges {
    display: flex;
    gap: 0.375rem;
    margin-left: auto;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .badge {
    font-size: 0.6875rem;
    padding: 0.125rem 0.5rem;
    border-radius: 10px;
    white-space: nowrap;
  }

  .health.healthy { background: #1b5e20; color: #a5d6a7; }
  .health.degraded { background: #e65100; color: #ffcc80; }
  .health.unhealthy { background: #b71c1c; color: #ef9a9a; }

  .mode { background: #1a237e; color: #9fa8da; }
  .mode.system { background: #4a148c; color: #ce93d8; }
  .mode.user { background: #1a237e; color: #9fa8da; }

  .vm { background: #263238; color: #b0bec5; }
  .os { background: #263238; color: #b0bec5; }
  .count { background: #1a1a2e; color: #757575; border: 1px solid #2a2a4a; }
</style>
