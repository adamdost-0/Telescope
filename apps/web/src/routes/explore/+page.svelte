<script lang="ts">
  import { page } from '$app/state';

  let cluster = $derived(page.url.searchParams.get('cluster'));
</script>

<h1>Explore</h1>

{#if cluster}
  <p>
    Cluster: <strong data-testid="selected-cluster">{cluster}</strong>
  </p>
{:else}
  <p role="alert">No cluster selected.</p>
{/if}
