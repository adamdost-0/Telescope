<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { getPods, getEvents, deleteResource, listContainers, applyResource, startPortForward, getPodMetrics, checkMetricsAvailable } from '$lib/api';
  import { formatBinaryBytes, formatCpuMillicores } from '$lib/metrics-format';
  import Tabs from '$lib/components/Tabs.svelte';
  import LogViewer from '$lib/components/LogViewer.svelte';
  import EventsTable from '$lib/components/EventsTable.svelte';
  import ExecTerminal from '$lib/components/ExecTerminal.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import YamlEditor from '$lib/components/YamlEditor.svelte';
  import PortForwardDialog from '$lib/components/PortForwardDialog.svelte';
  import AzureIdentitySection from '$lib/components/AzureIdentitySection.svelte';
  import Sparkline from '$lib/components/Sparkline.svelte';
  import Breadcrumbs from '$lib/components/Breadcrumbs.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { gvkForKind, resourceDetailHref } from '$lib/resource-routing';
  import { isProduction } from '$lib/stores';
  import type { ResourceEntry } from '$lib/tauri-commands';

  let namespace = $derived(page.params.namespace);
  let podName = $derived(page.params.name);

  let pod: any = $state(null);
  let events: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let activeTab = $state('summary');
  let error: string | null = $state(null);
  let showDeleteDialog = $state(false);
  let showPortForward = $state(false);
  let deleting = $state(false);
  let deleteError: string | null = $state(null);
  let containerNames: string[] = $state([]);

  // Sparkline metrics ring buffer (30 points, polled every 30s)
  const MAX_SPARKLINE_POINTS = 30;
  let cpuHistory: number[] = $state([]);
  let memoryHistory: number[] = $state([]);
  let metricsTimer: ReturnType<typeof setInterval> | null = null;

  async function pollMetrics() {
    try {
      const available = await checkMetricsAvailable();
      if (!available) return;
      const allMetrics = await getPodMetrics(namespace);
      const podMetric = allMetrics.find((m) => m.name === podName);
      if (podMetric) {
        cpuHistory = [...cpuHistory, podMetric.cpu_millicores].slice(-MAX_SPARKLINE_POINTS);
        memoryHistory = [...memoryHistory, podMetric.memory_bytes].slice(-MAX_SPARKLINE_POINTS);
      }
    } catch {
      // Silently skip metrics poll failures
    }
  }

  // YAML editor state
  let editedYaml = $state('');
  let yamlContent = $derived(pod ? JSON.stringify(pod, null, 2) : '');
  let yamlDirty = $derived(editedYaml !== '' && editedYaml !== yamlContent);
  let applying = $state(false);
  let applyMessage: string | null = $state(null);
  let applyError = $state(false);

  const PROTECTED_NAMESPACES = ['kube-system', 'kube-public', 'kube-node-lease'];
  let requireTypeName = $derived(PROTECTED_NAMESPACES.includes(namespace));
  let availablePorts: number[] = $derived(
    pod?.spec?.containers?.flatMap((c: any) => c.ports?.map((p: any) => p.containerPort) ?? []) ?? []
  );

  const tabs = [
    { id: 'summary', label: 'Summary' },
    { id: 'logs', label: 'Logs' },
    { id: 'exec', label: 'Exec' },
    { id: 'events', label: 'Events' },
    { id: 'yaml', label: 'YAML' },
  ];

  async function loadPod() {
    loading = true;
    error = null;
    try {
      const resources = await getPods(namespace);
      const entry = resources.find((r: ResourceEntry) => r.name === podName);
      if (entry) {
        pod = JSON.parse(entry.content);
      } else {
        error = `Pod "${podName}" not found in namespace "${namespace}"`;
      }
      events = await getEvents(namespace, podName);
      containerNames = await listContainers(namespace, podName);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load pod';
    } finally {
      loading = false;
    }
  }

  async function handleDelete() {
    deleting = true;
    deleteError = null;
    try {
      await deleteResource('v1/Pod', namespace, podName);
      showDeleteDialog = false;
      goto('/pods');
    } catch (e) {
      deleteError = e instanceof Error ? e.message : String(e);
    } finally {
      deleting = false;
    }
  }

  $effect(() => {
    void namespace;
    void podName;
    loadPod();
  });

  onMount(() => {
    pollMetrics();
    metricsTimer = setInterval(pollMetrics, 30_000);
    return () => { if (metricsTimer) clearInterval(metricsTimer); };
  });

  async function handleApply(dryRun: boolean) {
    applying = true;
    applyMessage = null;
    applyError = false;
    try {
      const result = await applyResource(editedYaml, dryRun);
      if (result.success) {
        applyMessage = result.message;
        if (!dryRun) { await loadPod(); }
      } else {
        applyMessage = result.message;
        applyError = true;
      }
    } catch (e) {
      applyMessage = e instanceof Error ? e.message : String(e);
      applyError = true;
    } finally {
      applying = false;
    }
  }

  function resetYaml() {
    editedYaml = yamlContent;
    applyMessage = null;
    applyError = false;
  }

  async function handlePortForward(localPort: number, remotePort: number) {
    showPortForward = false;
    try {
      await startPortForward(namespace, podName, localPort, remotePort);
    } catch (e) {
      console.error('Port forward failed:', e);
    }
  }

  function ownerHref(ref: { kind: string; name: string; apiVersion?: string }) {
    const gvk = ref.apiVersion ? `${ref.apiVersion}/${ref.kind}` : gvkForKind(ref.kind);
    return gvk
      ? resourceDetailHref({ gvk, namespace, name: ref.name, label: ref.kind })
      : null;
  }
