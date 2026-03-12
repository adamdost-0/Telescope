<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { getResources, getEvents, getPods, rolloutRestart, rolloutStatus, applyResource, scaleResource } from '$lib/api';
  import type { RolloutStatus } from '$lib/api';
  import Tabs from '$lib/components/Tabs.svelte';
  import EventsTable from '$lib/components/EventsTable.svelte';
  import YamlEditor from '$lib/components/YamlEditor.svelte';
  import ScaleDialog from '$lib/components/ScaleDialog.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import AzureIdentitySection from '$lib/components/AzureIdentitySection.svelte';
  import Breadcrumbs from '$lib/components/Breadcrumbs.svelte';
  import { isProduction } from '$lib/stores';
  import type { ResourceEntry } from '$lib/tauri-commands';

  const KIND_MAP: Record<string, { gvk: string; label: string }> = {
    deployments: { gvk: 'apps/v1/Deployment', label: 'Deployment' },
    services: { gvk: 'v1/Service', label: 'Service' },
    statefulsets: { gvk: 'apps/v1/StatefulSet', label: 'StatefulSet' },
    daemonsets: { gvk: 'apps/v1/DaemonSet', label: 'DaemonSet' },
    jobs: { gvk: 'batch/v1/Job', label: 'Job' },
    cronjobs: { gvk: 'batch/v1/CronJob', label: 'CronJob' },
    configmaps: { gvk: 'v1/ConfigMap', label: 'ConfigMap' },
    secrets: { gvk: 'v1/Secret', label: 'Secret' },
    ingresses: { gvk: 'networking.k8s.io/v1/Ingress', label: 'Ingress' },
    pvcs: { gvk: 'v1/PersistentVolumeClaim', label: 'PVC' },
  };

  const WORKLOAD_KINDS = new Set(['deployments', 'statefulsets', 'daemonsets']);
  const SCALABLE_KINDS = new Set(['deployments', 'statefulsets']);
  const RESTARTABLE_KINDS = new Set(['deployments', 'statefulsets', 'daemonsets']);
  const ROLLOUT_STATUS_KINDS = new Set(['deployments', 'statefulsets']);

  let kind = $derived(page.params.kind);
  let namespace = $derived(page.params.namespace);
  let resourceName = $derived(page.params.name);
  let kindInfo = $derived(KIND_MAP[kind]);
  let isWorkload = $derived(WORKLOAD_KINDS.has(kind));
  let isScalable = $derived(SCALABLE_KINDS.has(kind));
  let isRestartable = $derived(RESTARTABLE_KINDS.has(kind));
  let hasRolloutStatus = $derived(ROLLOUT_STATUS_KINDS.has(kind));
  let showScale = $state(false);
  let currentReplicas = $derived(resource?.spec?.replicas ?? 1);

  let resource: any = $state(null);
  let events: ResourceEntry[] = $state([]);
  let ownedPods: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let activeTab = $state('summary');
  let error: string | null = $state(null);

  let rollout: RolloutStatus | null = $state(null);
  let rolloutLoading = $state(false);
  let rolloutMessage: string | null = $state(null);
  let showRestartConfirm = $state(false);
  let showProdRestartConfirm = $state(false);
  let pendingScaleReplicas: number | null = $state(null);
  let showScaleConfirm = $state(false);

  // YAML editor state
  let editedYaml = $state('');
  let yamlContent = $derived(resource ? JSON.stringify(resource, null, 2) : '');
  let yamlDirty = $derived(editedYaml !== '' && editedYaml !== yamlContent);
  let applying = $state(false);
  let applyMessage: string | null = $state(null);
  let applyError = $state(false);

  let tabs = $derived(buildTabs());

  function buildTabs() {
    const t = [
      { id: 'summary', label: 'Summary' },
      { id: 'events', label: 'Events' },
      { id: 'yaml', label: 'YAML' },
    ];
    if (isWorkload) {
      t.splice(1, 0, { id: 'pods', label: 'Pods' });
    }
    return t;
  }

  async function loadResource() {
    loading = true;
    error = null;
    try {
      if (!kindInfo) {
        error = `Unknown resource kind: "${kind}"`;
        return;
      }
      const resources = await getResources(kindInfo.gvk, namespace);
      const entry = resources.find((r: ResourceEntry) => r.name === resourceName);
      if (entry) {
        resource = JSON.parse(entry.content);
      } else {
        error = `${kindInfo.label} "${resourceName}" not found in namespace "${namespace}"`;
        return;
      }
      events = await getEvents(namespace, resourceName);

      if (isWorkload && resource?.spec?.selector?.matchLabels) {
        const allPods = await getPods(namespace);
        const selectorLabels = resource.spec.selector.matchLabels as Record<string, string>;
        ownedPods = allPods.filter((p: ResourceEntry) => {
          try {
            const pod = JSON.parse(p.content);
            const podLabels = pod.metadata?.labels ?? {};
            return Object.entries(selectorLabels).every(
              ([k, v]) => podLabels[k] === v
            );
          } catch {
            return false;
          }
        });
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load resource';
    } finally {
      loading = false;
      loadRolloutStatus();
    }
  }

  onMount(loadResource);

  async function handleApply(dryRun: boolean) {
    applying = true;
    applyMessage = null;
    applyError = false;
    try {
      const result = await applyResource(editedYaml, dryRun);
      if (result.success) {
        applyMessage = result.message;
        if (!dryRun) { await loadResource(); }
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

  async function loadRolloutStatus() {
    if (!hasRolloutStatus || !kindInfo) return;
    try {
      rollout = await rolloutStatus(kindInfo.gvk, namespace, resourceName);
    } catch {
      rollout = null;
    }
  }

  async function handleRolloutRestart() {
    showRestartConfirm = false;
    showProdRestartConfirm = false;
    rolloutLoading = true;
    rolloutMessage = null;
    try {
      rolloutMessage = await rolloutRestart(kindInfo.gvk, namespace, resourceName);
      setTimeout(loadRolloutStatus, 2000);
    } catch (e) {
      rolloutMessage = `Error: ${e instanceof Error ? e.message : e}`;
    } finally {
      rolloutLoading = false;
    }
  }

  function formatAge(ts: string): string {
    if (!ts) return 'N/A';
    const diffMs = Date.now() - new Date(ts).getTime();
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 60) return `${diffSec}s`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h`;
    return `${Math.floor(diffHours / 24)}d`;
  }

  function podPhase(entry: ResourceEntry): string {
    try {
      return JSON.parse(entry.content).status?.phase ?? 'Unknown';
    } catch {
      return 'Unknown';
    }
  }

  function podRestarts(entry: ResourceEntry): number {
    try {
      const statuses = JSON.parse(entry.content).status?.containerStatuses ?? [];
      return statuses.reduce((sum: number, s: any) => sum + (s.restartCount ?? 0), 0);
    } catch {
      return 0;
    }
  }

  function podAge(entry: ResourceEntry): string {
    try {
      return formatAge(JSON.parse(entry.content).metadata?.creationTimestamp ?? '');
    } catch {
      return 'N/A';
    }
  }

  function truncate(str: string, max: number): string {
    return str.length > max ? str.slice(0, max) + '…' : str;
  }

  async function handleScale(replicas: number) {
    showScale = false;
    if (!kindInfo) return;
    if ($isProduction) {
      pendingScaleReplicas = replicas;
      showScaleConfirm = true;
      return;
    }
    await executeScale(replicas);
  }

  async function executeScale(replicas: number) {
    if (!kindInfo) return;
    try {
      await scaleResource(kindInfo.gvk, namespace, resourceName, replicas);
      await loadResource();
    } catch (e) {
      console.error('Scale failed:', e);
    }
  }

  async function confirmScale() {
    showScaleConfirm = false;
    if (pendingScaleReplicas !== null) {
      await executeScale(pendingScaleReplicas);
      pendingScaleReplicas = null;
    }
  }
</script>

<div class="detail-page">
  <header class="detail-header">
    <Breadcrumbs crumbs={[
      { label: 'Overview', href: '/' },
      { label: kindInfo?.label ?? kind, href: `/resources/${kind}` },
      { label: namespace, href: `/resources/${kind}?namespace=${encodeURIComponent(namespace)}` },
      { label: resourceName }
    ]} />
    <h1>{resourceName}</h1>
    <span class="namespace-badge">{namespace}</span>
    {#if isScalable && resource}
      <button class="action-btn" onclick={() => showScale = true}>⚖ Scale</button>
    {/if}
  </header>

  {#if loading}
    <p role="status">Loading {kindInfo?.label?.toLowerCase() ?? 'resource'} details…</p>
  {:else if error}
    <p role="alert" class="error">{error}</p>
  {:else if resource}
    <Tabs {tabs} {activeTab} onchange={(id) => activeTab = id} />

    {#if activeTab === 'summary'}
      <div class="summary">
        <!-- Deployment Summary -->
        {#if kind === 'deployments'}
          <h3>Replicas</h3>
          <dl>
            <dt>Desired</dt><dd>{resource.spec?.replicas ?? 0}</dd>
            <dt>Ready</dt><dd>{resource.status?.readyReplicas ?? 0}</dd>
            <dt>Updated</dt><dd>{resource.status?.updatedReplicas ?? 0}</dd>
            <dt>Available</dt><dd>{resource.status?.availableReplicas ?? 0}</dd>
          </dl>

          <h3>Strategy</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.strategy?.type ?? 'RollingUpdate'}</dd>
            {#if resource.spec?.strategy?.rollingUpdate}
              <dt>Max Surge</dt><dd>{resource.spec.strategy.rollingUpdate.maxSurge ?? 'N/A'}</dd>
              <dt>Max Unavailable</dt><dd>{resource.spec.strategy.rollingUpdate.maxUnavailable ?? 'N/A'}</dd>
            {/if}
          </dl>

          <h3>Selector</h3>
          {#if resource.spec?.selector?.matchLabels}
            <div class="labels">
              {#each Object.entries(resource.spec.selector.matchLabels) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector labels</p>
          {/if}

          <h3>Container Images</h3>
          {#each resource.spec?.template?.spec?.containers ?? [] as container}
            <div class="container-card">
              <strong>{container.name}</strong>
              <span class="muted">{container.image}</span>
            </div>
          {/each}

          <h3>Rollout</h3>
          <div class="rollout-section">
            {#if rollout}
              <dl>
                <dt>Status</dt>
                <dd>
                  {#if rollout.is_complete}
                    <span class="badge badge-success">Complete</span>
                  {:else}
                    <span class="badge badge-pending">In Progress</span>
                  {/if}
                  {rollout.message}
                </dd>
                <dt>Progress</dt>
                <dd>{rollout.ready}/{rollout.desired} ready, {rollout.updated}/{rollout.desired} updated, {rollout.available}/{rollout.desired} available</dd>
              </dl>
            {/if}
            <div class="rollout-actions">
              {#if showRestartConfirm}
                <span class="confirm-prompt">Restart all pods?</span>
                <button class="btn btn-danger btn-sm" onclick={handleRolloutRestart} disabled={rolloutLoading}>
                  {rolloutLoading ? 'Restarting…' : 'Confirm Restart'}
                </button>
                <button class="btn btn-secondary btn-sm" onclick={() => showRestartConfirm = false}>Cancel</button>
              {:else}
                <button class="btn btn-primary btn-sm" onclick={() => {
                  if ($isProduction) {
                    showProdRestartConfirm = true;
                  } else {
                    showRestartConfirm = true;
                  }
                }} disabled={rolloutLoading}>
                  Restart Rollout
                </button>
              {/if}
            </div>
            {#if rolloutMessage}
              <p class="rollout-message">{rolloutMessage}</p>
            {/if}
          </div>

        <!-- Service Summary -->
        {:else if kind === 'services'}
          <h3>Service Info</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.type ?? 'ClusterIP'}</dd>
            <dt>Cluster IP</dt><dd>{resource.spec?.clusterIP ?? 'None'}</dd>
            {#if resource.spec?.externalIPs?.length}
              <dt>External IPs</dt><dd>{resource.spec.externalIPs.join(', ')}</dd>
            {/if}
            {#if resource.spec?.loadBalancerIP}
              <dt>Load Balancer IP</dt><dd>{resource.spec.loadBalancerIP}</dd>
            {/if}
            {#if resource.status?.loadBalancer?.ingress?.length}
              <dt>External</dt>
              <dd>{resource.status.loadBalancer.ingress.map((i: any) => i.ip ?? i.hostname ?? '').join(', ')}</dd>
            {/if}
            <dt>Session Affinity</dt><dd>{resource.spec?.sessionAffinity ?? 'None'}</dd>
          </dl>

          <h3>Ports</h3>
          {#if resource.spec?.ports?.length}
            <table>
              <thead><tr><th scope="col">Name</th><th scope="col">Port</th><th scope="col">Target</th><th scope="col">Protocol</th><th scope="col">Node Port</th></tr></thead>
              <tbody>
                {#each resource.spec.ports as port}
                  <tr>
                    <td>{port.name ?? ''}</td>
                    <td>{port.port}</td>
                    <td>{port.targetPort ?? port.port}</td>
                    <td>{port.protocol ?? 'TCP'}</td>
                    <td>{port.nodePort ?? '—'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No ports defined</p>
          {/if}

          <h3>Selector</h3>
          {#if resource.spec?.selector}
            <div class="labels">
              {#each Object.entries(resource.spec.selector) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector</p>
          {/if}

        <!-- StatefulSet Summary -->
        {:else if kind === 'statefulsets'}
          <h3>Replicas</h3>
          <dl>
            <dt>Desired</dt><dd>{resource.spec?.replicas ?? 0}</dd>
            <dt>Ready</dt><dd>{resource.status?.readyReplicas ?? 0}</dd>
            <dt>Current</dt><dd>{resource.status?.currentReplicas ?? 0}</dd>
          </dl>

          <h3>Update Strategy</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.updateStrategy?.type ?? 'RollingUpdate'}</dd>
            <dt>Service Name</dt><dd>{resource.spec?.serviceName ?? 'N/A'}</dd>
            <dt>Pod Management</dt><dd>{resource.spec?.podManagementPolicy ?? 'OrderedReady'}</dd>
          </dl>

          <h3>Selector</h3>
          {#if resource.spec?.selector?.matchLabels}
            <div class="labels">
              {#each Object.entries(resource.spec.selector.matchLabels) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector labels</p>
          {/if}

          {#if resource.spec?.volumeClaimTemplates?.length}
            <h3>Volume Claim Templates</h3>
            {#each resource.spec.volumeClaimTemplates as vct}
              <div class="container-card">
                <strong>{vct.metadata?.name ?? 'unnamed'}</strong>
                <span class="muted">
                  {vct.spec?.accessModes?.join(', ') ?? 'N/A'} —
                  {vct.spec?.resources?.requests?.storage ?? 'N/A'}
                </span>
              </div>
            {/each}
          {/if}

          <h3>Rollout</h3>
          <div class="rollout-section">
            {#if rollout}
              <dl>
                <dt>Status</dt>
                <dd>
                  {#if rollout.is_complete}
                    <span class="badge badge-success">Complete</span>
                  {:else}
                    <span class="badge badge-pending">In Progress</span>
                  {/if}
                  {rollout.message}
                </dd>
                <dt>Progress</dt>
                <dd>{rollout.ready}/{rollout.desired} ready, {rollout.updated}/{rollout.desired} updated</dd>
              </dl>
            {/if}
            <div class="rollout-actions">
              {#if showRestartConfirm}
                <span class="confirm-prompt">Restart all pods?</span>
                <button class="btn btn-danger btn-sm" onclick={handleRolloutRestart} disabled={rolloutLoading}>
                  {rolloutLoading ? 'Restarting…' : 'Confirm Restart'}
                </button>
                <button class="btn btn-secondary btn-sm" onclick={() => showRestartConfirm = false}>Cancel</button>
              {:else}
                <button class="btn btn-primary btn-sm" onclick={() => {
                  if ($isProduction) {
                    showProdRestartConfirm = true;
                  } else {
                    showRestartConfirm = true;
                  }
                }} disabled={rolloutLoading}>
                  Restart Rollout
                </button>
              {/if}
            </div>
            {#if rolloutMessage}
              <p class="rollout-message">{rolloutMessage}</p>
            {/if}
          </div>

        <!-- DaemonSet Summary -->
        {:else if kind === 'daemonsets'}
          <h3>Status</h3>
          <dl>
            <dt>Desired</dt><dd>{resource.status?.desiredNumberScheduled ?? 0}</dd>
            <dt>Current</dt><dd>{resource.status?.currentNumberScheduled ?? 0}</dd>
            <dt>Ready</dt><dd>{resource.status?.numberReady ?? 0}</dd>
            <dt>Up-to-date</dt><dd>{resource.status?.updatedNumberScheduled ?? 0}</dd>
            <dt>Available</dt><dd>{resource.status?.numberAvailable ?? 0}</dd>
          </dl>

          <h3>Update Strategy</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.updateStrategy?.type ?? 'RollingUpdate'}</dd>
          </dl>

          <h3>Selector</h3>
          {#if resource.spec?.selector?.matchLabels}
            <div class="labels">
              {#each Object.entries(resource.spec.selector.matchLabels) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector labels</p>
          {/if}

          <h3>Rollout Restart</h3>
          <div class="rollout-section">
            <div class="rollout-actions">
              {#if showRestartConfirm}
                <span class="confirm-prompt">Restart all pods?</span>
                <button class="btn btn-danger btn-sm" onclick={handleRolloutRestart} disabled={rolloutLoading}>
                  {rolloutLoading ? 'Restarting…' : 'Confirm Restart'}
                </button>
                <button class="btn btn-secondary btn-sm" onclick={() => showRestartConfirm = false}>Cancel</button>
              {:else}
                <button class="btn btn-primary btn-sm" onclick={() => {
                  if ($isProduction) {
                    showProdRestartConfirm = true;
                  } else {
                    showRestartConfirm = true;
                  }
                }} disabled={rolloutLoading}>
                  Restart Rollout
                </button>
              {/if}
            </div>
            {#if rolloutMessage}
              <p class="rollout-message">{rolloutMessage}</p>
            {/if}
          </div>

        <!-- Job Summary -->
        {:else if kind === 'jobs'}
          <h3>Spec</h3>
          <dl>
            <dt>Completions</dt><dd>{resource.spec?.completions ?? 1}</dd>
            <dt>Parallelism</dt><dd>{resource.spec?.parallelism ?? 1}</dd>
            <dt>Backoff Limit</dt><dd>{resource.spec?.backoffLimit ?? 6}</dd>
          </dl>

          <h3>Status</h3>
          <dl>
            <dt>Active</dt><dd>{resource.status?.active ?? 0}</dd>
            <dt>Succeeded</dt><dd>{resource.status?.succeeded ?? 0}</dd>
            <dt>Failed</dt><dd>{resource.status?.failed ?? 0}</dd>
            {#if resource.status?.startTime}
              <dt>Start Time</dt><dd>{resource.status.startTime}</dd>
            {/if}
            {#if resource.status?.completionTime}
              <dt>Completion Time</dt><dd>{resource.status.completionTime}</dd>
              <dt>Duration</dt>
              <dd>{formatAge(resource.status.startTime)} → completed</dd>
            {/if}
          </dl>

        <!-- CronJob Summary -->
        {:else if kind === 'cronjobs'}
          <h3>Schedule</h3>
          <dl>
            <dt>Schedule</dt><dd>{resource.spec?.schedule ?? 'N/A'}</dd>
            <dt>Suspend</dt><dd>{resource.spec?.suspend ? 'Yes' : 'No'}</dd>
            <dt>Concurrency Policy</dt><dd>{resource.spec?.concurrencyPolicy ?? 'Allow'}</dd>
            {#if resource.status?.lastScheduleTime}
              <dt>Last Scheduled</dt><dd>{resource.status.lastScheduleTime}</dd>
            {/if}
          </dl>

          {#if resource.status?.active?.length}
            <h3>Active Jobs</h3>
            {#each resource.status.active as job}
              <div class="container-card">
                <strong>{job.name ?? 'unnamed'}</strong>
                <span class="muted">{job.namespace ?? namespace}</span>
              </div>
            {/each}
          {/if}

        <!-- ConfigMap Summary -->
        {:else if kind === 'configmaps'}
          <h3>Data ({Object.keys(resource.data ?? {}).length} keys)</h3>
          {#if resource.data && Object.keys(resource.data).length > 0}
            <table>
              <thead><tr><th scope="col">Key</th><th scope="col">Value</th></tr></thead>
              <tbody>
                {#each Object.entries(resource.data) as [key, value]}
                  <tr>
                    <td class="resource-name">{key}</td>
                    <td class="muted">{truncate(String(value), 120)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No data</p>
          {/if}

        <!-- Secret Summary -->
        {:else if kind === 'secrets'}
          <h3>Info</h3>
          <dl>
            <dt>Type</dt><dd>{resource.type ?? 'Opaque'}</dd>
            <dt>Keys</dt><dd>{Object.keys(resource.data ?? {}).length}</dd>
          </dl>

          <h3>Data Keys</h3>
          {#if resource.data && Object.keys(resource.data).length > 0}
            <table>
              <thead><tr><th scope="col">Key</th><th scope="col">Value</th></tr></thead>
              <tbody>
                {#each Object.keys(resource.data) as key}
                  <tr>
                    <td class="resource-name">{key}</td>
                    <td class="muted">••••••••</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No data</p>
          {/if}

        <!-- Ingress Summary -->
        {:else if kind === 'ingresses'}
          <h3>Info</h3>
          <dl>
            {#if resource.spec?.ingressClassName}
              <dt>Ingress Class</dt><dd>{resource.spec.ingressClassName}</dd>
            {/if}
          </dl>

          {#if resource.spec?.tls?.length}
            <h3>TLS</h3>
            {#each resource.spec.tls as tls}
              <div class="container-card">
                <strong>{tls.secretName ?? 'unnamed'}</strong>
                <span class="muted">{(tls.hosts ?? []).join(', ')}</span>
              </div>
            {/each}
          {/if}

          {#if resource.spec?.rules?.length}
            <h3>Rules</h3>
            {#each resource.spec.rules as rule}
              <div class="container-card">
                <strong>{rule.host ?? '*'}</strong>
                {#each rule.http?.paths ?? [] as path}
                  <span class="muted">
                    {path.path ?? '/'} ({path.pathType ?? 'Prefix'}) →
                    {path.backend?.service?.name ?? '?'}:{path.backend?.service?.port?.number ?? path.backend?.service?.port?.name ?? '?'}
                  </span>
                {/each}
              </div>
            {/each}
          {/if}

        <!-- PVC Summary -->
        {:else if kind === 'pvcs'}
          <h3>Info</h3>
          <dl>
            <dt>Status</dt><dd>{resource.status?.phase ?? 'Unknown'}</dd>
            <dt>Volume</dt><dd>{resource.spec?.volumeName ?? 'N/A'}</dd>
            <dt>Storage Class</dt><dd>{resource.spec?.storageClassName ?? 'N/A'}</dd>
            <dt>Access Modes</dt><dd>{(resource.spec?.accessModes ?? []).join(', ') || 'N/A'}</dd>
            <dt>Capacity</dt><dd>{resource.status?.capacity?.storage ?? 'N/A'}</dd>
            <dt>Requested</dt><dd>{resource.spec?.resources?.requests?.storage ?? 'N/A'}</dd>
          </dl>

        <!-- Fallback for unknown kinds -->
        {:else}
          <h3>Metadata</h3>
          <dl>
            <dt>Name</dt><dd>{resource.metadata?.name ?? 'N/A'}</dd>
            <dt>Namespace</dt><dd>{resource.metadata?.namespace ?? 'N/A'}</dd>
            <dt>Created</dt><dd>{resource.metadata?.creationTimestamp ?? 'N/A'}</dd>
          </dl>
        {/if}

        <!-- Common: Labels -->
        <h3>Labels</h3>
        {#if resource.metadata?.labels && Object.keys(resource.metadata.labels).length > 0}
          <div class="labels">
            {#each Object.entries(resource.metadata.labels) as [key, value]}
              <span class="label-badge">{key}={value}</span>
            {/each}
          </div>
        {:else}
          <p class="muted">No labels</p>
        {/if}

        <!-- Common: Annotations -->
        <h3>Annotations</h3>
        {#if resource.metadata?.annotations && Object.keys(resource.metadata.annotations).length > 0}
          <div class="labels">
            {#each Object.entries(resource.metadata.annotations) as [key, value]}
              <span class="label-badge">{key}={truncate(String(value), 60)}</span>
            {/each}
          </div>
        {:else}
          <p class="muted">No annotations</p>
        {/if}

        <!-- Azure Identity (workload kinds use pod template spec) -->
        {#if WORKLOAD_KINDS.has(kind) && resource.spec?.template}
          <AzureIdentitySection pod={resource.spec.template} />
        {/if}

        <!-- Common: Conditions (for kinds that have them) -->
        {#if resource.status?.conditions?.length}
          <h3>Conditions</h3>
          <table>
            <thead><tr><th scope="col">Type</th><th scope="col">Status</th><th scope="col">Reason</th><th scope="col">Message</th></tr></thead>
            <tbody>
              {#each resource.status.conditions as cond}
                <tr>
                  <td>{cond.type}</td>
                  <td class={cond.status === 'True' ? 'status-ok' : 'status-bad'}>{cond.status}</td>
                  <td>{cond.reason ?? ''}</td>
                  <td class="message">{cond.message ?? ''}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

    {:else if activeTab === 'pods'}
      <div class="pods-tab">
        {#if ownedPods.length === 0}
          <p class="muted">No pods found matching selector labels.</p>
        {:else}
          <p class="count">{ownedPods.length} pod{ownedPods.length !== 1 ? 's' : ''}</p>
          <table aria-label="Owned pods">
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col">Status</th>
                <th scope="col">Restarts</th>
                <th scope="col">Age</th>
              </tr>
            </thead>
            <tbody>
              {#each ownedPods as pod}
                <tr>
                  <td class="resource-name">
                    <a href="/pods/{pod.namespace}/{pod.name}">{pod.name}</a>
                  </td>
                  <td class={podPhase(pod) === 'Running' ? 'status-ok' : podPhase(pod) === 'Failed' ? 'status-bad' : ''}>
                    {podPhase(pod)}
                  </td>
                  <td>{podRestarts(pod)}</td>
                  <td>{podAge(pod)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

    {:else if activeTab === 'events'}
      <EventsTable events={events} showObject={false} />

    {:else if activeTab === 'yaml'}
      <div class="yaml-tab">
        <div class="yaml-actions">
          <button onclick={() => handleApply(true)} disabled={!yamlDirty || applying} class="action-btn">🧪 Dry Run</button>
          <button onclick={() => handleApply(false)} disabled={!yamlDirty || applying} class="action-btn primary">✅ Apply</button>
          <button onclick={resetYaml} disabled={!yamlDirty} class="action-btn">↩ Reset</button>
          {#if applyMessage}<span class={applyError ? 'apply-error' : 'apply-success'}>{applyMessage}</span>{/if}
        </div>
        <YamlEditor content={yamlContent} onchange={(v) => editedYaml = v} />
      </div>
    {/if}
  {/if}
</div>

<ScaleDialog
  open={showScale}
  resourceName={resourceName}
  currentReplicas={currentReplicas}
  onscale={handleScale}
  oncancel={() => showScale = false}
/>

<ConfirmDialog
  open={showScaleConfirm}
  title="Scale in Production"
  message={`You are about to scale "${resourceName}" to ${pendingScaleReplicas} replica(s) in a PRODUCTION context.`}
  confirmText="Scale"
  confirmValue={resourceName}
  requireType={true}
  productionContext={true}
  onconfirm={confirmScale}
  oncancel={() => { showScaleConfirm = false; pendingScaleReplicas = null; }}
/>

<ConfirmDialog
  open={showProdRestartConfirm}
  title="Rollout Restart in Production"
  message={`You are about to restart all pods for "${resourceName}" in a PRODUCTION context.`}
  confirmText="Restart"
  confirmValue={resourceName}
  requireType={true}
  productionContext={true}
  onconfirm={handleRolloutRestart}
  oncancel={() => showProdRestartConfirm = false}
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

  .error {
    color: #ef5350;
    background: rgba(239, 83, 80, 0.1);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    border: 1px solid rgba(239, 83, 80, 0.3);
  }

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

  .muted { color: #6e7681; font-size: 0.875rem; }
  .count { color: #9e9e9e; margin-bottom: 0.5rem; font-size: 0.875rem; }

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

  .resource-name {
    font-weight: 500;
    color: #4fc3f7;
  }
  .resource-name a {
    color: #4fc3f7;
    text-decoration: none;
  }
  .resource-name a:hover {
    text-decoration: underline;
    color: #58a6ff;
  }

  .status-ok { color: #66bb6a; }
  .status-bad { color: #ef5350; }

  .message {
    max-width: 360px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .yaml-tab { margin-top: 0.5rem; }
  .yaml-actions { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
  .action-btn { background: #21262d; color: #c9d1d9; border: 1px solid #30363d; padding: 0.35rem 0.75rem; border-radius: 6px; font-size: 0.8rem; cursor: pointer; }
  .action-btn:hover:not(:disabled) { background: #30363d; }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-btn.primary { background: #238636; border-color: #2ea043; }
  .action-btn.primary:hover:not(:disabled) { background: #2ea043; }
  .apply-success { color: #66bb6a; font-size: 0.8rem; }
  .apply-error { color: #ef5350; font-size: 0.8rem; }

  .pods-tab {
    margin-top: 0.5rem;
  }

  .rollout-section { margin-top: 0.5rem; }
  .rollout-actions { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.5rem; }
  .confirm-prompt { color: #fbbf24; font-weight: 500; }
  .rollout-message { margin-top: 0.5rem; color: #a0aec0; font-size: 0.85rem; }
  .badge { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: 600; margin-right: 0.5rem; }
  .badge-success { background: #065f46; color: #6ee7b7; }
  .badge-pending { background: #92400e; color: #fcd34d; }
  .btn { border: none; padding: 0.35rem 0.75rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; font-weight: 500; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary { background: #2563eb; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #1d4ed8; }
  .btn-danger { background: #dc2626; color: #fff; }
  .btn-danger:hover:not(:disabled) { background: #b91c1c; }
  .btn-secondary { background: #374151; color: #e0e0e0; }
  .btn-secondary:hover:not(:disabled) { background: #4b5563; }
  .btn-sm { padding: 0.25rem 0.6rem; font-size: 0.75rem; }
</style>
