<script lang="ts">
  import type { ResourceEntry } from '$lib/tauri-commands';

  interface Column {
    key: string;
    label: string;
    extract: (content: any) => string;
    width?: string;
  }

  let { resources = [], columns = [], emptyMessage = 'No resources found.', hrefFn }: {
    resources: ResourceEntry[];
    columns: Column[];
    emptyMessage?: string;
    hrefFn?: (entry: ResourceEntry) => string | null;
  } = $props();

  let rows = $derived(resources.map((entry) => {
    try {
      const content = JSON.parse(entry.content);
      return columns.map((col) => col.extract(content));
    } catch {
      return columns.map(() => '—');
    }
  }));

  function formatAge(timestamp: string): string {
    const created = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - created.getTime();
    const diffSec = Math.floor(diffMs / 1000);

    if (diffSec < 60) return `${diffSec}s`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h`;
    const diffDays = Math.floor(diffHours / 24);
    return `${diffDays}d`;
  }

  // Re-export for use in column extractors
  export { formatAge };
</script>

{#if rows.length === 0}
  <p class="empty">{emptyMessage}</p>
{:else}
  <div class="table-container">
    <table aria-label="Resource list">
      <thead>
        <tr>
          {#each columns as col}
            <th scope="col" style={col.width ? `width: ${col.width}` : ''}>{col.label}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as row, i (resources[i].name + resources[i].namespace)}
          {@const href = hrefFn?.(resources[i]) ?? null}
          <tr>
            {#each row as cell, j}
              {#if j === 0 && href}
                <td class="resource-name"><a {href}>{cell}</a></td>
              {:else}
                <td class:resource-name={j === 0}>{cell}</td>
              {/if}
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .table-container {
    overflow: auto;
    max-height: 70vh;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-family: monospace;
    font-size: 0.875rem;
  }
  thead {
    position: sticky;
    top: 0;
    background: #1a1a2e;
    color: #e0e0e0;
    z-index: 1;
  }
  th, td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid #2a2a3e;
  }
  tr:hover {
    background: #16213e;
  }
  .resource-name {
    font-weight: 500;
    color: #4fc3f7;
  }
  .resource-name a {
    color: #4fc3f7;
    text-decoration: none;
  }
  .resource-name a:hover {
    text-decoration: underline;
    color: #58a6ff;
  }
  .empty {
    color: #9e9e9e;
    padding: 2rem;
    text-align: center;
  }
</style>
