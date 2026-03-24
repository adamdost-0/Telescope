<script lang="ts">
  let { open = false, onclose }: { open: boolean; onclose?: () => void } = $props();
  let closeButton: HTMLButtonElement | undefined = $state();

  const shortcuts = [
    { keys: 'Ctrl+K', action: 'Search resources' },
    { keys: '?', action: 'Show this help' },
    { keys: 'g → o', action: 'Go to Overview' },
    { keys: 'g → p', action: 'Go to Pods' },
    { keys: 'g → d', action: 'Go to Deployments' },
    { keys: 'g → s', action: 'Go to Services' },
    { keys: 'g → n', action: 'Go to Nodes' },
    { keys: 'g → e', action: 'Go to Events' },
    { keys: 'g → h', action: 'Go to Helm' },
    { keys: 'g → c', action: 'Go to CRDs' },
    { keys: 'g → t', action: 'Go to Settings' },
    { keys: 'Escape', action: 'Close dialog/overlay' },
  ];

  function handleOverlayClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onclose?.();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose?.();
    }
  }

  $effect(() => {
    if (open && closeButton) {
      closeButton.focus();
    }
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" role="presentation" onclick={handleOverlayClick} onkeydown={handleKeydown}>
    <div class="modal" role="dialog" aria-modal="true" aria-labelledby="shortcut-help-title" tabindex="-1">
      <div class="modal-header" data-testid="shortcut-help">
        <h2 id="shortcut-help-title">Keyboard Shortcuts</h2>
        <button
          bind:this={closeButton}
          class="close-btn"
          type="button"
          aria-label="Close keyboard shortcuts"
          onclick={() => onclose?.()}
        >
          ✕
        </button>
      </div>
      <div class="shortcuts-grid">
        {#each shortcuts as shortcut}
          <div class="shortcut-row">
            <kbd class="shortcut-keys">{shortcut.keys}</kbd>
            <span class="shortcut-action">{shortcut.action}</span>
          </div>
        {/each}
      </div>
      <div class="modal-footer">
        Press <kbd>?</kbd> or <kbd>Escape</kbd> to close
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 100;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .modal {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: min(480px, 90vw);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }
  .modal-header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 1rem;
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 4px;
  }
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .shortcuts-grid {
    padding: 0.75rem 1.25rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.4rem 0.5rem;
    border-radius: 6px;
  }
  .shortcut-row:hover {
    background: var(--bg-hover);
  }
  .shortcut-keys {
    background: var(--bg-secondary);
    color: var(--text-secondary);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8rem;
    font-family: 'SF Mono', 'Fira Code', monospace;
    border: 1px solid var(--border);
    min-width: 70px;
    text-align: center;
  }
  .shortcut-action {
    color: var(--text-primary);
    font-size: 0.875rem;
    flex: 1;
    text-align: right;
  }
  .modal-footer {
    padding: 0.75rem 1.25rem;
    border-top: 1px solid var(--border);
    text-align: center;
    color: var(--text-muted);
    font-size: 0.75rem;
  }
  .modal-footer kbd {
    background: var(--bg-secondary);
    color: var(--text-secondary);
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    font-size: 0.7rem;
    border: 1px solid var(--border);
    font-family: inherit;
  }
</style>
