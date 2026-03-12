<script lang="ts">
  let theme = $state(
    typeof localStorage !== 'undefined'
      ? (localStorage.getItem('telescope-theme') ?? 'dark')
      : 'dark'
  );

  $effect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('telescope-theme', theme);
  });

  function toggle() {
    theme = theme === 'dark' ? 'light' : 'dark';
  }
</script>

<button onclick={toggle} class="theme-toggle" title="Toggle theme" aria-label="Toggle theme">
  {theme === 'dark' ? '☀️' : '🌙'}
</button>

<style>
  .theme-toggle {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 6px;
    padding: 0.25rem 0.5rem;
    font-size: 1rem;
    line-height: 1;
    transition: background 0.15s, color 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .theme-toggle:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
