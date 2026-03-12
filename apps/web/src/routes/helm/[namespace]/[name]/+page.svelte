<script lang="ts">
  import { page } from '$app/state';
  import { onMount, onDestroy } from 'svelte';
  import { listHelmReleases, getHelmReleaseHistory } from '$lib/api';
  import Tabs from '$lib/components/Tabs.svelte';
  import type { HelmRelease } from '$lib/tauri-commands';

  let releaseName = $derived(page.params.name);
  let namespace = $derived(page.params.namespace);

  let release: HelmRelease | null = $state(null);
  let history: HelmRelease[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let activeTab = $state('info');

  const tabs = [
    { id: 'info', label: 'Info' },
    { id: 'values', label: 'Values' },
    { id: 'history', label: 'History' },
  ];

  function formatTimestamp(ts: string): string {
    if (!ts) return '—';
    try {
      return new Date(ts).toLocaleString();
    } catch {
      return ts;
    }
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

  async function loadRelease() {
    loading = true;
    error = null;
    try {
      const [releases, hist] = await Promise.all([
        listHelmReleases(namespace),
        getHelmReleaseHistory(namespace, releaseName),
      ]);
      release = releases.find((r) => r.name === releaseName && r.namespace === namespace) ?? null;
      // If not found in filtered list, try the history (latest revision)
      if (!release && hist.length > 0) {
        release = hist[hist.length - 1];
      }
      history = hist;
      if (!release && hist.length === 0) {
        error = `Release "${releaseName}" not found in namespace "${namespace}"`;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load release';
    } finally {
      loading = false;
    }
  }

  onMount(loadRelease);
</script>

<div class="detail-page">
  <header class="detail-header">
    <a href="/helm" class="back">← Helm Releases</a>
    <h1>{releaseName}</h1>
    <span class="namespace-badge">{namespace}</span>
  </header>

  {#if loading}
    <p role="status">Loading release details…</p>
  {:else if error}
    <p role="alert" class="error">{error}</p>
  {:else if release}
    <Tabs {tabs} {activeTab} onchange={(id) => activeTab = id} />

    {#if activeTab === 'info'}
      <div class="summary">
        <h3>Release Info</h3>
        <dl>
          <dt>Name</dt><dd>{release.name}</dd>
          <dt>Namespace</dt><dd>{release.namespace}</dd>
          <dt>Chart</dt><dd>{release.chart}</dd>
          <dt>App Version</dt><dd>{release.app_version || '—'}</dd>
          <dt>Revision</dt><dd>{release.revision}</dd>
          <dt>Status</dt>
          <dd>
            <span class="status-badge" style="color: {statusColor(release.status)}">
              ● {release.status}
            </span>
          </dd>
          <dt>Last Deployed</dt><dd>{formatTimestamp(release.updated)}</dd>
        </dl>

        {#if history.length > 0}
          <h3>Deployment Timeline</h3>
          <dl>
            <dt>First Deployed</dt><dd>{formatTimestamp(history[0]?.updated ?? '')}</dd>
            <dt>Last Deployed</dt><dd>{formatTimestamp(history[history.length - 1]?.updated ?? '')}</dd>
            <dt>Total Revisions</dt><dd>{history.length}</dd>
          </dl>
        {/if}
      </div>

    {:else if activeTab === 'values'}
      <div class="tab-content">
        <div class="placeholder-card">
          <span class="placeholder-icon">📋</span>
          <p>Values view requires Helm CLI integration — coming soon</p>
          <p class="muted">In a future release, this tab will display the computed values for this Helm release by reading the release secret data.</p>
        </div>
      </div>

    {:else if activeTab === 'history'}
      <div class="tab-content">
        {#if history.length === 0}
          <p class="muted">No revision history available.</p>
        {:else}
          <table>
            <thead>
              <tr>
                <th>Revision</th>
                <th>Status</th>
                <th>Chart</th>
                <th>App Version</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {#each history as rev}
                <tr class:active={rev.revision === release?.revision}>
                  <td class="center">{rev.revision}</td>
                  <td>
                    <span class="status-badge" style="color: {statusColor(rev.status)}">
                      ● {rev.status}
                    </span>
                  </td>
                  <td class="mono">{rev.chart}</td>
                  <td>{rev.app_version || '—'}</td>
                  <td class="updated">{formatTimestamp(rev.updated)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
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

  .muted { color: #6e7681; font-size: 0.875rem; }
  .tab-content { margin-top: 0.5rem; }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }
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
  tbody tr.active { background: rgba(88, 166, 255, 0.08); }
  tbody td { padding: 0.5rem 0.75rem; white-space: nowrap; }
  .center { text-align: center; }
  .mono { font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 0.8rem; }
  .status-badge { font-weight: 500; }
  .updated { color: #8b949e; font-size: 0.8rem; }

  .placeholder-card {
    text-align: center;
    padding: 3rem 2rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    margin-top: 1rem;
  }
  .placeholder-icon {
    font-size: 2rem;
    display: block;
    margin-bottom: 0.75rem;
  }
  .placeholder-card p {
    margin: 0.25rem 0;
    color: #8b949e;
  }
</style>
