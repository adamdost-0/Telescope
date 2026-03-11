<script lang="ts">
  let {
    open = false,
    resourceName = '',
    currentReplicas = 1,
    onscale,
    oncancel
  }: {
    open: boolean;
    resourceName?: string;
    currentReplicas?: number;
    onscale?: (replicas: number) => void;
    oncancel?: () => void;
  } = $props();

  let replicas = $state(currentReplicas);

  $effect(() => {
    replicas = currentReplicas;
  });
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={oncancel}>
    <div
      class="dialog"
      role="dialog"
      aria-label="Scale resource"
      onclick={(e) => e.stopPropagation()}
    >
      <h3>Scale {resourceName}</h3>
      <div class="scale-controls">
        <button class="stepper" onclick={() => (replicas = Math.max(0, replicas - 1))}>−</button>
        <input type="number" bind:value={replicas} min="0" max="100" />
        <button class="stepper" onclick={() => replicas++}>+</button>
      </div>
      <p class="hint">{currentReplicas} → {replicas} replicas</p>
      <div class="actions">
        <button class="cancel" onclick={oncancel}>Cancel</button>
        <button
          class="confirm"
          onclick={() => onscale?.(replicas)}
          disabled={replicas === currentReplicas}
        >
          Scale
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
    min-width: 320px;
    color: var(--color-text, #cdd6f4);
  }

  h3 {
    margin: 0 0 1rem;
    font-size: 1.1rem;
  }

  .scale-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-content: center;
    margin-bottom: 0.75rem;
  }

  .stepper {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    border: 1px solid var(--color-border, #45475a);
    background: var(--color-surface-hover, #313244);
    color: var(--color-text, #cdd6f4);
    font-size: 1.2rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .stepper:hover {
    background: var(--color-accent, #89b4fa);
    color: var(--color-surface, #1e1e2e);
  }

  input[type='number'] {
    width: 72px;
    text-align: center;
    padding: 0.4rem;
    border-radius: 8px;
    border: 1px solid var(--color-border, #45475a);
    background: var(--color-surface-hover, #313244);
    color: var(--color-text, #cdd6f4);
    font-size: 1.1rem;
    -moz-appearance: textfield;
    appearance: textfield;
  }

  input[type='number']::-webkit-inner-spin-button,
  input[type='number']::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .hint {
    text-align: center;
    font-size: 0.85rem;
    color: var(--color-text-muted, #a6adc8);
    margin: 0 0 1rem;
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
    background: var(--color-accent, #89b4fa);
    color: var(--color-surface, #1e1e2e);
    border-color: var(--color-accent, #89b4fa);
  }

  .confirm:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .confirm:not(:disabled):hover {
    filter: brightness(1.1);
  }
</style>