</script>

<div class="detail-page">
  <header class="detail-header">
    <Breadcrumbs crumbs={[
      { label: 'Overview', href: '/overview' },
      { label: 'Pods', href: '/pods' },
      { label: namespace, href: `/pods?namespace=${encodeURIComponent(namespace)}` },
      { label: podName }
    ]} />
    <h1>{podName}</h1>
    <span class="namespace-badge">{namespace}</span>
    {#if pod}
      <button class="action-btn" onclick={() => showPortForward = true} data-testid="port-forward-btn">
        <Icon name="port-forward" size={14} aria-hidden="true" /> Port Forward
      </button>
      <button class="danger-btn" onclick={() => showDeleteDialog = true} data-testid="delete-pod-btn">
        <Icon name="delete" size={14} aria-hidden="true" /> Delete
      </button>
    {/if}
  </header>

  {#if loading}
    <p role="status">Loading pod details…</p>
  {:else if error}
    <p role="alert" class="error">{error}</p>
  {:else if pod}
    <Tabs {tabs} {activeTab} onchange={(id) => activeTab = id} />

    {#if activeTab === 'summary'}
      <div class="summary">
        <h3>Status</h3>
        <dl>
          <dt>Phase</dt><dd>{pod.status?.phase ?? 'Unknown'}</dd>
          <dt>Node</dt><dd>{pod.spec?.nodeName ?? 'N/A'}</dd>
          <dt>Pod IP</dt><dd>{pod.status?.podIP ?? 'N/A'}</dd>
          <dt>Host IP</dt><dd>{pod.status?.hostIP ?? 'N/A'}</dd>
          <dt>QoS Class</dt><dd>{pod.status?.qosClass ?? 'N/A'}</dd>
          <dt>Restart Policy</dt><dd>{pod.spec?.restartPolicy ?? 'N/A'}</dd>
          <dt>Service Account</dt><dd>{pod.spec?.serviceAccountName ?? 'default'}</dd>
          <dt>Created</dt><dd>{pod.metadata?.creationTimestamp ?? 'N/A'}</dd>
        </dl>

        <h3>Labels</h3>
        {#if pod.metadata?.labels}
          <div class="labels">
            {#each Object.entries(pod.metadata.labels) as [key, value]}
              <span class="label-badge">{key}={value}</span>
            {/each}
          </div>
        {:else}
          <p class="muted">No labels</p>
        {/if}

        <h3>Containers</h3>
        {#each pod.spec?.containers ?? [] as container}
          <div class="container-card">
            <strong>{container.name}</strong>
            <span class="muted">{container.image}</span>
            {#if container.ports}
              <span class="ports">
                {container.ports.map((p: any) => `${p.containerPort}/${p.protocol ?? 'TCP'}`).join(', ')}
              </span>
            {/if}
            {#if container.resources}
              <div class="resources">
                {#if container.resources.requests}
                  <span class="resource-detail">Requests: CPU {container.resources.requests.cpu ?? '-'}, Mem {container.resources.requests.memory ?? '-'}</span>
                {/if}
                {#if container.resources.limits}
                  <span class="resource-detail">Limits: CPU {container.resources.limits.cpu ?? '-'}, Mem {container.resources.limits.memory ?? '-'}</span>
                {/if}
              </div>
            {:else}
              <span class="no-resources">No resource requests/limits set</span>
            {/if}
          </div>
        {/each}

        {#if pod.metadata?.ownerReferences}
          <h3>Owner</h3>
          {#each pod.metadata.ownerReferences as ref}
            <p>
              {ref.kind}:
              {#if ownerHref(ref)}
                <a class="owner-link" href={ownerHref(ref)}>{ref.name}</a>
              {:else}
                <span class="muted">{ref.name}</span>
              {/if}
            </p>
          {/each}
        {/if}

        <AzureIdentitySection {pod} />

        {#if cpuHistory.length > 1 || memoryHistory.length > 1}
          <h3>Usage Trends</h3>
          <div class="sparkline-row">
            {#if cpuHistory.length > 1}
              <div class="sparkline-card">
                <span class="sparkline-label">CPU (m)</span>
                <Sparkline data={cpuHistory} color="#58a6ff" />
                <span class="sparkline-value">{formatCpuMillicores(cpuHistory[cpuHistory.length - 1])}</span>
              </div>
            {/if}
            {#if memoryHistory.length > 1}
              <div class="sparkline-card">
                <span class="sparkline-label">Memory</span>
                <Sparkline data={memoryHistory} color="#a371f7" />
                <span class="sparkline-value">{formatBinaryBytes(memoryHistory[memoryHistory.length - 1])}</span>
              </div>
            {/if}
          </div>
        {/if}

        {#if pod.status?.conditions}
          <h3>Conditions</h3>
          <table>
            <thead><tr><th scope="col">Type</th><th scope="col">Status</th><th scope="col">Reason</th></tr></thead>
            <tbody>
              {#each pod.status.conditions as cond}
                <tr>
                  <td>{cond.type}</td>
                  <td class={cond.status === 'True' ? 'status-ok' : 'status-bad'}>{cond.status}</td>
                  <td>{cond.reason ?? ''}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

    {:else if activeTab === 'logs'}
      <LogViewer {namespace} pod={podName} />

    {:else if activeTab === 'exec'}
      <ExecTerminal {namespace} pod={podName} containers={containerNames} />

    {:else if activeTab === 'events'}
      <EventsTable events={events} showObject={false} />

    {:else if activeTab === 'yaml'}
      <div class="yaml-tab">
        <div class="yaml-actions">
          <button onclick={() => handleApply(true)} disabled={!yamlDirty || applying} class="action-btn" data-testid="pod-yaml-dry-run">
            <Icon name="dry-run" size={14} aria-hidden="true" /> Dry Run
          </button>
          <button onclick={() => handleApply(false)} disabled={!yamlDirty || applying} class="action-btn primary" data-testid="pod-yaml-apply">
            <Icon name="apply" size={14} aria-hidden="true" /> Apply
          </button>
          <button onclick={resetYaml} disabled={!yamlDirty} class="action-btn" data-testid="pod-yaml-reset">
            <Icon name="reset" size={14} aria-hidden="true" /> Reset
          </button>
          {#if applyMessage}<span class={applyError ? 'apply-error' : 'apply-success'}>{applyMessage}</span>{/if}
        </div>
        <YamlEditor content={yamlContent} onchange={(v) => editedYaml = v} />
      </div>
    {/if}
  {/if}

  <ConfirmDialog
    open={showDeleteDialog}
    title="Delete Pod"
    message={`Are you sure you want to delete pod "${podName}" in namespace "${namespace}"? This action cannot be undone.`}
    confirmText={deleting ? 'Deleting…' : 'Delete'}
    confirmValue={podName}
    requireType={requireTypeName}
    productionContext={$isProduction}
    onconfirm={handleDelete}
    oncancel={() => { showDeleteDialog = false; deleteError = null; }}
  />

  {#if deleteError}
    <p class="error" role="alert">{deleteError}</p>
  {/if}
</div>

<PortForwardDialog
  open={showPortForward}
  podName={podName}
  {namespace}
  {availablePorts}
  onforward={handlePortForward}
  oncancel={() => showPortForward = false}
/>

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

  h1 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
    color: #e0e0e0;
  }

  .namespace-badge {
    background: #1a1a2e;
    color: #8b949e;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    border: 1px solid #21262d;
  }

  .danger-btn {
    margin-left: auto;
    background: transparent;
    color: #ef5350;
    border: 1px solid #ef5350;
    padding: 0.3rem 0.75rem;
    border-radius: 6px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .danger-btn:hover {
    background: rgba(239, 83, 80, 0.15);
  }

  .error {
    color: #ef5350;
    background: rgba(239, 83, 80, 0.1);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    border: 1px solid rgba(239, 83, 80, 0.3);
  }

  /* Summary tab */
  .summary h3 {
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
    grid-template-columns: 10rem 1fr;
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
  }

  .container-card {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
  }
  .container-card strong {
    color: #4fc3f7;
    font-size: 0.875rem;
  }
  .ports {
    color: #8b949e;
    font-size: 0.8rem;
    font-family: monospace;
  }
  .resources {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    margin-top: 0.25rem;
  }
  .resource-detail {
    color: #8b949e;
    font-size: 0.8rem;
    font-family: monospace;
  }
  .no-resources {
    color: #6e7681;
    font-size: 0.8rem;
    font-style: italic;
  }
  .owner-link {
    color: #58a6ff;
    text-decoration: none;
  }
  .owner-link:hover {
    text-decoration: underline;
  }

  .muted { color: #6e7681; font-size: 0.875rem; }

  /* Tables (conditions + events) */
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

  /* YAML tab */
  .yaml-tab { margin-top: 0.5rem; }
  .yaml-actions { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
  .action-btn { background: #21262d; color: #c9d1d9; border: 1px solid #30363d; padding: 0.35rem 0.75rem; border-radius: 6px; font-size: 0.8rem; cursor: pointer; }
  .action-btn:hover:not(:disabled) { background: #30363d; }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-btn.primary { background: #238636; border-color: #2ea043; }
  .action-btn.primary:hover:not(:disabled) { background: #2ea043; }
  .apply-success { color: #66bb6a; font-size: 0.8rem; }
  .apply-error { color: #ef5350; font-size: 0.8rem; }

  .sparkline-row {
    display: flex;
    gap: 1.5rem;
    margin-bottom: 0.75rem;
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
