<script lang="ts">
  import type { Cluster, Namespace } from '$lib/engine';

  export let clusterId: string;
  export let clusters: Cluster[];
  export let namespaces: Namespace[];
  export let namespace: string;

  const selected = () => clusters.find((c) => c.id === clusterId);
</script>

<div class="banner" data-testid="context-banner">
  <div class="left">
    <span class="label">Cluster</span>
    <span class="value" data-testid="cluster-name">{selected()?.name ?? clusterId}</span>
    <span class="muted">({clusterId})</span>
  </div>

  <div class="right">
    <label class="ns">
      <span class="label">Namespace</span>
      <select
        data-testid="namespace-select"
        bind:value={namespace}
        on:change={(e) => {
          const ns = (e.currentTarget as HTMLSelectElement).value;
          const u = new URL(window.location.href);
          u.searchParams.set('namespace', ns);
          window.history.replaceState({}, '', u);
          window.dispatchEvent(new PopStateEvent('popstate'));
        }}
      >
        {#each namespaces as ns}
          <option value={ns.name}>{ns.name}</option>
        {/each}
      </select>
    </label>
  </div>
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 12px;
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    background: #111;
  }
  .left,
  .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .label {
    font-size: 12px;
    color: #aaa;
  }
  .value {
    font-weight: 600;
  }
  .muted {
    font-size: 12px;
    color: #888;
  }
  select {
    background: #0b0b0b;
    color: #eee;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 6px 8px;
  }
</style>
