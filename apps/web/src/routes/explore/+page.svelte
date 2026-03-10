<script lang="ts">
  import ContextBanner from '$lib/components/ContextBanner.svelte';
  import type { Cluster, Kind, Namespace, ResourceRow } from '$lib/engine';

  export let data: {
    clusterId: string;
    namespace: string;
    kind: string;
    clusters: Cluster[];
    kinds: Kind[];
    namespaces: Namespace[];
    items: ResourceRow[];
  };

  const selectKind = (k: string) => {
    const u = new URL(window.location.href);
    u.searchParams.set('kind', k);
    window.location.assign(u);
  };
</script>

<h1>Explore</h1>

<ContextBanner
  clusterId={data.clusterId}
  clusters={data.clusters}
  namespaces={data.namespaces}
  namespace={data.namespace}
/>

<div class="layout">
  <aside class="sidebar">
    <h2>Kinds</h2>
    <ul>
      {#each data.kinds as k}
        <li>
          <button
            class:selected={k.kind === data.kind}
            data-testid={`kind-${k.kind}`}
            on:click={() => selectKind(k.kind)}
          >
            {k.kind}
            {#if !k.namespaced}
              <span class="pill">cluster</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  </aside>

  <main class="main">
    <div class="toolbar">
      <div class="title">{data.kind}</div>
      <div class="meta" data-testid="selected-cluster">{data.clusterId}</div>
    </div>

    {#if data.items.length === 0}
      <p class="empty">No resources found.</p>
    {:else}
      <table class="table" data-testid="resource-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Namespace</th>
            <th>Status</th>
            <th>Age</th>
          </tr>
        </thead>
        <tbody>
          {#each data.items as r}
            <tr>
              <td>{r.name}</td>
              <td>{r.namespace ?? '-'}</td>
              <td>{r.status ?? '-'}</td>
              <td>{r.age ?? '-'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </main>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 12px;
    margin-top: 12px;
  }
  .sidebar {
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    padding: 10px;
    background: #0e0e0e;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 6px;
  }
  button {
    width: 100%;
    text-align: left;
    padding: 8px;
    border-radius: 8px;
    border: 1px solid #2a2a2a;
    background: #111;
    color: #eee;
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
  }
  button.selected {
    border-color: #5b8cff;
  }
  .pill {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 999px;
    border: 1px solid #333;
    color: #bbb;
  }
  .main {
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    padding: 10px;
    background: #0e0e0e;
    overflow: hidden;
  }
  .toolbar {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .title {
    font-weight: 700;
  }
  .meta {
    font-size: 12px;
    color: #999;
  }
  .table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    border-bottom: 1px solid #222;
    padding: 8px;
  }
  th {
    text-align: left;
    color: #bbb;
    font-weight: 600;
  }
  .empty {
    color: #aaa;
  }
</style>
