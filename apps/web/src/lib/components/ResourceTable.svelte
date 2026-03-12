<script lang="ts">
  import type { ResourceEntry } from '$lib/tauri-commands';

  interface Column {
    key: string;
    label: string;
    extract: (content: any) => string;
    width?: string;
    colorFn?: (content: any) => string | null;
  }

  let { resources = [], columns = [], emptyMessage = 'No resources found.', hrefFn }: {
    resources: ResourceEntry[];
    columns: Column[];
    emptyMessage?: string;
    hrefFn?: (entry: ResourceEntry) => string | null;
  } = $props();

  let sortKey = $state<string | null>(null);
  let sortDir = $state<'asc' | 'desc'>('asc');

  function toggleSort(key: string) {
    if (sortKey !== key) {
      sortKey = key;
      sortDir = 'asc';
    } else if (sortDir === 'asc') {
      sortDir = 'desc';
    } else {
      sortKey = null;
      sortDir = 'asc';
    }
  }

  function sortIndicator(key: string): string {
    if (sortKey !== key) return ' ◇';
    return sortDir === 'asc' ? ' ▲' : ' ▼';
  }

  interface RowEntry {
    index: number;
    cells: { text: string; color: string | null }[];
  }

  let rows = $derived.by(() => {
    const mapped: RowEntry[] = resources.map((entry, index) => {
      try {
        const content = JSON.parse(entry.content);
        return {
          index,
          cells: columns.map((col) => ({
            text: col.extract(content),
            color: col.colorFn?.(content) ?? null,
          })),
        };
      } catch {
        return {
          index,
          cells: columns.map(() => ({ text: '—', color: null })),
        };
      }
    });

    if (sortKey === null) return mapped;

    const colIdx = columns.findIndex((c) => c.key === sortKey);
    if (colIdx < 0) return mapped;

    return [...mapped].sort((a, b) => {
      const aVal = a.cells[colIdx].text;
      const bVal = b.cells[colIdx].text;
      const cmp = aVal.localeCompare(bVal, undefined, { numeric: true, sensitivity: 'base' });
      return sortDir === 'asc' ? cmp : -cmp;
    });
  });

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
            <th scope="col" style={col.width ? `width: ${col.width}` : ''}>
              <button class="sort-btn" onclick={() => toggleSort(col.key)} aria-label="Sort by {col.label}">
                {col.label}{sortIndicator(col.key)}
              </button>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as row (resources[row.index].name + resources[row.index].namespace)}
          {@const href = hrefFn?.(resources[row.index]) ?? null}
          <tr>
            {#each row.cells as cell, j}
              {#if j === 0 && href}
                <td class="resource-name"><a {href}>{cell.text}</a></td>
              {:else}
                <td class:resource-name={j === 0} style={cell.color ? `color: ${cell.color}` : ''}>{cell.text}</td>
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
    background: var(--bg-tertiary);
    color: var(--text-primary);
    z-index: 1;
  }
  th, td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  tr:hover {
    background: var(--bg-hover);
  }
  .resource-name {
    font-weight: 500;
    color: var(--accent);
  }
  .resource-name a {
    color: var(--accent);
    text-decoration: none;
  }
  .resource-name a:hover {
    text-decoration: underline;
    color: var(--accent);
  }
  .sort-btn {
    all: unset;
    cursor: pointer;
    color: inherit;
    font: inherit;
    width: 100%;
    display: inline-flex;
    align-items: center;
    white-space: nowrap;
    user-select: none;
  }
  .sort-btn:hover {
    color: var(--accent);
  }
  .empty {
    color: var(--text-muted);
    padding: 2rem;
    text-align: center;
  }
</style>
