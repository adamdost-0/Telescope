<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    activeContext,
    disconnect,
    getAksClusterDetail,
    getAksUpgradeProfile,
    getClusterInfo,
    getEvents,
    getPodMetrics,
    getPods,
    getResourceCounts,
    listAksMaintenanceConfigs,
    resolveAksIdentity,
    startAksCluster,
    stopAksCluster,
    upgradeAksCluster,
  } from '$lib/api';
  import { formatBinaryBytes, formatCpuMillicores } from '$lib/metrics-format';
  import { getAutoRefreshIntervalMs } from '$lib/preferences';
  import { selectedContext, selectedNamespace, isConnected, isAks } from '$lib/stores';
  import ErrorMessage from '$lib/components/ErrorMessage.svelte';
  import type {
    AksClusterDetail,
    AksIdentityInfo,
    AksMaintenanceConfig,
    AksUpgradeProfile,
    AvailableUpgrade,
    ClusterInfo,
    ResourceEntry,
  } from '$lib/tauri-commands';

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
  let aksUpgradeProfile: AksUpgradeProfile | null = $state(null);
  let aksMaintenanceConfigs: AksMaintenanceConfig[] = $state([]);

  // Diagnostics derived state (#139)
  let diagNoNetworkPolicy = $derived(
    aksDetail?.networkProfile != null && !aksDetail.networkProfile.networkPolicy
  );
  let diagPublicNoIpRestriction = $derived(
    aksDetail?.apiServerAccessProfile != null &&
    !aksDetail.apiServerAccessProfile.enablePrivateCluster &&
    (!aksDetail.apiServerAccessProfile.authorizedIpRanges || aksDetail.apiServerAccessProfile.authorizedIpRanges.length === 0)
  );
  let diagNoAutoUpgrade = $derived(
    aksDetail != null &&
    (!aksDetail.autoUpgradeProfile?.upgradeChannel || aksDetail.autoUpgradeProfile.upgradeChannel === 'none')
  );
  let aksActionError: string | null = $state(null);
  let aksActionMessage: string | null = $state(null);
  let showPowerDialog = $state(false);
  let powerAction: 'start' | 'stop' | null = $state(null);
  let showUpgradeDialog = $state(false);
  let selectedUpgrade: AvailableUpgrade | null = $state(null);
  let upgradeConfirmationText = $state('');
  let previewAcknowledged = $state(false);
  let clusterUpgradePending = $state(false);
  let aksPollingTimer: ReturnType<typeof setInterval> | null = $state(null);

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
  let currentPowerState = $derived.by(() => {
    if (powerAction === 'start') return 'Starting';
    if (powerAction === 'stop') return 'Stopping';
    return aksDetail?.powerState?.code ?? 'Unknown';
  });
  let currentProvisioningState = $derived.by(() => {
    if (clusterUpgradePending) return 'Upgrading';
    return aksDetail?.provisioningState ?? 'Unknown';
  });
  let clusterUpgradeTargets = $derived(aksUpgradeProfile?.upgrades ?? []);
  let upgradeGuardText = $derived(aksIdentity?.cluster_name ?? contextName ?? 'cluster');

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

  async function refreshAksManagementData() {
    if (!(aksIdentity && (clusterInfo?.is_aks || $isAks))) {
      aksDetail = null;
      aksUpgradeProfile = null;
      aksMaintenanceConfigs = [];
      return;
    }

    const [detail, profile, configs] = await Promise.allSettled([
      getAksClusterDetail(),
      getAksUpgradeProfile(),
      listAksMaintenanceConfigs(),
    ]);

    if (detail.status === 'fulfilled') {
      aksDetail = detail.value;
    }
    if (profile.status === 'fulfilled') {
      aksUpgradeProfile = profile.value;
    }
    if (configs.status === 'fulfilled') {
      aksMaintenanceConfigs = configs.value;
    }
  }

  function powerBadgeVariant(powerState: string): string {
    const normalized = powerState.toLowerCase();
    if (normalized === 'running') return 'power-running';
    if (normalized === 'stopped') return 'power-stopped';
    if (normalized === 'starting' || normalized === 'stopping') return 'power-transitioning';
    return 'power-unknown';
  }

  function openPowerConfirmation() {
    aksActionError = null;
    showPowerDialog = true;
  }

  function beginAksPolling() {
    if (aksPollingTimer) clearInterval(aksPollingTimer);
    aksPollingTimer = setInterval(() => {
      void refreshAksManagementData();
    }, 10_000);
  }

  function stopAksPolling() {
    if (aksPollingTimer) {
      clearInterval(aksPollingTimer);
      aksPollingTimer = null;
    }
  }

  function openUpgradeDialog(upgrade: AvailableUpgrade) {
    aksActionError = null;
    selectedUpgrade = upgrade;
    upgradeConfirmationText = '';
    previewAcknowledged = false;
    showUpgradeDialog = true;
  }

  async function handlePowerAction() {
    const action = currentPowerState.toLowerCase() === 'running' ? 'stop' : 'start';
    aksActionError = null;
    aksActionMessage = null;
    powerAction = action;
    showPowerDialog = false;
    beginAksPolling();
    try {
      if (action === 'stop') {
        await stopAksCluster();
        aksActionMessage = 'Cluster stopped. Compute billing is paused and the Kubernetes session was disconnected.';
        await refreshAksManagementData();
        await disconnect();
      } else {
        await startAksCluster();
        aksActionMessage = 'Cluster started. Kubernetes API recovery can take a few minutes.';
        await refreshAksManagementData();
      }
    } catch (e) {
      aksActionError = e instanceof Error ? e.message : `Failed to ${action} cluster`;
    } finally {
      powerAction = null;
      stopAksPolling();
    }
  }

  async function handleClusterUpgrade() {
    if (!selectedUpgrade) return;
    aksActionError = null;
    aksActionMessage = null;
    clusterUpgradePending = true;
    showUpgradeDialog = false;
    beginAksPolling();
    try {
      await upgradeAksCluster(selectedUpgrade.kubernetesVersion);
      aksActionMessage = `Cluster upgraded to ${selectedUpgrade.kubernetesVersion}.`;
      await refreshAksManagementData();
      clusterInfo = await getClusterInfo();
    } catch (e) {
      aksActionError = e instanceof Error ? e.message : 'Failed to upgrade cluster';
    } finally {
      clusterUpgradePending = false;
      selectedUpgrade = null;
      upgradeConfirmationText = '';
      previewAcknowledged = false;
      stopAksPolling();
    }
  }

  function canConfirmUpgrade(): boolean {
    if (!selectedUpgrade) return false;
    if (selectedUpgrade.isPreview && !previewAcknowledged) return false;
    return upgradeConfirmationText.trim() === upgradeGuardText;
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
    void Promise.all([getClusterInfo(), resolveAksIdentity()]).then(
      ([info, identity]) => {
        clusterInfo = info;
        aksIdentity = identity;
        if (identity && info?.is_aks) {
          void refreshAksManagementData();
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
    stopAksPolling();
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

  import Icon from '$lib/icons/Icon.svelte';
  import type { ResourceIconName } from '$lib/icons';

  interface CardDef {
    label: string;
    gvk: string;
    icon: ResourceIconName;
    href: string;
  }

  const cards: CardDef[] = [
    { label: 'Pods', gvk: 'v1/Pod', icon: 'pods', href: '/pods' },
    { label: 'Deployments', gvk: 'apps/v1/Deployment', icon: 'deployments', href: '/resources/deployments' },
    { label: 'StatefulSets', gvk: 'apps/v1/StatefulSet', icon: 'statefulsets', href: '/resources/statefulsets' },
    { label: 'DaemonSets', gvk: 'apps/v1/DaemonSet', icon: 'daemonsets', href: '/resources/daemonsets' },
    { label: 'Jobs', gvk: 'batch/v1/Job', icon: 'jobs', href: '/resources/jobs' },
    { label: 'CronJobs', gvk: 'batch/v1/CronJob', icon: 'cronjobs', href: '/resources/cronjobs' },
    { label: 'Services', gvk: 'v1/Service', icon: 'services', href: '/resources/services' },
    { label: 'ConfigMaps', gvk: 'v1/ConfigMap', icon: 'configmaps', href: '/resources/configmaps' },
    { label: 'Secrets', gvk: 'v1/Secret', icon: 'secrets', href: '/resources/secrets' },
    { label: 'Nodes', gvk: 'v1/Node', icon: 'nodes', href: '/nodes' },
    { label: 'Events', gvk: 'v1/Event', icon: 'events', href: '/events' },
  ];

  const phaseColors: Record<string, string> = {
    Running: '#66bb6a',
    Pending: '#ffa726',
    Failed: '#ef5350',
    Succeeded: '#42a5f5',
    Unknown: '#757575',
  };
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
              <span class="entra-icon" title="Microsoft Entra ID" aria-hidden="true">
                <Icon name="auth-oidc" size={14} />
              </span>
            {/if}
            {clusterInfo.auth_hint}
          </span>
        </div>
      {/if}
    </section>

    {#if aksActionMessage}
      <div class="action-banner success" role="status">{aksActionMessage}</div>
    {/if}
    {#if aksActionError}
      <div class="action-banner error" role="alert">{aksActionError}</div>
    {/if}

    {#if aksDetail}
      <section aria-label="AKS lifecycle and upgrades">
        <div class="section-heading">
          <h2>AKS Lifecycle</h2>
          <span class="section-subtitle">Manage cluster power state and control plane upgrades</span>
        </div>
        <div class="lifecycle-grid">
          <div class="detail-card">
            <h3>Cluster Power</h3>
            <p class="card-copy">
              Stop the cluster to pause compute billing, or start it again when you need Kubernetes API access.
            </p>
            <div class="lifecycle-row">
              <span class={`power-badge ${powerBadgeVariant(currentPowerState)}`}>{currentPowerState}</span>
              <button class="primary-action" disabled={!!powerAction || clusterUpgradePending} onclick={openPowerConfirmation}>
                {powerAction === 'stop'
                  ? 'Stopping…'
                  : powerAction === 'start'
                    ? 'Starting…'
                    : currentPowerState.toLowerCase() === 'running'
                      ? 'Stop Cluster'
                      : 'Start Cluster'}
              </button>
            </div>
            <p class="hint">
              {#if currentPowerState.toLowerCase() === 'stopped'}
                Cluster is stopped — compute billing paused.
              {:else if currentPowerState.toLowerCase() === 'running'}
                Stopping disconnects the current Kubernetes session once power-off completes.
              {:else}
                Azure is processing a power-state transition.
              {/if}
            </p>
          </div>

          <div class="detail-card">
            <h3>Cluster Upgrades</h3>
            <div class="upgrade-summary">
              <div>
                <span class="details-label">Current Version</span>
                <div class="upgrade-current">{aksUpgradeProfile?.currentVersion ?? aksDetail.kubernetesVersion ?? '—'}</div>
              </div>
              <div>
                <span class="details-label">Provisioning</span>
                <span class={`state-badge state-${currentProvisioningState.toLowerCase()}`}>{currentProvisioningState}</span>
              </div>
            </div>
            {#if clusterUpgradeTargets.length > 0}
              <div class="upgrade-targets">
                {#each clusterUpgradeTargets as upgrade}
                  <button class="upgrade-target" disabled={clusterUpgradePending || !!powerAction} onclick={() => openUpgradeDialog(upgrade)}>
                    <span>{upgrade.kubernetesVersion}</span>
                    {#if upgrade.isPreview}
                      <span class="preview-pill">Preview</span>
                    {/if}
                  </button>
                {/each}
              </div>
              <p class="hint">
                {#if aksMaintenanceConfigs.length > 0}
                  {aksMaintenanceConfigs.length} maintenance configuration{aksMaintenanceConfigs.length === 1 ? '' : 's'} defined for this cluster.
                {:else}
                  No ARM maintenance windows reported for this cluster.
                {/if}
              </p>
            {:else}
              <p class="hint">No newer Kubernetes versions are currently offered by Azure for this cluster.</p>
            {/if}
          </div>
        </div>
      </section>

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
              {#if currentProvisioningState}
                <dt>Provisioning</dt>
                <dd>
                  <span class={`state-badge state-${currentProvisioningState.toLowerCase()}`}>{currentProvisioningState}</span>
                </dd>
              {/if}
              {#if currentPowerState}
                <dt>Power State</dt>
                <dd>
                  <span class={`power-badge ${powerBadgeVariant(currentPowerState)}`}>{currentPowerState}</span>
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
                    <span class="access-badge private">
                      <Icon name="auth-cert" size={14} aria-hidden="true" /> Private
                    </span>
                  {:else}
                    <span class="access-badge public">
                      <Icon name="services" size={14} aria-hidden="true" /> Public
                    </span>
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

          <!-- Identity (#132: Managed Identity Awareness) -->
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
                {#if aksDetail.identityProfile?.kubeletidentity?.clientId}
                  <dt>Kubelet Client ID</dt>
                  <dd class="mono truncate" title={aksDetail.identityProfile.kubeletidentity.clientId}>{aksDetail.identityProfile.kubeletidentity.clientId}</dd>
                {/if}
              </dl>
            </div>
          {/if}

          <!-- Security / OIDC (#132: Workload Identity) -->
          {#if aksDetail.oidcIssuerProfile?.enabled || aksDetail.securityProfile?.workloadIdentity?.enabled}
            <div class="detail-card">
              <h3>Workload Identity</h3>
              <dl class="detail-list">
                {#if aksDetail.securityProfile?.workloadIdentity?.enabled}
                  <dt>Workload Identity</dt>
                  <dd>
                    <span class="enabled-badge">✓ Enabled</span>
                  </dd>
                {:else}
                  <dt>Workload Identity</dt>
                  <dd>
                    <span class="disabled-badge">— Disabled</span>
                  </dd>
                {/if}
                {#if aksDetail.oidcIssuerProfile?.enabled}
                  <dt>OIDC Issuer</dt>
                  <dd>
                    <span class="enabled-badge">✓ Enabled</span>
                  </dd>
                {/if}
                {#if aksDetail.oidcIssuerProfile?.issuerUrl}
                  <dt>Issuer URL</dt>
                  <dd class="mono truncate" title={aksDetail.oidcIssuerProfile.issuerUrl}>{aksDetail.oidcIssuerProfile.issuerUrl}</dd>
                {/if}
              </dl>
              <p class="detail-hint">Pod-level Workload Identity bindings are shown on individual pod detail views.</p>
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
              {:else}
                <dt>Auto-Upgrade</dt>
                <dd>
                  <span class="info-notice">
                    <Icon name="diagnostic-info" size={14} aria-hidden="true" /> Disabled
                  </span>
                </dd>
              {/if}
              {#if aksDetail.autoUpgradeProfile?.nodeOsUpgradeChannel}
                <dt>Node OS Channel</dt>
                <dd>{aksDetail.autoUpgradeProfile.nodeOsUpgradeChannel}</dd>
              {/if}
            </dl>
          </div>

          <!-- Maintenance Windows (#139) -->
          {#if aksMaintenanceConfigs.length > 0}
            <div class="detail-card">
              <h3>Maintenance Windows</h3>
              {#each aksMaintenanceConfigs as config}
                <div class="maintenance-config">
                  <span class="maintenance-name">{config.name}</span>
                  {#if config.timeInWeek.length > 0}
                    <div class="maintenance-slots">
                      {#each config.timeInWeek as slot}
                        {#if slot.day}
                          <span class="maintenance-slot">{slot.day}{#if slot.hourSlots?.length} {slot.hourSlots.join(', ')}h{/if}</span>
                        {/if}
                      {/each}
                    </div>
                  {/if}
                  {#if config.notAllowedTime.length > 0}
                    <div class="maintenance-blocked">
                      <span class="blocked-label">
                        <Icon name="error" size={14} aria-hidden="true" /> Blocked:
                      </span>
                      {#each config.notAllowedTime as span}
                        <span class="blocked-span">{span.start ?? '?'} → {span.end ?? '?'}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Diagnostics Warnings (#139) -->
        {#if diagNoNetworkPolicy || diagPublicNoIpRestriction || diagNoAutoUpgrade}
          <div class="diagnostics-warnings" data-testid="aks-diagnostics-warnings">
            <h3>Diagnostics</h3>
            {#if diagNoNetworkPolicy}
              <div class="diag-warning">
                <Icon name="diagnostic-warning" size={16} aria-hidden="true" />
                No network policy configured — pod-to-pod traffic is unrestricted
              </div>
            {/if}
            {#if diagPublicNoIpRestriction}
              <div class="diag-warning">
                <Icon name="diagnostic-warning" size={16} aria-hidden="true" />
                Unrestricted public access — API server has no authorized IP restrictions
              </div>
            {/if}
            {#if diagNoAutoUpgrade}
              <div class="diag-info">
                <Icon name="diagnostic-info" size={16} aria-hidden="true" />
                Auto-upgrade disabled — cluster upgrades must be applied manually
              </div>
            {/if}
          </div>
        {/if}
      </section>
    {/if}

    {#if showPowerDialog}
      <div class="dialog-overlay" role="dialog" aria-label="AKS power action">
        <div class="dialog">
          <h3>{currentPowerState.toLowerCase() === 'running' ? 'Stop AKS cluster?' : 'Start AKS cluster?'}</h3>
          <p class="dialog-copy">
            {#if currentPowerState.toLowerCase() === 'running'}
              Stopping the cluster deallocates nodes, pauses compute billing, and disconnects the active Kubernetes session.
            {:else}
              Starting the cluster can take several minutes before the Kubernetes API becomes reachable again.
            {/if}
          </p>
          <div class="dialog-actions">
            <button class="secondary-action" onclick={() => (showPowerDialog = false)}>Cancel</button>
            <button class="primary-action" onclick={handlePowerAction}>
              {currentPowerState.toLowerCase() === 'running' ? 'Stop Cluster' : 'Start Cluster'}
            </button>
          </div>
        </div>
      </div>
    {/if}

    {#if showUpgradeDialog && selectedUpgrade}
      <div class="dialog-overlay" role="dialog" aria-label="AKS upgrade confirmation">
        <div class="dialog">
          <h3>Upgrade AKS cluster to {selectedUpgrade.kubernetesVersion}?</h3>
          <p class="dialog-copy">
            This triggers a rolling Azure-managed control plane upgrade and may cause brief disruption during maintenance windows.
          </p>
          <div class="dialog-field">
            <label for="upgrade-confirmation">Type <code>{upgradeGuardText}</code> to confirm</label>
            <input id="upgrade-confirmation" type="text" bind:value={upgradeConfirmationText} />
          </div>
          {#if selectedUpgrade.isPreview}
            <label class="dialog-checkbox">
              <input type="checkbox" bind:checked={previewAcknowledged} />
              I understand this is a preview Kubernetes version.
            </label>
          {/if}
          <div class="dialog-actions">
            <button class="secondary-action" onclick={() => { showUpgradeDialog = false; selectedUpgrade = null; }}>
              Cancel
            </button>
            <button class="primary-action" disabled={!canConfirmUpgrade()} onclick={handleClusterUpgrade}>
              Upgrade Cluster
            </button>
          </div>
        </div>
      </div>
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
              <span class="card-icon" aria-hidden="true">
                <Icon name={card.icon} size={18} />
              </span>
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
                  <span class="mini-badge warning">
                    <Icon name="diagnostic-warning" size={14} aria-hidden="true" />
                    {warningEvents.length} warnings
                  </span>
                </div>
              {/if}
            </a>
          {/each}
        </div>
      {/if}
    </section>

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
                <th>CPU</th>
                <th>Memory</th>
              </tr>
            </thead>
            <tbody>
              {#each namespaceUsage as ns}
                <tr>
                  <td class="cell-object">{ns.namespace}</td>
                  <td>{ns.pods}</td>
                  <td>{formatCpuMillicores(ns.cpuMillicores)}</td>
                  <td>{formatBinaryBytes(ns.memoryBytes)}</td>
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
  .auth-hint {
    font-size: 0.8rem;
    color: #8b949e;
  }
  .entra-icon {
    margin-right: 0.15rem;
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
  .action-banner {
    padding: 0.85rem 1rem;
    border-radius: 8px;
    margin: 1rem 0;
    font-size: 0.9rem;
  }
  .action-banner.success {
    background: rgba(102, 187, 106, 0.12);
    border: 1px solid rgba(102, 187, 106, 0.35);
    color: #66bb6a;
  }
  .action-banner.error {
    background: rgba(239, 83, 80, 0.12);
    border: 1px solid rgba(239, 83, 80, 0.35);
    color: #ef5350;
  }
  .lifecycle-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 0.75rem;
  }
  .lifecycle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-top: 0.75rem;
  }
  .card-copy,
  .hint {
    margin: 0.5rem 0 0;
    color: #8b949e;
    font-size: 0.82rem;
    line-height: 1.45;
  }
  .upgrade-summary {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }
  .upgrade-current {
    margin-top: 0.25rem;
    font-size: 1.1rem;
    font-weight: 600;
    color: #e0e0e0;
  }
  .upgrade-targets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .upgrade-target {
    background: #0f1724;
    border: 1px solid #2d3a4d;
    color: #e0e0e0;
    border-radius: 999px;
    padding: 0.4rem 0.75rem;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    cursor: pointer;
  }
  .upgrade-target:hover {
    border-color: #58a6ff;
  }
  .preview-pill {
    background: rgba(255, 167, 38, 0.2);
    color: #ffa726;
    border-radius: 999px;
    padding: 0.1rem 0.45rem;
    font-size: 0.72rem;
    font-weight: 600;
  }
  .primary-action,
  .secondary-action {
    border-radius: 6px;
    padding: 0.5rem 0.85rem;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .primary-action {
    background: #1a73e8;
    color: #fff;
  }
  .primary-action:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .secondary-action {
    background: transparent;
    border-color: #30363d;
    color: #c9d1d9;
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
  .state-badge.state-updating,
  .state-badge.state-upgrading { background: rgba(255, 167, 38, 0.2); color: #ffa726; }
  .power-badge {
    display: inline-block;
    padding: 0.05rem 0.35rem;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 500;
  }
  .power-badge.power-running { background: rgba(102, 187, 106, 0.2); color: #66bb6a; }
  .power-badge.power-stopped { background: rgba(139, 148, 158, 0.2); color: #8b949e; }
  .power-badge.power-transitioning { background: rgba(255, 167, 38, 0.2); color: #ffa726; }
  .power-badge.power-unknown { background: rgba(88, 166, 255, 0.15); color: #58a6ff; }
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
  .disabled-badge {
    color: #8b949e;
    font-size: 0.78rem;
  }
  .detail-hint {
    color: #6e7681;
    font-size: 0.72rem;
    margin: 0.5rem 0 0;
    font-style: italic;
  }
  .info-notice {
    color: #58a6ff;
    font-size: 0.78rem;
  }
  .maintenance-config {
    margin-bottom: 0.5rem;
  }
  .maintenance-name {
    font-size: 0.78rem;
    font-weight: 600;
    color: #c9d1d9;
    display: block;
    margin-bottom: 0.2rem;
  }
  .maintenance-slots {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.2rem;
  }
  .maintenance-slot {
    background: rgba(88, 166, 255, 0.12);
    color: #58a6ff;
    font-size: 0.7rem;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', monospace;
  }
  .maintenance-blocked {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    align-items: center;
  }
  .blocked-label {
    font-size: 0.7rem;
    color: #ffa726;
  }
  .blocked-span {
    background: rgba(255, 167, 38, 0.12);
    color: #ffa726;
    font-size: 0.7rem;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', monospace;
  }
  .diagnostics-warnings {
    margin-top: 0.75rem;
    padding: 0.75rem 1rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
  }
  .diagnostics-warnings h3 {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .diag-warning {
    color: #ffa726;
    font-size: 0.82rem;
    padding: 0.3rem 0;
    border-bottom: 1px solid #21262d;
  }
  .diag-warning:last-child {
    border-bottom: none;
  }
  .diag-info {
    color: #58a6ff;
    font-size: 0.82rem;
    padding: 0.3rem 0;
    border-bottom: 1px solid #21262d;
  }
  .diag-info:last-child {
    border-bottom: none;
  }
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    z-index: 30;
  }
  .dialog {
    width: min(100%, 420px);
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 10px;
    padding: 1rem;
  }
  .dialog h3 {
    margin: 0 0 0.5rem;
  }
  .dialog-copy {
    margin: 0 0 1rem;
    color: #8b949e;
    line-height: 1.45;
    font-size: 0.9rem;
  }
  .dialog-field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-bottom: 0.9rem;
  }
  .dialog-field input {
    border-radius: 6px;
    border: 1px solid #30363d;
    background: #0d1117;
    color: #e0e0e0;
    padding: 0.55rem 0.7rem;
  }
  .dialog-checkbox {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
    margin-bottom: 1rem;
    color: #c9d1d9;
    font-size: 0.86rem;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.65rem;
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
