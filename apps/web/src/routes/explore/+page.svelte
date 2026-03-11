<script lang="ts">
  import { selectedContext, selectedNamespace, isConnected } from '$lib/stores';
</script>

<h1>Explore</h1>

{#if $isConnected && $selectedContext}
  <p>
    Cluster: <strong data-testid="selected-cluster">{$selectedContext}</strong>
  </p>
  <p>Namespace: <strong>{$selectedNamespace}</strong></p>
  <p style="color: #757575;">Resource explorer coming in next milestone.</p>
{:else}
  <p role="alert">Not connected to a cluster. Select one from the header.</p>
{/if}
