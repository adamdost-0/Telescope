<script lang="ts">
  import type { ResourceEntry, PodMetrics } from '$lib/tauri-commands';

  interface PodInfo {
    name: string;
    namespace: string;
    status: string;
    restarts: number;
    age: string;
    ready: string;
    cpuMillicores: number | null;
    memoryBytes: number | null;
  }

  let { pods = [], metrics = [] }: { pods: ResourceEntry[]; metrics: PodMetrics[] } = $props();

  let metricsMap = $derived(
    new Map(metrics.map((m) => [`${m.namespace}/${m.name}`, m]))
  );

  type SortField = 'name' | 'ready' | 'status' | 'cpu' | 'memory' | 'restarts' | 'age';
  let sortKey = $state<SortField | null>(null);
  let sortDir = $state<'asc' | 'desc'>('asc');

  function toggleSort(key: SortField) {
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

  function sortIndicator(key: SortField): string {
    if (sortKey !== key) return ' ◇';
    return sortDir === 'asc' ? ' ▲' : ' ▼';
  }

  function sortValue(pod: PodInfo, key: SortField): string | number {
    switch (key) {
      case 'name': return pod.name;
      case 'ready': return pod.ready;
      case 'status': return pod.status;
      case 'cpu': return pod.cpuMillicores ?? -1;
      case 'memory': return pod.memoryBytes ?? -1;
      case 'restarts': return pod.restarts;
      case 'age': return pod.age;
    }
  }

  let parsedPods = $derived.by(() => {
    const base = pods.map((p) => parsePod(p));
    if (sortKey === null) return base;
    return [...base].sort((a, b) => {
      const aVal = sortValue(a, sortKey!);
      const bVal = sortValue(b, sortKey!);
      let cmp: number;
      if (typeof aVal === 'number' && typeof bVal === 'number') {
        cmp = aVal - bVal;
      } else {
        cmp = String(aVal).localeCompare(String(bVal), undefined, { numeric: true, sensitivity: 'base' });
      }
      return sortDir === 'asc' ? cmp : -cmp;
    });
  });

  function parsePod(entry: ResourceEntry): PodInfo {
    const m = metricsMap.get(`${entry.namespace}/${entry.name}`);
    try {
      const obj = JSON.parse(entry.content);
      const status = obj?.status?.phase ?? 'Unknown';
      const containers = obj?.status?.containerStatuses ?? [];
      const restarts = containers.reduce((sum: number, c: any) => sum + (c.restartCount ?? 0), 0);
      const readyCount = containers.filter((c: any) => c.ready).length;
      const totalCount = containers.length;
      const createdAt = obj?.metadata?.creationTimestamp;
      const age = createdAt ? formatAge(createdAt) : 'Unknown';

      return {
        name: entry.name,
        namespace: entry.namespace,
        status,
        restarts,
        age,
        ready: `${readyCount}/${totalCount}`,
        cpuMillicores: m?.cpu_millicores ?? null,
        memoryBytes: m?.memory_bytes ?? null,
      };
    } catch {
      return {
        name: entry.name,
        namespace: entry.namespace,
        status: 'Unknown',
        restarts: 0,
        age: 'Unknown',
        ready: '0/0',
        cpuMillicores: m?.cpu_millicores ?? null,
        memoryBytes: m?.memory_bytes ?? null,
      };
    }
  }

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

  function formatCpu(millicores: number | null): string {
    if (millicores === null) return '—';
    if (millicores < 1000) return `${millicores}m`;
    return `${(millicores / 1000).toFixed(1)}`;
  }

  function formatMemory(bytes: number | null): string {
    if (bytes === null) return '—';
    const mi = bytes / (1024 * 1024);
    if (mi < 1024) return `${Math.round(mi)}Mi`;
    return `${(mi / 1024).toFixed(1)}Gi`;
  }

  function statusClass(status: string): string {
    switch (status) {
      case 'Running': return 'status-running';
      case 'Succeeded': return 'status-succeeded';
      case 'Pending': return 'status-pending';
      case 'Failed': return 'status-failed';
      default: return 'status-unknown';
    }
  }
</script>

{#if parsedPods.length === 0}
  <p class="empty">No pods found in this namespace.</p>
{:else}
  <div class="table-container">
    <table aria-label="Pod list">
      <thead>
        <tr>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('name')} aria-label="Sort by Name">Name{sortIndicator('name')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('ready')} aria-label="Sort by Ready">Ready{sortIndicator('ready')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('status')} aria-label="Sort by Status">Status{sortIndicator('status')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('cpu')} aria-label="Sort by CPU">CPU{sortIndicator('cpu')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('memory')} aria-label="Sort by Memory">Memory{sortIndicator('memory')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('restarts')} aria-label="Sort by Restarts">Restarts{sortIndicator('restarts')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('age')} aria-label="Sort by Age">Age{sortIndicator('age')}</button></th>
        </tr>
      </thead>
      <tbody>
        {#each parsedPods as pod (pod.name)}
          <tr>
            <td class="pod-name"><a href="/pods/{pod.namespace}/{pod.name}">{pod.name}</a></td>
            <td>{pod.ready}</td>
            <td><span class={statusClass(pod.status)}>{pod.status}</span></td>
            <td class="metric">{formatCpu(pod.cpuMillicores)}</td>
            <td class="metric">{formatMemory(pod.memoryBytes)}</td>
            <td>{pod.restarts}</td>
            <td>{pod.age}</td>
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
  }
  th, td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  tr:hover {
    background: var(--bg-hover);
  }
  .pod-name a {
    font-weight: 500;
    color: var(--accent);
    text-decoration: none;
  }
  .pod-name a:hover {
    text-decoration: underline;
    color: var(--accent);
  }
  .status-running { color: var(--success); }
  .status-succeeded { color: var(--accent); }
  .status-pending { color: var(--warning); }
  .status-failed { color: var(--error); }
  .status-unknown { color: var(--text-secondary); }
  .empty {
    color: var(--text-muted);
    padding: 2rem;
    text-align: center;
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
  .metric {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
</style>
