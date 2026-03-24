<script lang="ts">
  import { getSuggestion } from '$lib/error-suggestions';
  import Icon from '$lib/icons/Icon.svelte';

  let { message = '', suggestion = '', onretry }: {
    message: string;
    suggestion?: string;
    onretry?: () => void;
  } = $props();

  let effectiveSuggestion = $derived(suggestion || getSuggestion(message));
</script>

<div class="error-message" role="alert" aria-live="polite" data-testid="error-message">
  <div class="error-icon" aria-hidden="true">
    <Icon name="error" size={20} />
  </div>
  <div class="error-content">
    <p class="error-text">{message}</p>
    {#if effectiveSuggestion}
      <p class="error-suggestion">
        <Icon name="suggestion" size={16} class="suggestion-icon" aria-hidden="true" />
        <span>{effectiveSuggestion}</span>
      </p>
    {/if}
  </div>
  {#if onretry}
    <button class="retry-btn" onclick={onretry}>Retry</button>
  {/if}
</div>

<style>
  .error-message {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    background: rgba(239, 83, 80, 0.08);
    border: 1px solid rgba(239, 83, 80, 0.25);
    border-radius: 8px;
    margin: 1rem 0;
  }
  .error-icon {
    font-size: 1.25rem;
    flex-shrink: 0;
    line-height: 1.4;
  }
  .error-content {
    flex: 1;
    min-width: 0;
  }
  .error-text {
    color: #ef5350;
    margin: 0;
    font-size: 0.9rem;
    line-height: 1.4;
  }
  .error-suggestion {
    color: #8b949e;
    margin: 0.35rem 0 0;
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .retry-btn {
    background: #1a73e8;
    color: white;
    border: none;
    padding: 0.375rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    white-space: nowrap;
    flex-shrink: 0;
    align-self: center;
  }
  .retry-btn:hover {
    background: #1565c0;
  }
</style>
