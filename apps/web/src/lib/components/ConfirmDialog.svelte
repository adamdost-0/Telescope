<script lang="ts">
  let {
    open = false,
    title = 'Confirm',
    message = 'Are you sure?',
    confirmText = 'Delete',
    confirmValue = '',
    requireType = false,
    onconfirm,
    oncancel,
  }: {
    open: boolean;
    title?: string;
    message?: string;
    confirmText?: string;
    confirmValue?: string;
    requireType?: boolean;
    onconfirm?: () => void;
    oncancel?: () => void;
  } = $props();

  let typed = $state('');
  let canConfirm = $derived(!requireType || typed === confirmValue);

  $effect(() => {
    if (open) typed = '';
  });
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={oncancel}>
    <div
      class="dialog"
      role="alertdialog"
      aria-label={title}
      onclick={(e) => e.stopPropagation()}
    >
      <h3>{title}</h3>
      <p class="message">{message}</p>
      {#if requireType}
        <p class="type-hint">
          Type <strong>{confirmValue}</strong> to confirm:
        </p>
        <input
          type="text"
          bind:value={typed}
          placeholder={confirmValue}
          autofocus
        />
      {/if}
      <div class="actions">
        <button class="cancel" onclick={oncancel}>Cancel</button>
        <button class="confirm" onclick={onconfirm} disabled={!canConfirm}>
          {confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--color-surface, #1e1e2e);
    border: 1px solid var(--color-border, #45475a);
    border-radius: 12px;
    padding: 1.5rem;
    min-width: 360px;
    max-width: 480px;
    color: var(--color-text, #cdd6f4);
  }

  h3 {
    margin: 0 0 0.75rem;
    font-size: 1.1rem;
    color: #ef5350;
  }

  .message {
    margin: 0 0 1rem;
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--color-text-muted, #a6adc8);
  }

  .type-hint {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    color: var(--color-text-muted, #a6adc8);
  }

  .type-hint strong {
    color: #ef5350;
    font-family: monospace;
  }

  input[type='text'] {
    width: 100%;
    box-sizing: border-box;
    padding: 0.5rem 0.75rem;
    border-radius: 8px;
    border: 1px solid var(--color-border, #45475a);
    background: var(--color-surface-hover, #313244);
    color: var(--color-text, #cdd6f4);
    font-size: 0.9rem;
    font-family: monospace;
    margin-bottom: 1rem;
  }

  input[type='text']:focus {
    outline: none;
    border-color: #ef5350;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  .cancel,
  .confirm {
    padding: 0.4rem 1rem;
    border-radius: 8px;
    border: 1px solid var(--color-border, #45475a);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .cancel {
    background: transparent;
    color: var(--color-text-muted, #a6adc8);
  }

  .cancel:hover {
    background: var(--color-surface-hover, #313244);
  }

  .confirm {
    background: #ef5350;
    color: #fff;
    border-color: #ef5350;
  }

  .confirm:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .confirm:not(:disabled):hover {
    filter: brightness(1.15);
  }
</style>
