<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { listHelmReleases, getHelmReleaseHistory, getHelmReleaseValues, helmRollback } from '$lib/api';
  import Tabs from '$lib/components/Tabs.svelte';
  import YamlEditor from '$lib/components/YamlEditor.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import Breadcrumbs from '$lib/components/Breadcrumbs.svelte';
  import type { HelmRelease } from '$lib/tauri-commands';

  let releaseName = $derived(page.params.name);
  let namespace = $derived(page.params.namespace);

  let release: HelmRelease | null = $state(null);
  let history: HelmRelease[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let activeTab = $state('info');

  // Values tab state
  let valuesYaml = $state('');
  let editedValues = $state('');
  let valuesLoading = $state(false);
  let valuesError: string | null = $state(null);
  let copied = $state(false);
  let valuesRevealed = $state(false);
  let showRevealDialog = $state(false);

  // Rollback state
  let rollbackTarget: HelmRelease | null = $state(null);
  let showRollbackDialog = $state(false);
  let rollbackLoading = $state(false);
  let rollbackResult: { ok: boolean; message: string } | null = $state(null);

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

  async function loadValues(reveal = false) {
    valuesLoading = true;
    valuesError = null;
    try {
      const yaml = await getHelmReleaseValues(namespace, releaseName, reveal);
      valuesYaml = yaml;
      editedValues = yaml;
      valuesRevealed = reveal;
    } catch (e) {
      valuesError = e instanceof Error ? e.message : 'Failed to load values';
    } finally {
      valuesLoading = false;
    }
  }

  function requestReveal() {
    showRevealDialog = true;
  }

  function confirmReveal() {
    showRevealDialog = false;
    loadValues(true);
  }

  function hideValues() {
    loadValues(false);
  }

  function generateUpgradeCommand(): string {
    return `helm upgrade ${releaseName} <CHART> --namespace ${namespace} --reuse-values -f values.yaml`;
  }

  async function copyUpgradeCommand() {
    try {
      await navigator.clipboard.writeText(generateUpgradeCommand());
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      // Fallback: select the text for manual copy
    }
  }

  function requestRollback(rev: HelmRelease) {
    rollbackTarget = rev;
    rollbackResult = null;
    showRollbackDialog = true;
  }

  async function confirmRollback() {
    if (!rollbackTarget) return;
    rollbackLoading = true;
    rollbackResult = null;
    try {
      const msg = await helmRollback(namespace, releaseName, rollbackTarget.revision);
      rollbackResult = { ok: true, message: msg };
      showRollbackDialog = false;
      await loadRelease();
    } catch (e) {
      rollbackResult = { ok: false, message: e instanceof Error ? e.message : String(e) };
      showRollbackDialog = false;
    } finally {
      rollbackLoading = false;
    }
  }

  $effect(() => {
    if (activeTab === 'values' && !valuesYaml && !valuesLoading) {
      loadValues();
    }
  });

  onMount(loadRelease);
</script>

<div class="detail-page">
  <header class="detail-header">
    <Breadcrumbs crumbs={[
      { label: 'Overview', href: '/overview' },
      { label: 'Helm', href: '/helm' },
      { label: namespace, href: `/helm?namespace=${encodeURIComponent(namespace)}` },
      { label: releaseName }
    ]} />
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
        {#if valuesLoading}
          <p role="status">Loading values…</p>
        {:else if valuesError}
          <p role="alert" class="error">{valuesError}</p>
        {:else}
          <div class="values-toolbar">
            {#if valuesRevealed}
              <div class="reveal-warning">
                <span>⚠️ Sensitive values are now visible</span>
                <button class="hide-btn" onclick={hideValues}>🔒 Hide Sensitive Values</button>
              </div>
            {:else}
              <button class="reveal-btn" onclick={requestReveal}>🔓 Reveal Sensitive Values</button>
            {/if}
          </div>

          <YamlEditor content={valuesYaml} onchange={(v) => { editedValues = v; }} />

          <div class="upgrade-section">
            <div class="upgrade-notice">
              <span class="notice-icon">ℹ️</span>
              <p>Direct upgrade from UI coming in a future release. Save your edited values to a file and use the command below:</p>
            </div>
            <div class="command-row">
              <code class="upgrade-cmd">{generateUpgradeCommand()}</code>
              <button class="copy-btn" onclick={copyUpgradeCommand}>
                {copied ? '✓ Copied' : '📋 Copy Command'}
              </button>
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'history'}
      <div class="tab-content">
        {#if rollbackResult}
          <div class="result-banner" class:success={rollbackResult.ok} class:fail={!rollbackResult.ok}>
            {rollbackResult.ok ? '✅' : '❌'} {rollbackResult.message}
          </div>
        {/if}
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
                <th>Actions</th>
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
                  <td>
                    {#if rev.revision !== release?.revision}
                      <button
                        class="rollback-btn"
                        onclick={() => requestRollback(rev)}
                        disabled={rollbackLoading}
                      >
                        ↩ Rollback
                      </button>
                    {:else}
                      <span class="current-badge">current</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<ConfirmDialog
  open={showRollbackDialog}
  title="Rollback Helm Release"
  message={`Roll back "${releaseName}" in namespace "${namespace}" to revision ${rollbackTarget?.revision ?? '?'}? This will create a new revision with the configuration from revision ${rollbackTarget?.revision ?? '?'}.`}
  confirmText="Rollback"
  confirmValue={releaseName}
  requireType={true}
  productionContext={namespace === 'production' || namespace === 'prod'}
  onconfirm={confirmRollback}
  oncancel={() => { showRollbackDialog = false; }}
/>

<ConfirmDialog
  open={showRevealDialog}
  title="Reveal Sensitive Values"
  message="This will display unredacted passwords, tokens, and other secrets. Make sure no one can see your screen."
  confirmText="Reveal"
  onconfirm={confirmReveal}
  oncancel={() => { showRevealDialog = false; }}
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

  .upgrade-section {
    margin-top: 1rem;
    padding: 1rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
  }
  .upgrade-notice {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .notice-icon { font-size: 1.1rem; flex-shrink: 0; }
  .upgrade-notice p { margin: 0; color: #8b949e; font-size: 0.85rem; }
  .command-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .upgrade-cmd {
    flex: 1;
    background: #0d1117;
    color: #79c0ff;
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    font-family: 'SF Mono', 'Fira Code', Consolas, monospace;
    font-size: 0.8rem;
    border: 1px solid #21262d;
    word-break: break-all;
  }
  .copy-btn {
    background: #21262d;
    color: #c9d1d9;
    border: 1px solid #30363d;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .copy-btn:hover { background: #30363d; }

  .rollback-btn {
    background: transparent;
    color: #f0883e;
    border: 1px solid #f0883e;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .rollback-btn:hover { background: rgba(240, 136, 62, 0.15); }
  .rollback-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .current-badge {
    color: #4caf50;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .result-banner {
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    font-size: 0.85rem;
    margin-bottom: 0.75rem;
  }
  .result-banner.success {
    background: rgba(76, 175, 80, 0.12);
    border: 1px solid rgba(76, 175, 80, 0.3);
    color: #4caf50;
  }
  .result-banner.fail {
    background: rgba(239, 83, 80, 0.12);
    border: 1px solid rgba(239, 83, 80, 0.3);
    color: #ef5350;
  }

  .values-toolbar {
    display: flex;
    align-items: center;
    margin-bottom: 0.75rem;
  }
  .reveal-btn {
    background: #21262d;
    color: #c9d1d9;
    border: 1px solid #30363d;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .reveal-btn:hover { background: #30363d; }
  .hide-btn {
    background: transparent;
    color: #58a6ff;
    border: 1px solid #58a6ff;
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    white-space: nowrap;
    margin-left: 0.75rem;
  }
  .hide-btn:hover { background: rgba(88, 166, 255, 0.1); }
  .reveal-warning {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgba(255, 193, 7, 0.1);
    border: 1px solid rgba(255, 193, 7, 0.3);
    color: #ffc107;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }
</style>
