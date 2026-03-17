<script lang="ts">
  import { onMount } from 'svelte';
  import Sparkline from './Sparkline.svelte';
  import {
    cpuCurrent,
    memoryCurrent,
    metrics,
    metricsAvailable,
    restartsCurrent,
    startMetricsPolling,
    stopMetricsPolling,
  } from '$lib/realMetrics';

  let { visible = true }: { visible?: boolean } = $props();

  onMount(() => {
    startMetricsPolling();
    return () => stopMetricsPolling();
  });
</script>

<div class="cluster-vitals" class:hidden={!visible}>
  <div class="vitals-container">
    <div class="vital-item">
      <div class="vital-header">
        <span class="vital-label">CPU</span>
        <span class="vital-value">{$metricsAvailable ? `${$cpuCurrent}%` : 'N/A'}</span>
      </div>
      <Sparkline data={$metrics.cpuUsage} width={100} height={28} color="#58a6ff" />
    </div>

    <div class="vital-item">
      <div class="vital-header">
        <span class="vital-label">Memory</span>
        <span class="vital-value">{$metricsAvailable ? `${$memoryCurrent}%` : 'N/A'}</span>
      </div>
      <Sparkline data={$metrics.memoryUsage} width={100} height={28} color="#79c0ff" />
    </div>

    <div class="vital-item">
      <div class="vital-header">
        <span class="vital-label">Restarts</span>
        <span class="vital-value">{$metricsAvailable ? $restartsCurrent : 'N/A'}</span>
      </div>
      <Sparkline data={$metrics.restarts} width={100} height={28} color="#f85149" />
    </div>
  </div>
</div>

<style>
  .cluster-vitals {
    display: flex;
    align-items: center;
  }

  .cluster-vitals.hidden {
    display: none;
  }

  .vitals-container {
    display: flex;
    gap: 1rem;
    align-items: center;
    padding: 0 0.5rem;
  }

  .vital-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    align-items: center;
  }

  .vital-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.125rem;
  }

  .vital-label {
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .vital-value {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-primary);
  }
</style>
