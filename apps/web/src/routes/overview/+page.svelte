<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    activeContext,
    getAksClusterDetail,
    getAzureCloud,
    getClusterInfo,
    getEvents,
    getPodMetrics,
    getPods,
    getResourceCounts,
    resolveAksIdentity,
  } from '$lib/api';
  import { getAutoRefreshIntervalMs } from '$lib/preferences';
  import { selectedContext, selectedNamespace, isConnected, isAks } from '$lib/stores';
  import { PORTAL_BLADES, getAzurePortalUrl } from '$lib/azure-utils';
  import AksAddons from '$lib/components/AksAddons.svelte';
  import ErrorMessage from '$lib/components/ErrorMessage.svelte';
  import type { AksClusterDetail, AksIdentityInfo, ClusterInfo, ResourceEntry } from '$lib/tauri-commands';

  // Resource counts keyed by GVK
  let counts: Map<string, number> = $state(new Map());
  let pods: ResourceEntry[] = $state([]);
  let warningEvents: WarningEvent[] = $state([]);
  let contextName: string | null = $state(null);
  let loading = $state(true);
  let refreshError = $state(false);
  let lastSuccessfulRefresh: number | null = $state(null);
  let timer: ReturnType<typeof setInterval> | null = $state(null);
  let namespaceTimer: ReturnType<typeof setInterval> | null = $state(null);
  let clusterInfo: ClusterInfo | null = $state(null);

  // Namespace resource usage (Feature: #74)
  interface NamespaceUsage {
    namespace: string;
    pods: number;
    cpuMillicores: number;
    memoryBytes: number;
  }

  let namespaceUsage: NamespaceUsage[] = $state([]);
  let aksIdentity: AksIdentityInfo | null = $state(null);
  let aksDetail: AksClusterDetail | null = $state(null);
  let azureCloud = $state('Commercial');

  let staleData = $derived(
    lastSuccessfulRefresh !== null && Date.now() - lastSuccessfulRefresh > 30_000
  );
  let hasAksPortalIdentity = $derived(
    (clusterInfo?.is_aks || $isAks) &&
      !!aksIdentity?.subscription_id &&
      !!aksIdentity?.resource_group &&
      !!aksIdentity?.cluster_name
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

  let destroyed = false;

  onMount(() => {
    refresh();
    // Fetch cluster info once (not on every poll cycle).
    void Promise.all([getClusterInfo(), resolveAksIdentity(), getAzureCloud()]).then(
      ([info, identity, cloud]) => {
        clusterInfo = info;
        aksIdentity = identity;
        azureCloud = cloud;
        // Fetch ARM cluster detail after identity is resolved.
        if (identity && (info?.is_aks)) {
          getAksClusterDetail().then((detail) => { aksDetail = detail; });
        }
      },
    );
    // Fetch namespace usage once and then every 30s
    refreshNamespaceUsage();
    namespaceTimer = setInterval(refreshNamespaceUsage, 30_000);
    void (async () => {
      const refreshIntervalMs = await getAutoRefreshIntervalMs(5000);
      if (!destroyed) {
        timer = setInterval(refresh, refreshIntervalMs);
      }
    })();
  });

  onDestroy(() => {
    destroyed = true;
    if (timer) clearInterval(timer);
    if (namespaceTimer) clearInterval(namespaceTimer);
  });

  async function refreshNamespaceUsage() {
    try {
      const metrics = await getPodMetrics();
      const nsMap = new Map<string, NamespaceUsage>();
      for (const pod of metrics) {
        const ns = pod.namespace || 'default';
        const entry = nsMap.get(ns) ?? { namespace: ns, pods: 0, cpuMillicores: 0, memoryBytes: 0 };
        entry.pods++;
        entry.cpuMillicores += pod.cpu_millicores;
        entry.memoryBytes += pod.memory_bytes;
        nsMap.set(ns, entry);
      }
      namespaceUsage = [...nsMap.values()]
        .sort((a, b) => b.cpuMillicores - a.cpuMillicores)
        .slice(0, 5);
    } catch {
      // Metrics not available — skip
    }
  }

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

  function getPortalUrl(blade?: string): string | null {
    return aksIdentity ? getAzurePortalUrl(aksIdentity, azureCloud, blade) : null;
  }

  const azurePortalSections = [
    { label: 'Overview', description: 'Cluster summary and health', blade: PORTAL_BLADES.overview },
    { label: 'Node Pools', description: 'Scaling and pool configuration', blade: PORTAL_BLADES.nodePools },
    { label: 'Upgrades', description: 'Available Kubernetes version upgrades', blade: PORTAL_BLADES.upgrade },
    { label: 'Networking', description: 'Load balancers, IPs, and CNI settings', blade: PORTAL_BLADES.networking },
    { label: 'Monitoring', description: 'Container insights and observability', blade: PORTAL_BLADES.monitoring },
    { label: 'Activity Log', description: 'Recent Azure management operations', blade: PORTAL_BLADES.activityLog },
  ] as const;

  function openInPortal(blade = PORTAL_BLADES.overview) {
    const portalUrl = getPortalUrl(blade);
    if (portalUrl) {
      window.open(portalUrl, '_blank', 'noopener');
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
      <ErrorMessage
        message="Data may be stale — last successful refresh was more than 30 seconds ago."
        suggestion="The cluster connection may be unstable. Check your network."
        onretry={refresh}
      />
    {:else if refreshError}
      <ErrorMessage
        message="Failed to refresh data. Retrying…"
        onretry={refresh}
      />
    {/if}

    <!-- Cluster info -->
    <section class="cluster-info" aria-label="Cluster information">
      <div class="info-item">
        <span class="info-label">Context</span>
        <span class="info-value">
          {contextName ?? '—'}
          {#if clusterInfo?.is_aks || $isAks}
            <span class="aks-badge" title="Azure Kubernetes Service">AKS</span>
          {/if}
        </span>
      </div>
      {#if clusterInfo?.server_version}
        <div class="info-item">
          <span class="info-label">K8s Version</span>
          <span class="info-value">{clusterInfo.server_version}</span>
        </div>
      {/if}
      <div class="info-item">
        <span class="info-label">Namespace</span>
        <span class="info-value">{$selectedNamespace}</span>
      </div>
      {#if clusterInfo?.auth_hint}
        <div class="info-item">
          <span class="info-label">Auth</span>
          <span class="info-value auth-hint">
            {#if clusterInfo.is_aks}
              <span class="entra-icon" title="Microsoft Entra ID">🔐</span>
            {/if}
            {clusterInfo.auth_hint}
          </span>
        </div>
      {/if}
      {#if hasAksPortalIdentity}
        <div class="info-item portal-link">
          <button class="portal-btn" onclick={() => openInPortal()} title="Open cluster in Azure Portal">
            🌐 Open in Azure Portal
          </button>
        </div>
      {/if}
    </section>

    {#if hasAksPortalIdentity}
      <section aria-label="Azure management">
        <div class="section-heading">
          <h2>Azure</h2>
          <span class="section-subtitle">Portal deep links for this AKS cluster</span>
        </div>
        <div class="azure-grid">
          {#each azurePortalSections as section}
            {@const href = getPortalUrl(section.blade)}
            {#if href}
              <a class="azure-card" href={href} target="_blank" rel="noopener">
                <div class="azure-card-header">
                  <span>{section.label}</span>
                  <span class="azure-card-action">View in Portal ↗</span>
                </div>
                <p>{section.description}</p>
              </a>
            {/if}
          {/each}
        </div>
      </section>
    {/if}

    {#if aksDetail}
      <section aria-label="AKS cluster details" data-testid="aks-cluster-detail">
        <div class="section-heading">
          <h2>AKS Cluster Details</h2>
          <span class="section-subtitle">Live data from Azure Resource Manager</span>
        </div>
        <div class="detail-grid">
          <!-- Azure Info -->
          <div class="detail-card">
            <h3>Azure Info</h3>
            <dl class="detail-list">
              {#if aksIdentity?.subscription_id}
                <dt>Subscription</dt>
                <dd class="mono">{aksIdentity.subscription_id}</dd>
              {/if}
              {#if aksIdentity?.resource_group}
                <dt>Resource Group</dt>
                <dd>{aksIdentity.resource_group}</dd>
              {/if}
              {#if aksDetail.sku?.tier}
                <dt>SKU Tier</dt>
                <dd>
                  <span class="tier-badge tier-{aksDetail.sku.tier.toLowerCase()}">{aksDetail.sku.tier}</span>
                </dd>
              {/if}
              {#if aksDetail.provisioningState}
                <dt>Provisioning</dt>
                <dd>
                  <span class="state-badge state-{aksDetail.provisioningState.toLowerCase()}">{aksDetail.provisioningState}</span>
                </dd>
              {/if}
              {#if aksDetail.powerState?.code}
                <dt>Power State</dt>
                <dd>
                  <span class="power-badge power-{aksDetail.powerState.code.toLowerCase()}">{aksDetail.powerState.code}</span>
                </dd>
              {/if}
            </dl>
          </div>

          <!-- Network -->
          {#if aksDetail.networkProfile}
            <div class="detail-card">
              <h3>Network</h3>
              <dl class="detail-list">
                {#if aksDetail.networkProfile.networkPlugin}
                  <dt>Plugin</dt>
                  <dd>{aksDetail.networkProfile.networkPlugin}</dd>
                {/if}
                {#if aksDetail.networkProfile.networkPolicy}
                  <dt>Policy</dt>
                  <dd>{aksDetail.networkProfile.networkPolicy}</dd>
                {/if}
                {#if aksDetail.networkProfile.serviceCidr}
                  <dt>Service CIDR</dt>
                  <dd class="mono">{aksDetail.networkProfile.serviceCidr}</dd>
                {/if}
                {#if aksDetail.networkProfile.podCidr}
                  <dt>Pod CIDR</dt>
                  <dd class="mono">{aksDetail.networkProfile.podCidr}</dd>
                {/if}
                {#if aksDetail.networkProfile.dnsServiceIp}
                  <dt>DNS Service IP</dt>
                  <dd class="mono">{aksDetail.networkProfile.dnsServiceIp}</dd>
                {/if}
                {#if aksDetail.networkProfile.outboundType}
                  <dt>Outbound</dt>
                  <dd>{aksDetail.networkProfile.outboundType}</dd>
                {/if}
                {#if aksDetail.networkProfile.loadBalancerSku}
                  <dt>LB SKU</dt>
                  <dd>{aksDetail.networkProfile.loadBalancerSku}</dd>
                {/if}
              </dl>
            </div>
          {/if}

          <!-- API Server -->
          {#if aksDetail.apiServerAccessProfile}
            <div class="detail-card">
              <h3>API Server</h3>
              <dl class="detail-list">
                <dt>Access</dt>
                <dd>
                  {#if aksDetail.apiServerAccessProfile.enablePrivateCluster}
                    <span class="access-badge private">🔒 Private</span>
                  {:else}
                    <span class="access-badge public">🌐 Public</span>
                  {/if}
                </dd>
                {#if aksDetail.apiServerAccessProfile.authorizedIpRanges?.length}
                  <dt>Authorized IPs</dt>
                  <dd class="mono ip-list">
                    {#each aksDetail.apiServerAccessProfile.authorizedIpRanges as range}
                      <span class="ip-range">{range}</span>
                    {/each}
                  </dd>
                {/if}
              </dl>
            </div>
          {/if}

          <!-- Identity -->
          {#if aksDetail.identity}
            <div class="detail-card">
              <h3>Identity</h3>
              <dl class="detail-list">
                {#if aksDetail.identity.type_}
                  <dt>Type</dt>
                  <dd>{aksDetail.identity.type_}</dd>
                {/if}
                {#if aksDetail.identity.principalId}
                  <dt>Principal ID</dt>
                  <dd class="mono truncate" title={aksDetail.identity.principalId}>{aksDetail.identity.principalId}</dd>
                {/if}
                {#if aksDetail.identity.tenantId}
                  <dt>Tenant ID</dt>
                  <dd class="mono truncate" title={aksDetail.identity.tenantId}>{aksDetail.identity.tenantId}</dd>
                {/if}
              </dl>
            </div>
          {/if}

          <!-- Upgrade -->
          <div class="detail-card">
            <h3>Upgrades</h3>
            <dl class="detail-list">
              {#if aksDetail.kubernetesVersion}
                <dt>K8s Version</dt>
                <dd>{aksDetail.kubernetesVersion}</dd>
              {/if}
              {#if aksDetail.autoUpgradeProfile?.upgradeChannel}
                <dt>Auto-Upgrade</dt>
                <dd>{aksDetail.autoUpgradeProfile.upgradeChannel}</dd>
              {/if}
              {#if aksDetail.autoUpgradeProfile?.nodeOsUpgradeChannel}
                <dt>Node OS Channel</dt>
                <dd>{aksDetail.autoUpgradeProfile.nodeOsUpgradeChannel}</dd>
              {/if}
            </dl>
          </div>

          <!-- Security / OIDC -->
          {#if aksDetail.oidcIssuerProfile?.enabled || aksDetail.securityProfile?.workloadIdentity?.enabled}
            <div class="detail-card">
              <h3>Security</h3>
              <dl class="detail-list">
                {#if aksDetail.oidcIssuerProfile?.enabled}
                  <dt>OIDC Issuer</dt>
                  <dd>
                    <span class="enabled-badge">✓ Enabled</span>
                  </dd>
                {/if}
                {#if aksDetail.securityProfile?.workloadIdentity?.enabled}
                  <dt>Workload Identity</dt>
                  <dd>
                    <span class="enabled-badge">✓ Enabled</span>
                  </dd>
                {/if}
              </dl>
            </div>
          {/if}
        </div>
      </section>
    {/if}

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
    {#if clusterInfo?.is_aks || $isAks}
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
    {#if namespaceUsage.length > 0}
      <section aria-label="Top namespaces by resource usage">
        <h2>Top Namespaces by Usage</h2>
        <div class="events-table-wrap">
          <table class="events-table">
            <thead>
              <tr>
                <th>Namespace</th>
                <th>Pods</th>
                <th>CPU (millicores)</th>
                <th>Memory (MiB)</th>
              </tr>
            </thead>
            <tbody>
              {#each namespaceUsage as ns}
                <tr>
                  <td class="cell-object">{ns.namespace}</td>
                  <td>{ns.pods}</td>
                  <td>{ns.cpuMillicores.toFixed(0)}</td>
                  <td>{(ns.memoryBytes / (1024 * 1024)).toFixed(1)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}

    <!-- Recent warning events (original) -->
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
  .auth-hint {
    font-size: 0.8rem;
    color: #8b949e;
  }
  .entra-icon {
    margin-right: 0.15rem;
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
  .section-heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
  }
  .section-subtitle {
    color: #8b949e;
    font-size: 0.8rem;
  }
  .azure-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
  }
  .azure-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.9rem 1rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    text-decoration: none;
    color: #e0e0e0;
    transition: border-color 0.15s, background 0.15s;
  }
  .azure-card:hover {
    border-color: #58a6ff;
    background: #1a2332;
  }
  .azure-card-header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    font-weight: 600;
  }
  .azure-card-action {
    color: #58a6ff;
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .azure-card p {
    margin: 0;
    color: #8b949e;
    font-size: 0.8rem;
    line-height: 1.4;
  }

  /* AKS detail grid */
  .detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 0.75rem;
  }
  .detail-card {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    padding: 0.9rem 1rem;
  }
  .detail-card h3 {
    margin: 0 0 0.6rem;
    font-size: 0.85rem;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .detail-list {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.3rem 0.75rem;
    margin: 0;
  }
  .detail-list dt {
    font-size: 0.78rem;
    color: #484f58;
    white-space: nowrap;
  }
  .detail-list dd {
    font-size: 0.82rem;
    color: #e0e0e0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .detail-list dd.mono {
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', monospace;
    font-size: 0.78rem;
  }
  .detail-list dd.truncate {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tier-badge {
    display: inline-block;
    padding: 0.05rem 0.35rem;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 600;
  }
  .tier-badge.tier-free { background: rgba(139, 148, 158, 0.2); color: #8b949e; }
  .tier-badge.tier-standard { background: rgba(0, 120, 212, 0.2); color: #58a6ff; }
  .tier-badge.tier-premium { background: rgba(130, 80, 223, 0.2); color: #a371f7; }
  .state-badge {
    display: inline-block;
    padding: 0.05rem 0.35rem;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 500;
  }
  .state-badge.state-succeeded { background: rgba(102, 187, 106, 0.2); color: #66bb6a; }
  .state-badge.state-failed { background: rgba(239, 83, 80, 0.2); color: #ef5350; }
  .state-badge.state-creating,
  .state-badge.state-updating { background: rgba(255, 167, 38, 0.2); color: #ffa726; }
  .power-badge {
    display: inline-block;
    padding: 0.05rem 0.35rem;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 500;
  }
  .power-badge.power-running { background: rgba(102, 187, 106, 0.2); color: #66bb6a; }
  .power-badge.power-stopped { background: rgba(139, 148, 158, 0.2); color: #8b949e; }
  .access-badge {
    font-size: 0.78rem;
    font-weight: 500;
  }
  .access-badge.private { color: #ffa726; }
  .access-badge.public { color: #58a6ff; }
  .ip-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .ip-range {
    background: #21262d;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-size: 0.72rem;
  }
  .enabled-badge {
    color: #66bb6a;
    font-size: 0.78rem;
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
