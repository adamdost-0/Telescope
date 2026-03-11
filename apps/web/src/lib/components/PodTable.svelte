<script lang="ts">
  import type { ResourceEntry } from '$lib/tauri-commands';

  interface PodInfo {
    name: string;
    namespace: string;
    status: string;
    restarts: number;
    age: string;
    ready: string;
  }

  let { pods = [] }: { pods: ResourceEntry[] } = $props();

  let parsedPods = $derived(pods.map(parsePod));

  function parsePod(entry: ResourceEntry): PodInfo {
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
      };
    } catch {
      return {
        name: entry.name,
        namespace: entry.namespace,
        status: 'Unknown',
        restarts: 0,
        age: 'Unknown',
        ready: '0/0',
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
          <th scope="col">Name</th>
          <th scope="col">Ready</th>
          <th scope="col">Status</th>
          <th scope="col">Restarts</th>
          <th scope="col">Age</th>
        </tr>
      </thead>
      <tbody>
        {#each parsedPods as pod (pod.name)}
          <tr>
            <td class="pod-name"><a href="/pods/{pod.namespace}/{pod.name}">{pod.name}</a></td>
            <td>{pod.ready}</td>
            <td><span class={statusClass(pod.status)}>{pod.status}</span></td>
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
    background: #1a1a2e;
    color: #e0e0e0;
  }
  th, td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid #2a2a3e;
  }
  tr:hover {
    background: #16213e;
  }
  .pod-name a {
    font-weight: 500;
    color: #4fc3f7;
    text-decoration: none;
  }
  .pod-name a:hover {
    text-decoration: underline;
    color: #58a6ff;
  }
  .status-running { color: #66bb6a; }
  .status-succeeded { color: #42a5f5; }
  .status-pending { color: #ffa726; }
  .status-failed { color: #ef5350; }
  .status-unknown { color: #bdbdbd; }
  .empty {
    color: #9e9e9e;
    padding: 2rem;
    text-align: center;
  }
</style>
