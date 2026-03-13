<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    getPoolUpgradeProfile,
    listAksNodePools,
    scaleAksNodePool,
    updateAksAutoscaler,
    createAksNodePool,
    deleteAksNodePool,
    upgradePoolNodeImage,
    upgradePoolVersion,
    type CreateNodePoolConfig,
  } from '$lib/api';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import { getAutoRefreshIntervalMs } from '$lib/preferences';
  import { isAks, isConnected } from '$lib/stores';
  import type { AksNodePool, AvailableUpgrade, PoolUpgradeProfile } from '$lib/tauri-commands';

  const PAGE_TITLE = 'Node Pools';

  let pools: AksNodePool[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error: string | null = $state(null);
  let filterQuery = $state('');
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;
  let expandedPools: Record<string, boolean> = $state({});
  let upgradeProfiles: Record<string, PoolUpgradeProfile | null> = $state({});
  let profileLoading: Record<string, boolean> = $state({});
  let upgradeErrors: Record<string, string> = $state({});
  let upgradeMessages: Record<string, string> = $state({});
  let poolUpgradeBusy: Record<string, boolean> = $state({});
  let pendingUpgrade:
    | { poolName: string; type: 'version'; upgrade: AvailableUpgrade }
    | { poolName: string; type: 'node-image' }
    | null = $state(null);
  let previewAcknowledged = $state(false);
  let destroyed = false;

  // ── Operation state ──────────────────────────────────────────────────
  let operationInProgress = $state(false);
  let operationError: string | null = $state(null);
  let operationSuccess: string | null = $state(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Scale dialog ─────────────────────────────────────────────────────
  let showScaleDialog = $state(false);
  let scalePoolName = $state('');
  let scaleCurrentCount = $state(0);
  let scaleTargetCount = $state(0);
  let scaleIsSystem = $state(false);

  // ── Autoscaler dialog ────────────────────────────────────────────────
  let showAutoscalerDialog = $state(false);
  let autoscalerPoolName = $state('');
  let autoscalerEnabled = $state(false);
  let autoscalerMin = $state(1);
  let autoscalerMax = $state(5);

  // ── Create dialog ────────────────────────────────────────────────────
  let showCreateDialog = $state(false);
  let createName = $state('');
  let createVmSize = $state('Standard_DS2_v2');
  let createCount = $state(3);
  let createOsType = $state('Linux');
  let createMode = $state('User');
  let createK8sVersion = $state('');
  let createAutoScaling = $state(false);
  let createMinCount = $state(1);
  let createMaxCount = $state(5);
  let createZones = $state('');
  let createMaxPods = $state(110);
  let createLabelsRaw = $state('');
  let createTaintsRaw = $state('');

  // ── Delete dialog ────────────────────────────────────────────────────
  let showDeleteDialog = $state(false);
  let deletePoolName = $state('');
  let deletePoolMode = $state('');
  let deletePoolCount = $state(0);
  let deleteConfirmText = $state('');

  // Common VM SKUs for the dropdown
  const commonVmSizes = [
    'Standard_DS2_v2',
    'Standard_D4s_v5',
    'Standard_D8s_v5',
    'Standard_D16s_v5',
    'Standard_E4s_v5',
    'Standard_E8s_v5',
    'Standard_F4s_v2',
    'Standard_F8s_v2',
    'Standard_NC6s_v3',
    'Standard_NC12s_v3',
    'Standard_B2s',
    'Standard_B4ms',
  ];

  function showToast(msg: string, isError = false) {
    if (isError) {
      operationError = msg;
      operationSuccess = null;
    } else {
      operationSuccess = msg;
      operationError = null;
    }
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      operationError = null;
      operationSuccess = null;
    }, 6000);
  }

  function formatTimestamp(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    return `Updated ${Math.floor(seconds / 60)}m ago`;
  }

  function togglePool(name: string) {
    const nextExpanded = !expandedPools[name];
    expandedPools = { ...expandedPools, [name]: nextExpanded };
    if (nextExpanded) {
      void loadUpgradeProfile(name);
    }
  }

  function poolAnchorId(name: string): string {
    return `pool-${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  }

  function formatAutoscaler(pool: AksNodePool): string {
    if (!pool.enableAutoScaling) return 'Off';
    const min = pool.minCount ?? '—';
    const max = pool.maxCount ?? '—';
    return `${min}-${max}`;
  }

  function formatZones(pool: AksNodePool): string {
    return pool.availabilityZones?.length ? pool.availabilityZones.join(', ') : '—';
  }

  function formatStatus(pool: AksNodePool): string {
    return pool.provisioningState ?? pool.powerState?.code ?? 'Unknown';
  }

  function statusVariant(status: string): string {
    const normalized = status.toLowerCase();
    if (normalized === 'succeeded' || normalized === 'running') return 'success';
    if (normalized === 'updating' || normalized === 'creating' || normalized === 'stopping') {
      return 'warning';
    }
    if (normalized === 'failed' || normalized === 'error') return 'danger';
    return 'neutral';
  }

  function isLastSystemPool(poolName: string): boolean {
    const systemPools = pools.filter((p) => p.mode?.toLowerCase() === 'system');
    return systemPools.length === 1 && systemPools[0].name === poolName;
  }

  let filteredPools = $derived.by(() => {
    if (!filterQuery) return pools;
    const query = filterQuery.toLowerCase();
    return pools.filter((pool) =>
      [
        pool.name,
        pool.mode ?? '',
        pool.vmSize ?? '',
        pool.osType ?? '',
        pool.orchestratorVersion ?? '',
        pool.nodeImageVersion ?? '',
        pool.provisioningState ?? '',
        pool.powerState?.code ?? '',
      ].some((value) => value.toLowerCase().includes(query))
    );
  });

  async function loadUpgradeProfile(poolName: string, force = false) {
    if (profileLoading[poolName]) return;
    if (!force && upgradeProfiles[poolName]) return;

    profileLoading = { ...profileLoading, [poolName]: true };
    try {
      const profile = await getPoolUpgradeProfile(poolName);
      upgradeProfiles = { ...upgradeProfiles, [poolName]: profile };
      upgradeErrors = { ...upgradeErrors, [poolName]: '' };
    } catch (e) {
      upgradeErrors = {
        ...upgradeErrors,
        [poolName]: e instanceof Error ? e.message : 'Failed to load upgrade profile',
      };
    } finally {
      profileLoading = { ...profileLoading, [poolName]: false };
    }
  }

  function openVersionUpgrade(poolName: string, upgrade: AvailableUpgrade) {
    pendingUpgrade = { poolName, type: 'version', upgrade };
    previewAcknowledged = false;
  }

  function openNodeImageUpgrade(poolName: string) {
    pendingUpgrade = { poolName, type: 'node-image' };
    previewAcknowledged = false;
  }

  async function confirmUpgrade() {
    if (!pendingUpgrade) return;
    const { poolName } = pendingUpgrade;
    poolUpgradeBusy = { ...poolUpgradeBusy, [poolName]: true };
    upgradeErrors = { ...upgradeErrors, [poolName]: '' };

    try {
      if (pendingUpgrade.type === 'version') {
        await upgradePoolVersion(poolName, pendingUpgrade.upgrade.kubernetesVersion);
        upgradeMessages = {
          ...upgradeMessages,
          [poolName]: `Pool upgraded to Kubernetes ${pendingUpgrade.upgrade.kubernetesVersion}.`,
        };
      } else {
        await upgradePoolNodeImage(poolName);
        upgradeMessages = {
          ...upgradeMessages,
          [poolName]: 'Pool node image upgrade completed.',
        };
      }
      await Promise.all([loadPools(true), loadUpgradeProfile(poolName, true)]);
    } catch (e) {
      upgradeErrors = {
        ...upgradeErrors,
        [poolName]: e instanceof Error ? e.message : 'Pool upgrade failed',
      };
    } finally {
      poolUpgradeBusy = { ...poolUpgradeBusy, [poolName]: false };
      pendingUpgrade = null;
      previewAcknowledged = false;
    }
  }

  async function loadPools(userInitiated = false) {
    if (!$isConnected || !$isAks) {
      pools = [];
      error = null;
      loading = false;
      refreshing = false;
      return;
    }

    const isInitial = pools.length === 0 && !lastUpdated;
    if (isInitial) {
      loading = true;
    } else if (userInitiated) {
      refreshing = true;
    }

    error = null;

    try {
      pools = await listAksNodePools();
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load node pools';
      pools = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  // ── Scale ────────────────────────────────────────────────────────────

  function openScaleDialog(pool: AksNodePool) {
    scalePoolName = pool.name;
    scaleCurrentCount = pool.count ?? 0;
    scaleTargetCount = pool.count ?? 1;
    scaleIsSystem = pool.mode?.toLowerCase() === 'system';
    showScaleDialog = true;
  }

  async function submitScale() {
    if (scaleIsSystem && scaleTargetCount < 1) {
      showToast('System pools require at least 1 node.', true);
      return;
    }
    operationInProgress = true;
    try {
      await scaleAksNodePool(scalePoolName, scaleTargetCount);
      showScaleDialog = false;
      showToast(`Scaling ${scalePoolName} to ${scaleTargetCount} nodes…`);
      await loadPools(true);
    } catch (e) {
      showToast(e instanceof Error ? e.message : 'Scale operation failed', true);
    } finally {
      operationInProgress = false;
    }
  }

  // ── Autoscaler ───────────────────────────────────────────────────────

  function openAutoscalerDialog(pool: AksNodePool) {
    autoscalerPoolName = pool.name;
    autoscalerEnabled = pool.enableAutoScaling ?? false;
    autoscalerMin = pool.minCount ?? 1;
    autoscalerMax = pool.maxCount ?? 5;
    showAutoscalerDialog = true;
  }

  async function submitAutoscaler() {
    operationInProgress = true;
    try {
      await updateAksAutoscaler(
        autoscalerPoolName,
        autoscalerEnabled,
        autoscalerEnabled ? autoscalerMin : null,
        autoscalerEnabled ? autoscalerMax : null,
      );
      showAutoscalerDialog = false;
      showToast(
        autoscalerEnabled
          ? `Autoscaler enabled on ${autoscalerPoolName} (${autoscalerMin}–${autoscalerMax})`
          : `Autoscaler disabled on ${autoscalerPoolName}`,
      );
      await loadPools(true);
    } catch (e) {
      showToast(e instanceof Error ? e.message : 'Autoscaler update failed', true);
    } finally {
      operationInProgress = false;
    }
  }

  // ── Create ───────────────────────────────────────────────────────────

  function openCreateDialog() {
    createName = '';
    createVmSize = 'Standard_DS2_v2';
    createCount = 3;
    createOsType = 'Linux';
    createMode = 'User';
    createK8sVersion = '';
    createAutoScaling = false;
    createMinCount = 1;
    createMaxCount = 5;
    createZones = '';
    createMaxPods = 110;
    createLabelsRaw = '';
    createTaintsRaw = '';
    showCreateDialog = true;
  }

  function parseLabels(raw: string): Record<string, string> | undefined {
    if (!raw.trim()) return undefined;
    const labels: Record<string, string> = {};
    for (const pair of raw.split(',')) {
      const [key, ...rest] = pair.split('=');
      if (key?.trim()) labels[key.trim()] = rest.join('=').trim();
    }
    return Object.keys(labels).length ? labels : undefined;
  }

  function parseTaints(raw: string): string[] | undefined {
    if (!raw.trim()) return undefined;
    const taints = raw
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);
    return taints.length ? taints : undefined;
  }

  function parseZones(raw: string): string[] | undefined {
    if (!raw.trim()) return undefined;
    const zones = raw
      .split(',')
      .map((z) => z.trim())
      .filter(Boolean);
    return zones.length ? zones : undefined;
  }

  let createNameValid = $derived(/^[a-z][a-z0-9]{0,11}$/.test(createName));

  async function submitCreate() {
    if (!createNameValid) {
      showToast('Pool name must be lowercase alphanumeric, 1–12 chars, starting with a letter.', true);
      return;
    }
    if (pools.some((p) => p.name === createName)) {
      showToast(`A pool named "${createName}" already exists.`, true);
      return;
    }
    operationInProgress = true;
    try {
      const config: CreateNodePoolConfig = {
        name: createName,
        vmSize: createVmSize,
        count: createCount,
        osType: createOsType,
        mode: createMode,
        orchestratorVersion: createK8sVersion || undefined,
        enableAutoScaling: createAutoScaling || undefined,
        minCount: createAutoScaling ? createMinCount : undefined,
        maxCount: createAutoScaling ? createMaxCount : undefined,
        availabilityZones: parseZones(createZones),
        maxPods: createMaxPods || undefined,
        nodeLabels: parseLabels(createLabelsRaw),
        nodeTaints: parseTaints(createTaintsRaw),
      };
      await createAksNodePool(config);
      showCreateDialog = false;
      showToast(`Creating node pool "${createName}"…`);
      await loadPools(true);
    } catch (e) {
      showToast(e instanceof Error ? e.message : 'Create operation failed', true);
    } finally {
      operationInProgress = false;
    }
  }

  // ── Delete ───────────────────────────────────────────────────────────

  function openDeleteDialog(pool: AksNodePool) {
    if (isLastSystemPool(pool.name)) {
      showToast('Cannot delete the last system pool.', true);
      return;
    }
    deletePoolName = pool.name;
    deletePoolMode = pool.mode ?? 'User';
    deletePoolCount = pool.count ?? 0;
    deleteConfirmText = '';
    showDeleteDialog = true;
  }

  async function submitDelete() {
    if (deleteConfirmText !== deletePoolName) {
      showToast('Type the pool name to confirm deletion.', true);
      return;
    }
    operationInProgress = true;
    try {
      await deleteAksNodePool(deletePoolName);
      showDeleteDialog = false;
      showToast(`Deleting node pool "${deletePoolName}"…`);
      await loadPools(true);
    } catch (e) {
      showToast(e instanceof Error ? e.message : 'Delete operation failed', true);
    } finally {
      operationInProgress = false;
    }
  }

  $effect(() => {
    void $isConnected;
    void $isAks;
    loadPools();
  });

  onMount(() => {
    void (async () => {
      const refreshIntervalMs = await getAutoRefreshIntervalMs(10_000);
      if (!destroyed) {
        refreshTimer = setInterval(() => {
          void loadPools();
        }, refreshIntervalMs);
      }
    })();
    timestampTimer = setInterval(() => {
      lastUpdatedText = formatTimestamp();
    }, 1000);
  });

  onDestroy(() => {
    destroyed = true;
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
    if (toastTimer) clearTimeout(toastTimer);
  });
</script>

<div class="resource-page">
  <!-- Toast notifications -->
  {#if operationError}
    <div class="toast toast-error" role="alert">{operationError}</div>
  {/if}
  {#if operationSuccess}
    <div class="toast toast-success" role="status">{operationSuccess}</div>
  {/if}

  <header>
    <div>
      <h1>{PAGE_TITLE}</h1>
      <p class="subtitle">Authoritative AKS node pool data from the Azure ARM API.</p>
    </div>
    <div class="controls">
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" class="btn-primary" onclick={openCreateDialog} disabled={!$isConnected || !$isAks || loading}>
        + Create Pool
      </button>
      <button type="button" onclick={() => loadPools(true)} disabled={refreshing} class:spinning={refreshing}>
        <span class="refresh-icon">↻</span>
        Refresh
      </button>
    </div>
  </header>

  {#if !$isConnected && !loading}
    <div class="state-card">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if !$isAks && !loading}
    <div class="state-card">
      <p>Azure node pool data requires an AKS cluster.</p>
    </div>
  {:else if loading}
    <LoadingSkeleton rows={6} columns={10} />
  {:else if error}
    <div role="alert" class="state-card error-card">
      <p>Failed to load node pools.</p>
      <p class="hint">{error}</p>
      <button type="button" onclick={() => loadPools(true)}>Retry</button>
    </div>
  {:else}
    <FilterBar query={filterQuery} onfilter={(query) => (filterQuery = query)} />
    <p class="count">
      {filterQuery ? `${filteredPools.length} of ${pools.length}` : pools.length}
      node pools
    </p>

    {#if filteredPools.length === 0}
      <div class="state-card">
        <p>No node pools found.</p>
      </div>
    {:else}
      <div class="table-wrapper">
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Mode</th>
              <th>VM Size</th>
              <th>Count</th>
              <th>Autoscaler</th>
              <th>K8s Version</th>
              <th>Node Image</th>
              <th>OS</th>
              <th>Zones</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredPools as pool (pool.name)}
              <tr id={poolAnchorId(pool.name)}>
                <td>
                  <a
                    href={`#${poolAnchorId(pool.name)}`}
                    class="pool-link"
                    onclick={() => togglePool(pool.name)}
                  >
                    {expandedPools[pool.name] ? '▾' : '▸'} {pool.name}
                  </a>
                </td>
                <td>{pool.mode ?? '—'}</td>
                <td>{pool.vmSize ?? '—'}</td>
                <td>{pool.count ?? '—'}</td>
                <td>{formatAutoscaler(pool)}</td>
                <td>{pool.orchestratorVersion ?? '—'}</td>
                <td>{pool.nodeImageVersion ?? '—'}</td>
                <td>{pool.osType ?? '—'}</td>
                <td>{formatZones(pool)}</td>
                <td>
                  <span class={`status-badge ${statusVariant(formatStatus(pool))}`}>
                    {formatStatus(pool)}
                  </span>
                </td>
                <td class="actions-cell">
                  <button type="button" class="btn-sm" onclick={() => openScaleDialog(pool)} disabled={operationInProgress} title="Scale node count">
                    Scale
                  </button>
                  <button type="button" class="btn-sm" onclick={() => openAutoscalerDialog(pool)} disabled={operationInProgress} title="Configure autoscaler">
                    Autoscaler
                  </button>
                  <button
                    type="button"
                    class="btn-sm btn-danger"
                    onclick={() => openDeleteDialog(pool)}
                    disabled={operationInProgress || isLastSystemPool(pool.name)}
                    title={isLastSystemPool(pool.name) ? 'Cannot delete the last system pool' : 'Delete pool'}
                  >
                    Delete
                  </button>
                </td>
              </tr>
              {#if expandedPools[pool.name]}
                <tr class="details-row">
                  <td colspan="11">
                    <div class="details-grid">
                      <div><span class="details-label">OS Disk</span><span>{pool.osDiskSizeGb ?? '—'} GiB</span></div>
                      <div><span class="details-label">Max Pods</span><span>{pool.maxPods ?? '—'}</span></div>
                      <div><span class="details-label">Power State</span><span>{pool.powerState?.code ?? '—'}</span></div>
                      <div><span class="details-label">Subnet</span><span class="break">{pool.vnetSubnetId ?? '—'}</span></div>
                      <div><span class="details-label">Taints</span><span>{pool.nodeTaints?.join(', ') ?? '—'}</span></div>
                      <div><span class="details-label">Labels</span><span class="break">{pool.nodeLabels ? JSON.stringify(pool.nodeLabels) : '—'}</span></div>
                    </div>

                    <div class="upgrade-panel">
                      <div class="upgrade-panel-header">
                        <div>
                          <h3>Upgrade Options</h3>
                          <p>
                            Current K8s {upgradeProfiles[pool.name]?.currentVersion ?? pool.orchestratorVersion ?? '—'}
                            · Node image {pool.nodeImageVersion ?? '—'}
                          </p>
                        </div>
                        <button
                          type="button"
                          class="btn-sm"
                          onclick={() => loadUpgradeProfile(pool.name, true)}
                          disabled={profileLoading[pool.name] || poolUpgradeBusy[pool.name]}
                        >
                          {profileLoading[pool.name] ? 'Loading…' : 'Refresh Upgrade Data'}
                        </button>
                      </div>

                      {#if upgradeMessages[pool.name]}
                        <div class="inline-message success" role="status">{upgradeMessages[pool.name]}</div>
                      {/if}
                      {#if upgradeErrors[pool.name]}
                        <div class="inline-message error" role="alert">{upgradeErrors[pool.name]}</div>
                      {/if}

                      {#if profileLoading[pool.name]}
                        <p class="hint">Loading Azure upgrade options…</p>
                      {:else if upgradeProfiles[pool.name]}
                        <div class="upgrade-actions">
                          <button
                            type="button"
                            class="btn-sm btn-primary"
                            onclick={() => openNodeImageUpgrade(pool.name)}
                            disabled={poolUpgradeBusy[pool.name]}
                          >
                            {poolUpgradeBusy[pool.name] ? 'Upgrading…' : 'Upgrade Node Image'}
                          </button>
                          <span class="hint">
                            Latest node image: {upgradeProfiles[pool.name]?.latestNodeImageVersion ?? 'Unavailable'}
                          </span>
                        </div>

                        <div class="upgrade-targets">
                          {#if upgradeProfiles[pool.name]?.upgrades.length}
                            {#each upgradeProfiles[pool.name]?.upgrades ?? [] as upgrade}
                              <button
                                type="button"
                                class="upgrade-target"
                                onclick={() => openVersionUpgrade(pool.name, upgrade)}
                                disabled={poolUpgradeBusy[pool.name]}
                              >
                                <span>{upgrade.kubernetesVersion}</span>
                                {#if upgrade.isPreview}
                                  <span class="preview-pill">Preview</span>
                                {/if}
                              </button>
                            {/each}
                          {:else}
                            <p class="hint">No newer Kubernetes versions available for this pool.</p>
                          {/if}
                        </div>
                      {:else}
                        <p class="hint">Expand the pool to load Azure upgrade targets.</p>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/if}
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<!-- ── Scale Dialog ────────────────────────────────────────────────────── -->
{#if showScaleDialog}
  <div class="dialog-backdrop" onclick={() => (showScaleDialog = false)} role="presentation">
    <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Scale Node Pool">
      <h2>Scale Node Pool: {scalePoolName}</h2>
      <p class="dialog-subtitle">Current count: <strong>{scaleCurrentCount}</strong></p>
      <label class="dialog-field">
        <span>Target count</span>
        <input type="number" bind:value={scaleTargetCount} min={scaleIsSystem ? 1 : 0} max="1000" />
      </label>
      {#if scaleIsSystem && scaleTargetCount < 1}
        <p class="field-error">System pools require at least 1 node.</p>
      {/if}
      <div class="dialog-actions">
        <button type="button" onclick={() => (showScaleDialog = false)}>Cancel</button>
        <button type="button" class="btn-primary" onclick={submitScale} disabled={operationInProgress || (scaleIsSystem && scaleTargetCount < 1)}>
          {operationInProgress ? 'Scaling…' : 'Scale'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ── Autoscaler Dialog ───────────────────────────────────────────────── -->
{#if showAutoscalerDialog}
  <div class="dialog-backdrop" onclick={() => (showAutoscalerDialog = false)} role="presentation">
    <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Configure Autoscaler">
      <h2>Autoscaler: {autoscalerPoolName}</h2>
      <label class="dialog-field toggle-field">
        <span>Enable autoscaler</span>
        <input type="checkbox" bind:checked={autoscalerEnabled} />
      </label>
      {#if autoscalerEnabled}
        <label class="dialog-field">
          <span>Min count</span>
          <input type="number" bind:value={autoscalerMin} min="1" max="1000" />
        </label>
        <label class="dialog-field">
          <span>Max count</span>
          <input type="number" bind:value={autoscalerMax} min="1" max="1000" />
        </label>
      {/if}
      <div class="dialog-actions">
        <button type="button" onclick={() => (showAutoscalerDialog = false)}>Cancel</button>
        <button type="button" class="btn-primary" onclick={submitAutoscaler} disabled={operationInProgress}>
          {operationInProgress ? 'Updating…' : 'Apply'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ── Create Dialog ───────────────────────────────────────────────────── -->
{#if showCreateDialog}
  <div class="dialog-backdrop" onclick={() => (showCreateDialog = false)} role="presentation">
    <div class="dialog dialog-wide" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Create Node Pool">
      <h2>Create Node Pool</h2>
      <div class="dialog-form-grid">
        <label class="dialog-field">
          <span>Pool name <small>(a-z0-9, max 12 chars)</small></span>
          <input type="text" bind:value={createName} maxlength="12" placeholder="mypool" />
          {#if createName && !createNameValid}
            <span class="field-error">Must be lowercase alphanumeric, start with a letter, 1–12 chars.</span>
          {/if}
        </label>
        <label class="dialog-field">
          <span>VM size</span>
          <select bind:value={createVmSize}>
            {#each commonVmSizes as size}
              <option value={size}>{size}</option>
            {/each}
          </select>
        </label>
        <label class="dialog-field">
          <span>Node count</span>
          <input type="number" bind:value={createCount} min="1" max="1000" />
        </label>
        <label class="dialog-field">
          <span>OS type</span>
          <select bind:value={createOsType}>
            <option value="Linux">Linux</option>
            <option value="Windows">Windows</option>
          </select>
        </label>
        <label class="dialog-field">
          <span>Mode</span>
          <select bind:value={createMode}>
            <option value="User">User</option>
            <option value="System">System</option>
          </select>
        </label>
        <label class="dialog-field">
          <span>K8s version <small>(optional)</small></span>
          <input type="text" bind:value={createK8sVersion} placeholder="e.g. 1.29.2" />
        </label>
        <label class="dialog-field toggle-field">
          <span>Enable autoscaler</span>
          <input type="checkbox" bind:checked={createAutoScaling} />
        </label>
        {#if createAutoScaling}
          <label class="dialog-field">
            <span>Min count</span>
            <input type="number" bind:value={createMinCount} min="1" max="1000" />
          </label>
          <label class="dialog-field">
            <span>Max count</span>
            <input type="number" bind:value={createMaxCount} min="1" max="1000" />
          </label>
        {/if}
        <label class="dialog-field">
          <span>Availability zones <small>(comma-separated, e.g. 1,2,3)</small></span>
          <input type="text" bind:value={createZones} placeholder="1,2,3" />
        </label>
        <label class="dialog-field">
          <span>Max pods per node</span>
          <input type="number" bind:value={createMaxPods} min="10" max="250" />
        </label>
        <label class="dialog-field full-width">
          <span>Labels <small>(key=value, comma-separated)</small></span>
          <input type="text" bind:value={createLabelsRaw} placeholder="env=staging,team=platform" />
        </label>
        <label class="dialog-field full-width">
          <span>Taints <small>(key=value:effect, comma-separated)</small></span>
          <input type="text" bind:value={createTaintsRaw} placeholder="gpu=true:NoSchedule" />
        </label>
      </div>
      <div class="dialog-actions">
        <button type="button" onclick={() => (showCreateDialog = false)}>Cancel</button>
        <button type="button" class="btn-primary" onclick={submitCreate} disabled={operationInProgress || !createNameValid}>
          {operationInProgress ? 'Creating…' : 'Create Pool'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ── Delete Dialog ───────────────────────────────────────────────────── -->
{#if showDeleteDialog}
  <div class="dialog-backdrop" onclick={() => (showDeleteDialog = false)} role="presentation">
    <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Delete Node Pool">
      <h2>Delete Node Pool</h2>
      <div class="delete-warning">
        <p>⚠️ This will permanently remove pool <strong>{deletePoolName}</strong>.</p>
        <p>Mode: <strong>{deletePoolMode}</strong> · Nodes: <strong>{deletePoolCount}</strong></p>
        <p class="hint">All nodes and workloads on this pool will be removed.</p>
      </div>
      <label class="dialog-field">
        <span>Type <strong>{deletePoolName}</strong> to confirm</span>
        <input type="text" bind:value={deleteConfirmText} placeholder={deletePoolName} />
      </label>
      <div class="dialog-actions">
        <button type="button" onclick={() => (showDeleteDialog = false)}>Cancel</button>
        <button type="button" class="btn-danger" onclick={submitDelete} disabled={operationInProgress || deleteConfirmText !== deletePoolName}>
          {operationInProgress ? 'Deleting…' : 'Delete Pool'}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if pendingUpgrade}
  <div class="dialog-backdrop" onclick={() => (pendingUpgrade = null)} role="presentation">
    <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Upgrade Node Pool">
      <h2>
        {#if pendingUpgrade.type === 'version'}
          Upgrade {pendingUpgrade.poolName} to Kubernetes {pendingUpgrade.upgrade.kubernetesVersion}
        {:else}
          Upgrade node image for {pendingUpgrade.poolName}
        {/if}
      </h2>
      <p class="dialog-subtitle">
        Azure performs a rolling node replacement for this operation. Pool status will show updating until completion.
      </p>
      {#if pendingUpgrade.type === 'version' && pendingUpgrade.upgrade.isPreview}
        <label class="dialog-field toggle-field">
          <span>I understand this targets a preview Kubernetes version.</span>
          <input type="checkbox" bind:checked={previewAcknowledged} />
        </label>
      {/if}
      {#if pendingUpgrade.type === 'node-image'}
        <p class="hint">
          Latest available node image: {upgradeProfiles[pendingUpgrade.poolName]?.latestNodeImageVersion ?? 'Unavailable'}
        </p>
      {/if}
      <div class="dialog-actions">
        <button type="button" onclick={() => (pendingUpgrade = null)}>Cancel</button>
        <button
          type="button"
          class="btn-primary"
          onclick={confirmUpgrade}
          disabled={pendingUpgrade.type === 'version' && pendingUpgrade.upgrade.isPreview && !previewAcknowledged}
        >
          Confirm Upgrade
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .resource-page {
    padding: 1rem;
    color: #e0e0e0;
    background: #0f0f23;
    min-height: 100vh;
  }

  header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  h1 {
    font-size: 1.5rem;
    margin: 0;
  }

  .subtitle {
    margin: 0.25rem 0 0;
    color: #9e9e9e;
  }

  .controls {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }

  .last-updated {
    color: #757575;
    font-size: 0.75rem;
  }

  button {
    background: #1a73e8;
    color: white;
    border: none;
    padding: 0.375rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }

  button:hover {
    background: #1565c0;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-primary {
    background: #1a73e8;
  }

  .btn-sm {
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
  }

  .btn-danger {
    background: #c62828;
  }
  .btn-danger:hover {
    background: #b71c1c;
  }

  .actions-cell {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .refresh-icon {
    display: inline-block;
  }

  .spinning .refresh-icon {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .count {
    color: #9e9e9e;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
  }

  .state-card {
    padding: 1.5rem;
    text-align: center;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
  }

  .hint {
    color: #9e9e9e;
    font-size: 0.875rem;
  }

  .error-card p:first-child {
    color: #ef5350;
  }

  .table-wrapper {
    overflow-x: auto;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    text-align: left;
    padding: 0.75rem;
    border-bottom: 1px solid #21262d;
    vertical-align: top;
  }

  th {
    color: #9e9e9e;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  tbody tr:hover {
    background: rgba(255, 255, 255, 0.02);
  }

  .pool-link {
    color: #8ab4f8;
    text-decoration: none;
    font-weight: 500;
  }

  .pool-link:hover {
    text-decoration: underline;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.8rem;
    font-weight: 600;
    border: 1px solid transparent;
  }

  .status-badge.success {
    color: #66bb6a;
    background: rgba(102, 187, 106, 0.12);
    border-color: rgba(102, 187, 106, 0.35);
  }

  .status-badge.warning {
    color: #ffa726;
    background: rgba(255, 167, 38, 0.12);
    border-color: rgba(255, 167, 38, 0.35);
  }

  .status-badge.danger {
    color: #ef5350;
    background: rgba(239, 83, 80, 0.12);
    border-color: rgba(239, 83, 80, 0.35);
  }

  .status-badge.neutral {
    color: #c5c5c5;
    background: rgba(197, 197, 197, 0.1);
    border-color: rgba(197, 197, 197, 0.2);
  }

  .details-row td {
    background: #121826;
  }

  .details-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.75rem 1rem;
  }

  .details-grid > div {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .upgrade-panel {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #21262d;
  }
  .upgrade-panel-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 0.75rem;
  }
  .upgrade-panel-header h3 {
    margin: 0;
    font-size: 0.95rem;
  }
  .upgrade-panel-header p {
    margin: 0.25rem 0 0;
    color: #9e9e9e;
    font-size: 0.8rem;
  }
  .upgrade-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    margin-bottom: 0.75rem;
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
    padding: 0.35rem 0.7rem;
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
  .inline-message {
    border-radius: 6px;
    padding: 0.65rem 0.75rem;
    margin-bottom: 0.75rem;
    font-size: 0.82rem;
  }
  .inline-message.success {
    background: rgba(102, 187, 106, 0.12);
    border: 1px solid rgba(102, 187, 106, 0.35);
    color: #66bb6a;
  }
  .inline-message.error {
    background: rgba(239, 83, 80, 0.12);
    border: 1px solid rgba(239, 83, 80, 0.35);
    color: #ef5350;
  }

  .details-label {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: #9e9e9e;
  }

  .break {
    overflow-wrap: anywhere;
  }

  /* ── Toast ──────────────────────────────────────────────────────────── */
  .toast {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 10000;
    padding: 0.75rem 1.25rem;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    max-width: 420px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  }
  .toast-success {
    background: #1b5e20;
    color: #c8e6c9;
    border: 1px solid #388e3c;
  }
  .toast-error {
    background: #b71c1c;
    color: #ffcdd2;
    border: 1px solid #e53935;
  }

  /* ── Dialog ─────────────────────────────────────────────────────────── */
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9000;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .dialog {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 10px;
    padding: 1.5rem;
    min-width: 360px;
    max-width: 480px;
    max-height: 85vh;
    overflow-y: auto;
  }
  .dialog-wide {
    min-width: 440px;
    max-width: 600px;
  }
  .dialog h2 {
    margin: 0 0 0.75rem;
    font-size: 1.15rem;
  }
  .dialog-subtitle {
    color: #9e9e9e;
    margin: 0 0 0.75rem;
    font-size: 0.875rem;
  }
  .dialog-field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.75rem;
  }
  .dialog-field span {
    font-size: 0.8rem;
    color: #9e9e9e;
  }
  .dialog-field input,
  .dialog-field select {
    background: #0d1117;
    color: #e0e0e0;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.4rem 0.55rem;
    font-size: 0.875rem;
  }
  .dialog-field input:focus,
  .dialog-field select:focus {
    outline: none;
    border-color: #1a73e8;
  }
  .toggle-field {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
  }
  .toggle-field input[type='checkbox'] {
    width: 1rem;
    height: 1rem;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1rem;
  }
  .dialog-form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 1rem;
  }
  .full-width {
    grid-column: 1 / -1;
  }
  .field-error {
    color: #ef5350;
    font-size: 0.78rem;
    margin: 0;
  }
  .delete-warning {
    background: rgba(198, 40, 40, 0.12);
    border: 1px solid rgba(198, 40, 40, 0.35);
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .delete-warning p {
    margin: 0.25rem 0;
  }
</style>
