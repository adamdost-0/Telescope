<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { helmUninstall, listHelmReleases } from '$lib/api';
  import { isConnected } from '$lib/stores';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import ErrorMessage from '$lib/components/ErrorMessage.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import type { HelmRelease } from '$lib/tauri-commands';

  let releases: HelmRelease[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let filterQuery = $state('');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;
  let uninstallTarget: HelmRelease | null = $state(null);
  let showUninstallDialog = $state(false);
  let uninstalling = $state(false);
  let actionSuccess: string | null = $state(null);
  let actionError: string | null = $state(null);

  let filtered = $derived.by(() => {
    if (!filterQuery) return releases;
    const q = filterQuery.toLowerCase();
    return releases.filter(r => r.name.toLowerCase().includes(q));
  });

  function formatTimestamp(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    return `Updated ${Math.floor(seconds / 60)}m ago`;
  }

  function statusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'deployed': return '#4caf50';
      case 'failed': return '#ef5350';
      case 'pending-install':
      case 'pending-upgrade':
      case 'pending-rollback':
        return '#ffc107';
      case 'superseded': return '#9e9e9e';
      case 'uninstalling': return '#ff9800';
      default: return '#8b949e';
    }
  }

  function formatUpdated(ts: string): string {
    if (!ts) return '\u2014';
    try {
      const d = new Date(ts);
      return d.toLocaleString();
    } catch {
      return ts;
    }
  }

  async function loadReleases(userInitiated = false) {
    if (!$isConnected) { loading = false; releases = []; return; }
    const isInitial = releases.length === 0 && !lastUpdated;
    if (isInitial) loading = true; else if (userInitiated) refreshing = true;
    error = null;
    try {
      releases = await listHelmReleases();
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load Helm releases';
      releases = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  function requestUninstall(release: HelmRelease) {
    uninstallTarget = release;
    showUninstallDialog = true;
  }

  async function confirmUninstall() {
    if (!uninstallTarget) return;

    actionSuccess = null;
    actionError = null;
    uninstalling = true;

    try {
      const result = await helmUninstall(uninstallTarget.namespace, uninstallTarget.name);
      actionSuccess = result || `Uninstalled Helm release ${uninstallTarget.name}.`;
      showUninstallDialog = false;
      uninstallTarget = null;
      await loadReleases(true);
    } catch (err) {
      actionError = err instanceof Error
        ? err.message
        : `Failed to uninstall Helm release "${uninstallTarget.name}".`;
      showUninstallDialog = false;
      uninstallTarget = null;
    } finally {
      uninstalling = false;
    }
  }

  $effect(() => {
    void $isConnected;
    loadReleases();
  });

  onMount(() => {
    refreshTimer = setInterval(loadReleases, 10_000);
    timestampTimer = setInterval(() => { lastUpdatedText = formatTimestamp(); }, 1000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="resource-page">
  <header>
    <h1>⎈ Helm Releases</h1>
    <div class="controls">
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" onclick={() => loadReleases(true)} disabled={refreshing} class:spinning={refreshing} aria-label="Refresh Helm releases">
        <span class="refresh-icon">\u21bb</span> Refresh
      </button>
    </div>
  </header>

  {#if !$isConnected && !loading}
    <div class="not-connected">
      <p>🔌 Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if loading}
    <LoadingSkeleton rows={5} columns={7} />
  {:else if error}
    <ErrorMessage message={error} onretry={() => loadReleases(true)} />
  {:else}
    {#if actionSuccess}
      <div class="notice success" role="status">{actionSuccess}</div>
    {/if}

    {#if actionError}
      <div class="notice error" role="alert">{actionError}</div>
    {/if}

    <FilterBar query={filterQuery} onfilter={(q) => filterQuery = q} />
    <p class="count">{filterQuery ? `${filtered.length} of ${releases.length}` : releases.length} release{(filterQuery ? filtered.length : releases.length) !== 1 ? 's' : ''}</p>
    <div class="table-wrapper">
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Namespace</th>
            <th>Chart</th>
            <th>App Version</th>
            <th>Revision</th>
            <th>Status</th>
            <th>Updated</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as release (release.name + '/' + release.namespace)}
            <tr>
              <td class="name"><a href="/helm/{release.namespace}/{release.name}">{release.name}</a></td>
              <td>{release.namespace}</td>
              <td class="mono">{release.chart}</td>
              <td>{release.app_version || '—'}</td>
              <td class="center">{release.revision}</td>
              <td>
                <span class="status-badge" style="color: {statusColor(release.status)}">
                  ● {release.status}
                </span>
              </td>
              <td class="updated">{formatUpdated(release.updated)}</td>
              <td class="actions">
                <button
                  type="button"
                  class="uninstall-btn"
                  onclick={() => requestUninstall(release)}
                  disabled={uninstalling}
                >
                  Uninstall
                </button>
              </td>
            </tr>
          {:else}
            <tr>
              <td colspan="8" class="empty">No Helm releases found in this cluster.</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<ConfirmDialog
  open={showUninstallDialog}
  title="Uninstall Helm Release"
  message={`Are you sure you want to uninstall ${uninstallTarget?.name ?? 'this release'}?`}
  confirmText={uninstalling ? 'Uninstalling…' : 'Uninstall'}
  confirmValue={uninstallTarget?.name ?? ''}
  requireType={true}
  productionContext={uninstallTarget?.namespace === 'production' || uninstallTarget?.namespace === 'prod'}
  onconfirm={confirmUninstall}
  oncancel={() => {
    if (!uninstalling) {
      showUninstallDialog = false;
      uninstallTarget = null;
    }
  }}
/>

<style>
  .resource-page { padding: 1rem; color: #e0e0e0; background: #0f0f23; min-height: 100vh; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  h1 { font-size: 1.5rem; margin: 0; }
  .controls { display: flex; gap: 0.75rem; align-items: center; }
  .last-updated { color: #757575; font-size: 0.75rem; }
  button { background: #1a73e8; color: white; border: none; padding: 0.375rem 0.75rem; border-radius: 4px; cursor: pointer; display: inline-flex; align-items: center; gap: 0.25rem; }
  button:hover { background: #1565c0; }
  button:disabled { opacity: 0.6; cursor: not-allowed; }
  .refresh-icon { display: inline-block; }
  .spinning .refresh-icon { animation: spin 1s linear infinite; }
  @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
  .count { color: #9e9e9e; margin-bottom: 0.5rem; font-size: 0.875rem; }
  .not-connected { text-align: center; padding: 3rem 1rem; color: #757575; }
  .not-connected p { margin: 0.25rem 0; font-size: 1.125rem; }
  .not-connected .hint { font-size: 0.875rem; color: #616161; }
  .notice {
    margin-bottom: 0.75rem;
    padding: 0.625rem 0.75rem;
    border-radius: 6px;
    border: 1px solid transparent;
    font-size: 0.875rem;
  }
  .notice.success {
    color: #8fd19e;
    background: rgba(67, 160, 71, 0.16);
    border-color: rgba(67, 160, 71, 0.32);
  }
  .notice.error {
    color: #ff8a80;
    background: rgba(239, 83, 80, 0.16);
    border-color: rgba(239, 83, 80, 0.32);
  }

  .table-wrapper { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  thead th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 2px solid #21262d;
    color: #8b949e;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
  }
  tbody tr { border-bottom: 1px solid #161b22; }
  tbody tr:hover { background: #161b22; }
  tbody td { padding: 0.5rem 0.75rem; white-space: nowrap; }
  .name { color: #58a6ff; font-weight: 500; }
  .name a { color: #58a6ff; text-decoration: none; }
  .name a:hover { text-decoration: underline; }
  .mono { font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 0.8rem; }
  .center { text-align: center; }
  .status-badge { font-weight: 500; }
  .updated { color: #8b949e; font-size: 0.8rem; }
  .actions { text-align: right; }
  .uninstall-btn {
    background: #b71c1c;
    border: 1px solid #d32f2f;
    color: #fff;
    padding: 0.3rem 0.65rem;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .uninstall-btn:hover:not(:disabled) { background: #c62828; }
  .uninstall-btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .empty { text-align: center; color: #757575; padding: 2rem 0.75rem !important; }
</style>
