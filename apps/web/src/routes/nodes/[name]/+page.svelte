<script lang="ts">
  import { page } from '$app/state';
  import { onMount, onDestroy } from 'svelte';
  import { getResources, getNodeMetrics, checkMetricsAvailable } from '$lib/api';
  import Tabs from '$lib/components/Tabs.svelte';
  import Sparkline from '$lib/components/Sparkline.svelte';
  import type { ResourceEntry, NodeMetricsData } from '$lib/tauri-commands';

  let nodeName = $derived(page.params.name);

  let node: any = $state(null);
  let metrics: NodeMetricsData | null = $state(null);
  let metricsAvailable = $state(false);
  let loading = $state(true);
  let activeTab = $state('summary');
  let error: string | null = $state(null);

  // Sparkline metrics ring buffer (30 points, polled every 30s)
  const MAX_SPARKLINE_POINTS = 30;
  let cpuHistory: number[] = $state([]);
  let memoryHistory: number[] = $state([]);
  let metricsTimer: ReturnType<typeof setInterval> | null = null;

  async function pollNodeMetrics() {
    try {
      const available = await checkMetricsAvailable();
      if (!available) return;
      const allMetrics = await getNodeMetrics();
      const nodeMetric = allMetrics.find((m) => m.name === nodeName);
      if (nodeMetric) {
        cpuHistory = [...cpuHistory, nodeMetric.cpu_millicores].slice(-MAX_SPARKLINE_POINTS);
        memoryHistory = [...memoryHistory, nodeMetric.memory_bytes / (1024 * 1024)].slice(-MAX_SPARKLINE_POINTS);
      }
    } catch {
      // Silently skip metrics poll failures
    }
  }

  const tabs = [
    { id: 'summary', label: 'Summary' },
    { id: 'conditions', label: 'Conditions' },
    { id: 'capacity', label: 'Capacity' },
    { id: 'metrics', label: 'Metrics' },
    { id: 'yaml', label: 'YAML' },
  ];

  function usageColor(pct: number): string {
    if (pct < 70) return '#66bb6a';
    if (pct < 90) return '#ffa726';
    return '#ef5350';
  }

  function usageLabel(pct: number): string {
    if (pct < 70) return 'Normal';
    if (pct < 90) return 'Warning';
    return 'Critical';
  }

  async function loadNode() {
    loading = true;
    error = null;
    try {
      const [resources, available] = await Promise.all([
        getResources('v1/Node', null as unknown as string),
        checkMetricsAvailable(),
      ]);
      metricsAvailable = available;
      const entry = resources.find((r: ResourceEntry) => r.name === nodeName);
      if (entry) {
        node = JSON.parse(entry.content);
      } else {
        error = `Node "${nodeName}" not found`;
      }
      if (available) {
        const allMetrics = await getNodeMetrics();
        metrics = allMetrics.find(m => m.name === nodeName) ?? null;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load node';
    } finally {
      loading = false;
    }
  }

  function getNodeRoles(obj: any): string {
    const labels = obj?.metadata?.labels ?? {};
    const roles: string[] = [];
    for (const key of Object.keys(labels)) {
      if (key.startsWith('node-role.kubernetes.io/')) {
        roles.push(key.replace('node-role.kubernetes.io/', ''));
      }
    }
    return roles.length > 0 ? roles.join(', ') : '<none>';
  }

  function getTaints(obj: any): any[] {
    return obj?.spec?.taints ?? [];
  }

  onMount(loadNode);

  onMount(() => {
    pollNodeMetrics();
    metricsTimer = setInterval(pollNodeMetrics, 30_000);
  });

  onDestroy(() => {
    if (metricsTimer) clearInterval(metricsTimer);
  });
</script>

<div class="detail-page">
  <header class="detail-header">
    <a href="/nodes" class="back">← Nodes</a>
    <h1>{nodeName}</h1>
  </header>

  {#if loading}
    <p role="status">Loading node details…</p>
  {:else if error}
    <p role="alert" class="error">{error}</p>
  {:else if node}
    <Tabs {tabs} {activeTab} onchange={(id) => activeTab = id} />

    {#if activeTab === 'summary'}
      <div class="summary">
        <h3>Node Info</h3>
        <dl>
          <dt>Roles</dt><dd>{getNodeRoles(node)}</dd>
          <dt>OS</dt><dd>{node.status?.nodeInfo?.operatingSystem ?? 'N/A'}</dd>
          <dt>OS Image</dt><dd>{node.status?.nodeInfo?.osImage ?? 'N/A'}</dd>
          <dt>Architecture</dt><dd>{node.status?.nodeInfo?.architecture ?? 'N/A'}</dd>
          <dt>Kernel Version</dt><dd>{node.status?.nodeInfo?.kernelVersion ?? 'N/A'}</dd>
          <dt>Container Runtime</dt><dd>{node.status?.nodeInfo?.containerRuntimeVersion ?? 'N/A'}</dd>
          <dt>Kubelet Version</dt><dd>{node.status?.nodeInfo?.kubeletVersion ?? 'N/A'}</dd>
          <dt>Kube-Proxy Version</dt><dd>{node.status?.nodeInfo?.kubeProxyVersion ?? 'N/A'}</dd>
          <dt>Created</dt><dd>{node.metadata?.creationTimestamp ?? 'N/A'}</dd>
        </dl>

        <h3>Labels</h3>
        {#if node.metadata?.labels}
          <div class="labels">
            {#each Object.entries(node.metadata.labels) as [key, value]}
              <span class="label-badge">{key}={value}</span>
            {/each}
          </div>
        {:else}
          <p class="muted">No labels</p>
        {/if}

        <h3>Annotations</h3>
        {#if node.metadata?.annotations}
          <div class="labels">
            {#each Object.entries(node.metadata.annotations) as [key, value]}
              <span class="label-badge">{key}={value}</span>
            {/each}
          </div>
        {:else}
          <p class="muted">No annotations</p>
        {/if}

        <h3>Taints</h3>
        {#if getTaints(node).length > 0}
          <table>
            <thead><tr><th scope="col">Key</th><th scope="col">Value</th><th scope="col">Effect</th></tr></thead>
            <tbody>
              {#each getTaints(node) as taint}
                <tr>
                  <td>{taint.key}</td>
                  <td>{taint.value ?? ''}</td>
                  <td>{taint.effect}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="muted">No taints</p>
        {/if}
      </div>

    {:else if activeTab === 'conditions'}
      <div class="tab-content">
        {#if node.status?.conditions}
          <table>
            <thead>
              <tr>
                <th scope="col">Type</th>
                <th scope="col">Status</th>
                <th scope="col">Reason</th>
                <th scope="col">Message</th>
                <th scope="col">Last Transition</th>
              </tr>
            </thead>
            <tbody>
              {#each node.status.conditions as cond}
                <tr>
                  <td>{cond.type}</td>
                  <td class={cond.type === 'Ready' ? (cond.status === 'True' ? 'status-ok' : 'status-bad') : (cond.status === 'False' ? 'status-ok' : 'status-bad')}>{cond.status}</td>
                  <td>{cond.reason ?? ''}</td>
                  <td class="message-cell">{cond.message ?? ''}</td>
                  <td>{cond.lastTransitionTime ?? ''}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="muted">No conditions available</p>
        {/if}
      </div>

    {:else if activeTab === 'capacity'}
      <div class="tab-content">
        <table>
          <thead>
            <tr>
              <th scope="col">Resource</th>
              <th scope="col">Capacity</th>
              <th scope="col">Allocatable</th>
            </tr>
          </thead>
          <tbody>
            {#each ['cpu', 'memory', 'pods', 'ephemeral-storage'] as resource}
              <tr>
                <td>{resource}</td>
                <td>{node.status?.capacity?.[resource] ?? 'N/A'}</td>
                <td>{node.status?.allocatable?.[resource] ?? 'N/A'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

    {:else if activeTab === 'metrics'}
      <div class="tab-content">
        {#if !metricsAvailable}
          <p class="muted">Metrics server is not available on this cluster. Install metrics-server to see resource usage.</p>
        {:else if !metrics}
          <p class="muted">No metrics data available for this node.</p>
        {:else}
          <h3>Resource Usage</h3>

          {#if cpuHistory.length > 1 || memoryHistory.length > 1}
            <div class="sparkline-row">
              {#if cpuHistory.length > 1}
                <div class="sparkline-card">
                  <span class="sparkline-label">CPU Trend (m)</span>
                  <Sparkline data={cpuHistory} color="#58a6ff" />
                  <span class="sparkline-value">{cpuHistory[cpuHistory.length - 1]?.toFixed(0) ?? '—'}m</span>
                </div>
              {/if}
              {#if memoryHistory.length > 1}
                <div class="sparkline-card">
                  <span class="sparkline-label">Memory Trend (MiB)</span>
                  <Sparkline data={memoryHistory} color="#a371f7" />
                  <span class="sparkline-value">{memoryHistory[memoryHistory.length - 1]?.toFixed(0) ?? '—'} MiB</span>
                </div>
              {/if}
            </div>
          {/if}

          <div class="metrics-grid">
            <div class="metric-card">
              <div class="metric-label">CPU Usage</div>
              <div class="metric-value" style="color: {usageColor(metrics.cpu_percent)}">{metrics.cpu_millicores}m / {metrics.cpu_allocatable}m</div>
              <div class="metric-bar">
                <div class="metric-bar-fill" style="width: {Math.min(metrics.cpu_percent, 100)}%; background: {usageColor(metrics.cpu_percent)}"></div>
              </div>
              <div class="metric-detail">
                <span style="color: {usageColor(metrics.cpu_percent)}">{metrics.cpu_percent}%</span> of allocatable — {usageLabel(metrics.cpu_percent)}
              </div>
            </div>
            <div class="metric-card">
              <div class="metric-label">Memory Usage</div>
              <div class="metric-value" style="color: {usageColor(metrics.memory_percent)}">{Math.round(metrics.memory_bytes / (1024 * 1024))}Mi / {Math.round(metrics.memory_allocatable / (1024 * 1024))}Mi</div>
              <div class="metric-bar">
                <div class="metric-bar-fill" style="width: {Math.min(metrics.memory_percent, 100)}%; background: {usageColor(metrics.memory_percent)}"></div>
              </div>
              <div class="metric-detail">
                <span style="color: {usageColor(metrics.memory_percent)}">{metrics.memory_percent}%</span> of allocatable — {usageLabel(metrics.memory_percent)}
              </div>
            </div>
          </div>

          <h3>Capacity vs Usage</h3>
          <table>
            <thead>
              <tr>
                <th scope="col">Resource</th>
                <th scope="col">Used</th>
                <th scope="col">Allocatable</th>
                <th scope="col">Capacity</th>
                <th scope="col">% of Allocatable</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>CPU</td>
                <td>{metrics.cpu_millicores}m</td>
                <td>{metrics.cpu_allocatable}m</td>
                <td>{node?.status?.capacity?.cpu ?? 'N/A'}</td>
                <td style="color: {usageColor(metrics.cpu_percent)}">{metrics.cpu_percent}%</td>
              </tr>
              <tr>
                <td>Memory</td>
                <td>{Math.round(metrics.memory_bytes / (1024 * 1024))}Mi</td>
                <td>{Math.round(metrics.memory_allocatable / (1024 * 1024))}Mi</td>
                <td>{node?.status?.capacity?.memory ?? 'N/A'}</td>
                <td style="color: {usageColor(metrics.memory_percent)}">{metrics.memory_percent}%</td>
              </tr>
            </tbody>
          </table>
        {/if}
      </div>

    {:else if activeTab === 'yaml'}
      <pre class="yaml-view"><code>{JSON.stringify(node, null, 2)}</code></pre>
    {/if}
  {/if}
</div>

<style>
  .detail-page {
    padding: 1.5rem;
    color: #e0e0e0;
    max-width: 960px;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .back {
    color: #58a6ff;
    text-decoration: none;
    font-size: 0.875rem;
  }
  .back:hover { text-decoration: underline; }

  h1 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
    color: #e0e0e0;
  }

  .error {
    color: #ef5350;
    background: rgba(239, 83, 80, 0.1);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    border: 1px solid rgba(239, 83, 80, 0.3);
  }

  .summary h3, .tab-content h3 {
    color: #8b949e;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 1.25rem 0 0.5rem;
    border-bottom: 1px solid #21262d;
    padding-bottom: 0.25rem;
  }
  .summary h3:first-child {
    margin-top: 0;
  }

  dl {
    display: grid;
    grid-template-columns: 12rem 1fr;
    gap: 0.25rem 1rem;
    margin: 0;
    font-size: 0.875rem;
  }
  dt {
    color: #8b949e;
    font-weight: 500;
  }
  dd {
    color: #e0e0e0;
    margin: 0;
    font-family: monospace;
  }

  .labels {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }
  .label-badge {
    background: #1a1a2e;
    color: #8b949e;
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    font-size: 0.75rem;
    font-family: monospace;
    border: 1px solid #21262d;
    word-break: break-all;
  }

  .muted { color: #6e7681; font-size: 0.875rem; }

  .tab-content {
    margin-top: 0.5rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
    font-family: monospace;
  }
  thead {
    background: #1a1a2e;
    color: #e0e0e0;
  }
  th, td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid #2a2a3e;
  }
  tr:hover { background: #16213e; }

  .status-ok { color: #66bb6a; }
  .status-bad { color: #ef5350; }
  .message-cell { max-width: 300px; word-break: break-word; }

  .yaml-view {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 1rem;
    overflow: auto;
    max-height: 70vh;
    font-size: 0.8rem;
    line-height: 1.5;
    color: #c9d1d9;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .metric-card {
    background: #1a1a2e;
    border: 1px solid #21262d;
    border-radius: 8px;
    padding: 1rem;
  }

  .metric-label {
    color: #8b949e;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.5rem;
  }

  .metric-value {
    font-size: 1.25rem;
    font-weight: 600;
    font-family: monospace;
    margin-bottom: 0.5rem;
  }

  .metric-bar {
    height: 6px;
    background: #21262d;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 0.5rem;
  }

  .metric-bar-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .metric-detail {
    font-size: 0.8rem;
    color: #8b949e;
  }

  .sparkline-row {
    display: flex;
    gap: 1.5rem;
    margin-bottom: 1rem;
  }
  .sparkline-card {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
  }
  .sparkline-label {
    font-size: 0.75rem;
    color: #8b949e;
    white-space: nowrap;
  }
  .sparkline-value {
    font-size: 0.8rem;
    font-family: monospace;
    color: #e0e0e0;
    white-space: nowrap;
  }
</style>
