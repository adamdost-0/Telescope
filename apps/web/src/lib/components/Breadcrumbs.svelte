<script lang="ts">
  interface Crumb { label: string; href?: string; }
  let { crumbs = [] }: { crumbs: Crumb[] } = $props();
</script>

<nav aria-label="Breadcrumb" class="breadcrumbs">
  <ol>
    {#each crumbs as crumb, i}
      <li>
        {#if crumb.href && i < crumbs.length - 1}
          <a href={crumb.href}>{crumb.label}</a>
        {:else}
          <span class="current">{crumb.label}</span>
        {/if}
        {#if i < crumbs.length - 1}
          <span class="separator">›</span>
        {/if}
      </li>
    {/each}
  </ol>
</nav>

<style>
  .breadcrumbs { margin-bottom: 1rem; }
  ol { display: flex; list-style: none; padding: 0; margin: 0; gap: 0.25rem; align-items: center; }
  a { color: var(--accent, #58a6ff); text-decoration: none; font-size: 0.85rem; }
  a:hover { text-decoration: underline; }
  .current { color: var(--text-secondary, #8b949e); font-size: 0.85rem; }
  .separator { color: var(--text-muted, #484f58); font-size: 0.85rem; }
</style>
