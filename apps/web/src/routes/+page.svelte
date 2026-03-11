<script lang="ts">
  import type { Cluster } from '$lib/engine';

  let { data }: { data: { clusters: Cluster[] } } = $props();
</script>

<h1>Telescope</h1>

<nav aria-label="resources">
  <a href="/pods">Pods</a>
</nav>

<section aria-label="clusters">
  <h2>Clusters</h2>

  {#if data.clusters.length === 0}
    <p>No clusters found.</p>
  {:else}
    <ul>
      {#each data.clusters as cluster (cluster.id)}
        <li data-testid="cluster-item">{cluster.name}</li>
      {/each}
    </ul>
  {/if}
</section>
