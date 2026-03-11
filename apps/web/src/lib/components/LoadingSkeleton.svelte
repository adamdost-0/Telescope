<script lang="ts">
  let { rows = 5, columns = 4 }: { rows?: number; columns?: number } = $props();
</script>

<div class="skeleton-table" role="status" aria-label="Loading data">
  <div class="skeleton-header">
    {#each Array(columns) as _}
      <div class="skeleton-cell header"></div>
    {/each}
  </div>
  {#each Array(rows) as _}
    <div class="skeleton-row">
      {#each Array(columns) as _, i}
        <div class="skeleton-cell" style="width: {i === 0 ? '40%' : '20%'}"></div>
      {/each}
    </div>
  {/each}
</div>

<style>
  .skeleton-table { width: 100%; }
  .skeleton-header, .skeleton-row {
    display: flex;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #2a2a3e;
  }
  .skeleton-cell {
    height: 1rem;
    background: linear-gradient(90deg, #1a1a2e 25%, #252540 50%, #1a1a2e 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
    border-radius: 4px;
  }
  .skeleton-cell.header {
    height: 0.875rem;
    opacity: 0.6;
  }
  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
</style>
