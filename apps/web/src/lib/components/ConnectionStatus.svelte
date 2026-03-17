<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getConnectionState, isTauriDesktop } from '$lib/api';
  import { connectionState } from '$lib/stores';
  import type { ConnectionState } from '$lib/tauri-commands';

  let connState: ConnectionState = $state({ state: 'Disconnected' });
  let cleanup: (() => void) | null = $state(null);

  function updateState(newState: ConnectionState) {
    connState = newState;
    connectionState.set(newState);
  }

  onMount(async () => {
    const initial = await getConnectionState();
    updateState(initial);

    if (!isTauriDesktop()) {
      return;
    }

    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<ConnectionState>('connection-state-changed', (event) => {
      updateState(event.payload);
    });
    cleanup = unlisten;
  });

  onDestroy(() => {
    cleanup?.();
  });

  let label = $derived(getLabel(connState));
  let statusClass = $derived(getStatusClass(connState));
  let tooltip = $derived(getTooltip(connState));

  function getLabel(s: ConnectionState): string {
    switch (s.state) {
      case 'Disconnected': return 'Disconnected';
      case 'Connecting': return 'Connecting…';
      case 'Syncing': {
        const d = s.detail;
        const total = d.resources_total ? `/${d.resources_total}` : '';
        return `Syncing ${d.resources_synced}${total}`;
      }
      case 'Ready': return 'Connected';
      case 'Degraded': return 'Degraded';
      case 'Error': return 'Error';
      case 'Backoff': return `Retry #${s.detail.attempt}`;
    }
  }

  function getStatusClass(s: ConnectionState): string {
    switch (s.state) {
      case 'Ready': return 'ready';
      case 'Connecting':
      case 'Syncing': return 'syncing';
      case 'Degraded': return 'degraded';
      case 'Error': return 'error';
      case 'Backoff': return 'backoff';
      default: return 'disconnected';
    }
  }

  function getTooltip(s: ConnectionState): string {
    switch (s.state) {
      case 'Degraded': return s.detail.message;
      case 'Error': return s.detail.message;
      case 'Backoff': return `Retrying in ${s.detail.wait.secs}s (attempt ${s.detail.attempt})`;
      default: return s.state;
    }
  }
</script>

<div class="connection-status {statusClass}" title={tooltip} role="status" aria-live="polite">
  <span class="dot"></span>
  <span class="label">{label}</span>
</div>

<style>
  .connection-status {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border-radius: 12px;
    cursor: default;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .ready .dot { background: #66bb6a; }
  .ready { background: rgba(102, 187, 106, 0.1); color: #66bb6a; }

  .syncing .dot { background: #42a5f5; animation: pulse 1.5s infinite; }
  .syncing { background: rgba(66, 165, 245, 0.1); color: #42a5f5; }

  .degraded .dot { background: #ffa726; }
  .degraded { background: rgba(255, 167, 38, 0.1); color: #ffa726; }

  .error .dot { background: #ef5350; }
  .error { background: rgba(239, 83, 80, 0.1); color: #ef5350; }

  .backoff .dot { background: #ffa726; animation: pulse 2s infinite; }
  .backoff { background: rgba(255, 167, 38, 0.1); color: #ffa726; }

  .disconnected .dot { background: #757575; }
  .disconnected { background: rgba(117, 117, 117, 0.1); color: #757575; }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
</style>
