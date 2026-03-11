<script lang="ts">
  import type { Snippet } from 'svelte';
  import AppHeader from '$lib/components/AppHeader.svelte';

  let { children }: { children: Snippet } = $props();
</script>

<AppHeader />
<main>
  {@render children()}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #0f0f23;
    color: #e0e0e0;
  }
  main {
    padding: 1rem;
  }
</style>
