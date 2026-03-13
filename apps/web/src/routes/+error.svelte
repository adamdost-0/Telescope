<script lang="ts">
  import { page } from '$app/stores';

  function errorMessage(error: unknown): string {
    if (error instanceof Error && error.message) {
      return error.message;
    }

    if (typeof error === 'object' && error !== null && 'message' in error) {
      const message = (error as { message?: unknown }).message;
      if (typeof message === 'string' && message.length > 0) {
        return message;
      }
    }

    return 'Something went wrong while loading this page.';
  }

  function reloadPage() {
    window.location.reload();
  }
</script>

<svelte:head>
  <title>{$page.status} · Telescope</title>
</svelte:head>

<div class="error-shell">
  <section class="error-card" aria-labelledby="error-title">
    <p class="status">{$page.status}</p>
    <h1 id="error-title">Something went wrong</h1>
    <p class="message">{errorMessage($page.error)}</p>

    <div class="actions">
      <a class="primary" href="/">Go Home</a>
      <button type="button" class="secondary" onclick={reloadPage}>Try Again</button>
    </div>
  </section>
</div>

<style>
  .error-shell {
    min-height: calc(100vh - 2rem);
    display: grid;
    place-items: center;
    padding: 2rem;
    background: var(--bg-primary, #0f0f23);
    color: var(--text-primary, #e0e0e0);
  }

  .error-card {
    width: min(100%, 34rem);
    padding: 2rem;
    border-radius: 16px;
    background: var(--bg-tertiary, #161b22);
    border: 1px solid var(--border, #21262d);
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.35);
  }

  .status {
    margin: 0 0 0.5rem;
    color: var(--accent, #58a6ff);
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.08em;
  }

  h1 {
    margin: 0;
    font-size: 1.75rem;
  }

  .message {
    margin: 0.75rem 0 0;
    color: var(--text-secondary, #8b949e);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    margin-top: 1.5rem;
  }

  .actions :is(a, button) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 8.5rem;
    padding: 0.7rem 1rem;
    border-radius: 8px;
    border: 1px solid var(--border, #21262d);
    font: inherit;
    text-decoration: none;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .primary {
    background: var(--accent, #58a6ff);
    border-color: var(--accent, #58a6ff);
    color: #ffffff;
  }

  .secondary {
    background: transparent;
    color: var(--text-primary, #e0e0e0);
  }

  .actions :is(a, button):hover {
    background: var(--bg-hover, #1f2937);
  }

  .primary:hover {
    opacity: 0.92;
  }
</style>
