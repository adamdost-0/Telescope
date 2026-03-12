<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getResourceCounts, getPods, getEvents, activeContext } from '$lib/api';
  import { selectedContext, selectedNamespace, isConnected, clusterServerUrl, isAks } from '$lib/stores';
  import { parseAksUrl, getAzurePortalUrl } from '$lib/azure-utils';
  import AksAddons from '$lib/components/AksAddons.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  // Resource counts keyed by GVK
  let counts: Map<string, number> = $state(new Map());
  let pods: ResourceEntry[] = $state([]);
  let warningEvents: WarningEvent[] = $state([]);
  let contextName: string | null = $state(null);
  let loading = $state(true);
  let refreshError = $state(false);
  let lastSuccessfulRefresh: number | null = $state(null);
  let timer: ReturnType<typeof setInterval> | null = $state(null);

  let staleData = $derived(
    lastSuccessfulRefresh !== null && Date.now() - lastSuccessfulRefresh > 30_000
  );

  interface PodPhase {
    Running: number;
    Pending: number;
    Failed: number;
    Succeeded: number;
    Unknown: number;
  }

  interface WarningEvent {
    name: string;
    namespace: string;
    reason: string;
    message: string;
    involvedObject: string;
    lastTimestamp: string;
  }

  let podPhases: PodPhase = $derived.by(() => {
    const phases: PodPhase = { Running: 0, Pending: 0, Failed: 0, Succeeded: 0, Unknown: 0 };
    for (const pod of pods) {
      try {
        const spec = JSON.parse(pod.content);
        const phase = spec?.status?.phase ?? 'Unknown';
        if (phase in phases) {
          phases[phase as keyof PodPhase]++;
        } else {
          phases.Unknown++;
        }
      } catch {
        phases.Unknown++;
      }
    }
    return phases;
  });

  let totalPods = $derived(pods.length);

  function parseWarningEvents(events: ResourceEntry[]): WarningEvent[] {
    return events
      .map((e) => {
        try {
          const obj = JSON.parse(e.content);
          if (obj?.type !== 'Warning') return null;
          return {
            name: e.name,
            namespace: e.namespace,
            reason: obj?.reason ?? '',
            message: obj?.message ?? '',
            involvedObject: obj?.involvedObject?.name ?? '',
            lastTimestamp: obj?.lastTimestamp ?? obj?.metadata?.creationTimestamp ?? '',
          };
        } catch {
          return null;
        }
      })
      .filter((e): e is WarningEvent => e !== null)
      .sort((a, b) => b.lastTimestamp.localeCompare(a.lastTimestamp))
      .slice(0, 10);
  }

  async function refresh() {
    try {
      const [countsArr, podList, eventList, ctx] = await Promise.all([
        getResourceCounts(),
        getPods($selectedNamespace),
        getEvents($selectedNamespace),
        activeContext(),
      ]);
      counts = new Map(countsArr);
      pods = podList;
      warningEvents = parseWarningEvents(eventList);
      contextName = ctx ?? $selectedContext;
      refreshError = false;
      lastSuccessfulRefresh = Date.now();
    } catch {
      refreshError = true;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    refresh();
    timer = setInterval(refresh, 5000);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  function getCount(gvk: string): number {
    return counts.get(gvk) ?? 0;
  }

  interface CardDef {
    label: string;
    gvk: string;
    icon: string;
    href: string;
  }

  const cards: CardDef[] = [
    { label: 'Pods', gvk: 'v1/Pod', icon: '📦', href: '/pods' },
    { label: 'Deployments', gvk: 'apps/v1/Deployment', icon: '🚀', href: '/resources/deployments' },
    { label: 'StatefulSets', gvk: 'apps/v1/StatefulSet', icon: '🗄️', href: '/resources/statefulsets' },
    { label: 'DaemonSets', gvk: 'apps/v1/DaemonSet', icon: '🔄', href: '/resources/daemonsets' },
    { label: 'Jobs', gvk: 'batch/v1/Job', icon: '⚙️', href: '/resources/jobs' },
    { label: 'CronJobs', gvk: 'batch/v1/CronJob', icon: '🕐', href: '/resources/cronjobs' },
    { label: 'Services', gvk: 'v1/Service', icon: '🌐', href: '/resources/services' },
    { label: 'ConfigMaps', gvk: 'v1/ConfigMap', icon: '📋', href: '/resources/configmaps' },
    { label: 'Secrets', gvk: 'v1/Secret', icon: '🔒', href: '/resources/secrets' },
    { label: 'Nodes', gvk: 'v1/Node', icon: '🖥️', href: '/nodes' },
    { label: 'Events', gvk: 'v1/Event', icon: '⚡', href: '/events' },
  ];

  const phaseColors: Record<string, string> = {
    Running: '#66bb6a',
    Pending: '#ffa726',
    Failed: '#ef5350',
    Succeeded: '#42a5f5',
    Unknown: '#757575',
  };

  function openInPortal() {
    if (!$clusterServerUrl) return;
    const info = parseAksUrl($clusterServerUrl);
    if (!info) return;
    const url = getAzurePortalUrl(info);
    if (url) {
      window.open(url, '_blank', 'noopener');
    }
  }
</script>

<div class="overview">
  {#if !$isConnected}
    <div class="not-connected">
      <h2>No cluster connected</h2>
      <p>Select a cluster from the <a href="/">home page</a> to view the overview dashboard.</p>
    </div>
  {:else}
    <h1>Cluster Overview</h1>

    {#if refreshError && staleData}
      <div class="stale-warning" role="alert">
        ⚠ Data may be stale — last successful refresh was more than 30 seconds ago.
      </div>
    {:else if refreshError}
      <div class="stale-warning" role="alert">
        ⚠ Failed to refresh data. Retrying…
      </div>
    {/if}

    <!-- Cluster info -->
    <section class="cluster-info" aria-label="Cluster information">
      <div class="info-item">
        <span class="info-label">Context</span>
        <span class="info-value">
          {contextName ?? '—'}
          {#if $isAks}
            <span class="aks-badge" title="Azure Kubernetes Service">AKS</span>
          {/if}
        </span>
      </div>
      <div class="info-item">
        <span class="info-label">Namespace</span>
        <span class="info-value">{$selectedNamespace}</span>
      </div>
      {#if $isAks}
        <div class="info-item portal-link">
          <button class="portal-btn" onclick={openInPortal} title="Open cluster in Azure Portal">
            🌐 Open in Azure Portal
          </button>
        </div>
      {/if}
    </section>

    <!-- Resource counts grid -->
    <section aria-label="Resource counts">
      <h2>Resources</h2>
      {#if loading}
        <p role="status">Loading resource counts…</p>
      {:else}
        <div class="card-grid">
          {#each cards as card}
            <a href={card.href} class="card" data-testid="resource-card-{card.label.toLowerCase()}">
              <span class="card-icon">{card.icon}</span>
              <div class="card-body">
                <span class="card-count">{getCount(card.gvk)}</span>
                <span class="card-label">{card.label}</span>
              </div>
              {#if card.gvk === 'v1/Pod' && totalPods > 0}
                <div class="card-indicator">
                  <span class="mini-badge running">{podPhases.Running} running</span>
                  {#if podPhases.Pending > 0}
                    <span class="mini-badge pending">{podPhases.Pending} pending</span>
                  {/if}
                  {#if podPhases.Failed > 0}
                    <span class="mini-badge failed">{podPhases.Failed} failed</span>
                  {/if}
                </div>
              {/if}
              {#if card.gvk === 'v1/Event'}
                <div class="card-indicator">
                  <span class="mini-badge warning">⚠ {warningEvents.length} warnings</span>
                </div>
              {/if}
            </a>
          {/each}
        </div>
      {/if}
    </section>

    <!-- AKS Add-ons -->
    {#if $isAks}
      <AksAddons />
    {/if}

    <!-- Pod phase breakdown -->
    {#if totalPods > 0}
      <section aria-label="Pod phase breakdown">
        <h2>Pod Phases</h2>
        <div class="phase-bar">
          {#each Object.entries(podPhases) as [phase, count]}
            {#if count > 0}
              <div
                class="phase-segment"
                style="flex: {count}; background: {phaseColors[phase]};"
                title="{phase}: {count}"
              >
                {phase} ({count})
              </div>
            {/if}
          {/each}
        </div>
        <div class="phase-legend">
          {#each Object.entries(phaseColors) as [phase, color]}
            <span class="legend-item">
              <span class="legend-dot" style="background: {color};"></span>
              {phase}
            </span>
          {/each}
        </div>
      </section>
    {/if}

    <!-- Recent warning events -->
    {#if warningEvents.length > 0}
      <section aria-label="Recent warning events">
        <h2>Recent Warnings</h2>
        <div class="events-table-wrap">
          <table class="events-table">
            <thead>
              <tr>
                <th>Object</th>
                <th>Reason</th>
                <th>Message</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {#each warningEvents as evt}
                <tr>
                  <td class="cell-object">{evt.involvedObject}</td>
                  <td class="cell-reason">{evt.reason}</td>
                  <td class="cell-message">{evt.message}</td>
                  <td class="cell-time">{evt.lastTimestamp ? new Date(evt.lastTimestamp).toLocaleTimeString() : '—'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .overview {
    max-width: 1200px;
    margin: 0 auto;
  }
  h1 {
    margin: 0 0 1rem;
    font-size: 1.5rem;
    color: #e0e0e0;
  }
  h2 {
    font-size: 1.1rem;
    color: #8b949e;
    margin: 1.5rem 0 0.75rem;
  }

  /* Not connected */
  .not-connected {
    text-align: center;
    padding: 3rem 1rem;
    color: #8b949e;
  }
  .not-connected a {
    color: #58a6ff;
  }

  /* Stale data warning */
  .stale-warning {
    padding: 0.5rem 1rem;
    margin-bottom: 0.75rem;
    background: rgba(255, 167, 38, 0.12);
    border: 1px solid rgba(255, 167, 38, 0.3);
    border-radius: 6px;
    color: #ffa726;
    font-size: 0.85rem;
  }

  /* Cluster info bar */
  .cluster-info {
    display: flex;
    gap: 2rem;
    padding: 0.75rem 1rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    margin-bottom: 0.5rem;
  }
  .info-item {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .info-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #484f58;
  }
  .info-value {
    font-size: 0.9rem;
    color: #e0e0e0;
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', monospace;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .aks-badge {
    display: inline-flex;
    align-items: center;
    background: rgba(0, 120, 212, 0.2);
    color: #0078d4;
    font-size: 0.65rem;
    font-weight: 600;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }
  .portal-link {
    margin-left: auto;
    justify-content: center;
  }
  .portal-btn {
    background: rgba(0, 120, 212, 0.15);
    color: #58a6ff;
    border: 1px solid rgba(0, 120, 212, 0.3);
    padding: 0.35rem 0.75rem;
    border-radius: 6px;
    font-size: 0.8rem;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s, border-color 0.15s;
  }
  .portal-btn:hover {
    background: rgba(0, 120, 212, 0.3);
    border-color: #0078d4;
  }

  /* Card grid */
  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 0.75rem;
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    padding: 1rem 0.75rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    text-decoration: none;
    color: #e0e0e0;
    transition: border-color 0.15s, background 0.15s;
  }
  .card:hover {
    border-color: #58a6ff;
    background: #1a2332;
  }
  .card-icon {
    font-size: 1.5rem;
  }
  .card-body {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .card-count {
    font-size: 1.75rem;
    font-weight: 600;
    line-height: 1.2;
  }
  .card-label {
    font-size: 0.8rem;
    color: #8b949e;
  }
  .card-indicator {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    justify-content: center;
    margin-top: 0.25rem;
  }
  .mini-badge {
    font-size: 0.65rem;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    white-space: nowrap;
  }
  .mini-badge.running { background: rgba(102, 187, 106, 0.2); color: #66bb6a; }
  .mini-badge.pending { background: rgba(255, 167, 38, 0.2); color: #ffa726; }
  .mini-badge.failed  { background: rgba(239, 83, 80, 0.2); color: #ef5350; }
  .mini-badge.warning { background: rgba(255, 167, 38, 0.15); color: #ffa726; }

  /* Phase bar */
  .phase-bar {
    display: flex;
    height: 28px;
    border-radius: 6px;
    overflow: hidden;
    gap: 2px;
  }
  .phase-segment {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.7rem;
    font-weight: 500;
    color: #000;
    min-width: 40px;
    white-space: nowrap;
    padding: 0 0.5rem;
  }
  .phase-legend {
    display: flex;
    gap: 1rem;
    margin-top: 0.5rem;
    font-size: 0.75rem;
    color: #8b949e;
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }

  /* Events table */
  .events-table-wrap {
    overflow-x: auto;
  }
  .events-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  .events-table th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #21262d;
    color: #8b949e;
    font-weight: 500;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .events-table td {
    padding: 0.4rem 0.75rem;
    border-bottom: 1px solid #161b22;
    color: #e0e0e0;
  }
  .events-table tr:hover td {
    background: #161b22;
  }
  .cell-object {
    font-family: 'SFMono-Regular', Consolas, monospace;
    color: #58a6ff;
    white-space: nowrap;
  }
  .cell-reason {
    color: #ffa726;
    white-space: nowrap;
  }
  .cell-message {
    max-width: 400px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cell-time {
    white-space: nowrap;
    color: #8b949e;
  }
</style>
