<script lang="ts">
  let { query = '', onfilter }: { query?: string; onfilter?: (q: string) => void } = $props();
  let input = $state(query);

  function handleInput(e: Event) {
    input = (e.target as HTMLInputElement).value;
    onfilter?.(input);
  }
  function clear() { input = ''; onfilter?.(''); }
</script>

<div class="filter-bar">
  <span class="search-icon">🔍</span>
  <input type="text" value={input} oninput={handleInput} placeholder="Filter by name..." aria-label="Filter resources" />
  {#if input}<button type="button" onclick={clear} class="clear-btn" aria-label="Clear filter">✕</button>{/if}
</div>

<style>
  .filter-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.375rem 0.75rem;
    margin-bottom: 0.75rem;
    max-width: 400px;
  }
  .filter-bar:focus-within {
    border-color: #58a6ff;
    box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.15);
  }
  .search-icon {
    font-size: 0.8rem;
    color: #6e7681;
    flex-shrink: 0;
  }
  input {
    all: unset;
    flex: 1;
    color: #e0e0e0;
    font-size: 0.875rem;
    min-width: 0;
  }
  input::placeholder {
    color: #484f58;
  }
  .clear-btn {
    all: unset;
    cursor: pointer;
    color: #6e7681;
    font-size: 0.75rem;
    padding: 0.125rem 0.25rem;
    border-radius: 3px;
    flex-shrink: 0;
    line-height: 1;
  }
  .clear-btn:hover {
    color: #e0e0e0;
    background: #30363d;
  }
</style>
