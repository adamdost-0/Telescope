<script lang="ts">
  import { onMount } from 'svelte';
  import { getPreference, setPreference } from '$lib/api';

  let theme = $state<'dark' | 'light'>('dark');

  function systemTheme(): 'dark' | 'light' {
    if (typeof window === 'undefined') {
      return 'dark';
    }

    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  }

  function resolveTheme(value: string | null | undefined): 'dark' | 'light' | null {
    if (value === 'dark' || value === 'light') {
      return value;
    }

    if (value === 'system') {
      return systemTheme();
    }

    return null;
  }

  onMount(() => {
    void (async () => {
      const savedTheme = resolveTheme(await getPreference('theme'));
      const storedTheme = resolveTheme(
        typeof localStorage !== 'undefined' ? localStorage.getItem('telescope-theme') : null
      );
      theme = savedTheme ?? storedTheme ?? systemTheme();
    })();
  });

  $effect(() => {
    if (typeof document === 'undefined') {
      return;
    }

    document.documentElement.setAttribute('data-theme', theme);

    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('telescope-theme', theme);
    }
  });

  async function toggle() {
    const nextTheme = theme === 'dark' ? 'light' : 'dark';
    theme = nextTheme;

    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('telescope-theme', nextTheme);
    }

    await setPreference('theme', nextTheme);
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
