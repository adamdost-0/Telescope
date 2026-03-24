<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import {
    applyDynamicResource,
    deleteDynamicResource,
    listDynamicResources,
  } from '$lib/api';
  import { resourceDetailHref } from '$lib/resource-routing';
  import { selectedNamespace, isConnected } from '$lib/stores';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import YamlEditor from '$lib/components/YamlEditor.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  const group = $derived(decodeURIComponent(page.params.group));
  const kind = $derived(decodeURIComponent(page.params.kind));

  const version = $derived(page.url.searchParams.get('version') ?? 'v1');
  const plural = $derived(page.url.searchParams.get('plural') ?? kind.toLowerCase() + 's');
  const scope = $derived(page.url.searchParams.get('scope') ?? 'Namespaced');
  const isNamespaced = $derived(scope === 'Namespaced' || scope === '"Namespaced"');
  const gvk = $derived(`${group}/${version}/${kind}`);

  let resources: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;

  let showCreateEditor = $state(false);
  let createManifest = $state('');
  let applying = $state(false);
  let actionMessage: string | null = $state(null);
  let actionError = $state(false);
  let pendingDelete: ResourceEntry | null = $state(null);
  let deleting = $state(false);

  function formatTimestamp(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    return `Updated ${Math.floor(seconds / 60)}m ago`;
  }

  function defaultManifest(): string {
    const lines = [
      `apiVersion: ${group}/${version}`,
      `kind: ${kind}`,
      'metadata:',
      '  name: ""',
    ];
    if (isNamespaced) {
      lines.push(`  namespace: ${$selectedNamespace}`);
    }
    lines.push('spec: {}');
    return lines.join('\n');
  }

  function openCreateEditor() {
    createManifest = defaultManifest();
    showCreateEditor = true;
    actionMessage = null;
    actionError = false;
  }

  async function loadResources() {
    if (!$isConnected) {
      loading = false;
      resources = [];
      return;
    }
    const isInitial = resources.length === 0 && !lastUpdated;
    if (isInitial) loading = true;
    else refreshing = true;
    error = null;
    try {
      const ns = isNamespaced ? $selectedNamespace : undefined;
      resources = await listDynamicResources(group, version, plural, ns);
      lastUpdated = new Date();
      lastUpdatedText = formatTimestamp();
    } catch (e) {
      error = e instanceof Error ? e.message : `Failed to load ${kind} resources`;
      resources = [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  async function handleCreateApply(dryRun: boolean) {
    applying = true;
    actionMessage = null;
    actionError = false;
    try {
      const result = await applyDynamicResource(
        group,
        version,
        kind,
        plural,
        isNamespaced ? $selectedNamespace : null,
        createManifest,
        dryRun,
      );
      actionMessage = result.message;
      actionError = !result.success;
      if (result.success && !dryRun) {
        showCreateEditor = false;
        await loadResources();
      }
    } catch (e) {
      actionMessage = e instanceof Error ? e.message : String(e);
      actionError = true;
    } finally {
      applying = false;
    }
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    deleting = true;
    actionMessage = null;
    actionError = false;
    try {
      actionMessage = await deleteDynamicResource(
        group,
        version,
        kind,
        plural,
        pendingDelete.namespace || null,
        pendingDelete.name,
      );
      pendingDelete = null;
      await loadResources();
    } catch (e) {
      actionMessage = e instanceof Error ? e.message : String(e);
      actionError = true;
    } finally {
      deleting = false;
    }
  }

  $effect(() => {
    void gvk;
    void plural;
    void $selectedNamespace;
    void $isConnected;
    loadResources();
  });

  onMount(() => {
    refreshTimer = setInterval(loadResources, 3000);
    timestampTimer = setInterval(() => {
      lastUpdatedText = formatTimestamp();
    }, 1000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<div class="resource-page">
  <header>
    <div>
      <h1>{kind}</h1>
      <p class="subtitle">{group}/{version} · {scope.replace(/"/g, '')}</p>
    </div>
    <div class="controls">
      {#if isNamespaced}
        <span class="ns-label">Namespace: <strong>{$selectedNamespace}</strong></span>
      {/if}
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" class="secondary" onclick={openCreateEditor} disabled={!$isConnected}>＋ Create</button>
      <button type="button" onclick={loadResources} disabled={refreshing} class:spinning={refreshing} aria-label="Refresh {kind}">
        <span class="refresh-icon">↻</span> Refresh
      </button>
    </div>
  </header>

  <nav class="breadcrumb">
    <a href="/crds">CRDs</a> <span>›</span> <span>{kind}</span>
  </nav>

  {#if actionMessage}
    <p class={actionError ? 'message error-message' : 'message success-message'}>{actionMessage}</p>
  {/if}

  {#if showCreateEditor}
    <section class="create-panel">
      <div class="create-header">
        <div>
          <h2>Create {kind}</h2>
          <p>Server-side apply will create or update the custom resource instance.</p>
        </div>
        <button type="button" class="secondary" onclick={() => (showCreateEditor = false)}>Close</button>
      </div>
      <div class="yaml-actions">
        <button type="button" onclick={() => handleCreateApply(true)} disabled={applying} data-testid="crd-dry-run">
          <Icon name="dry-run" size={16} aria-hidden="true" /> Dry Run
        </button>
        <button type="button" class="primary" onclick={() => handleCreateApply(false)} disabled={applying} data-testid="crd-apply">
          <Icon name="apply" size={16} aria-hidden="true" /> Apply
        </button>
      </div>
      <YamlEditor content={createManifest} onchange={(value) => (createManifest = value)} />
    </section>
  {/if}

  {#if !$isConnected && !loading}
    <div class="not-connected">
      <p>Not connected to a cluster</p>
      <p class="hint">Select a context from the header to connect.</p>
    </div>
  {:else if loading}
    <LoadingSkeleton rows={8} columns={4} />
  {:else if error}
    <div role="alert" aria-live="polite" class="error-container">
      <p class="error">Failed to load {kind} resources.</p>
      {#if error !== `Failed to load ${kind} resources`}
        <p class="error-detail">{error}</p>
      {/if}
      <button type="button" onclick={loadResources}>Retry</button>
    </div>
  {:else}
    <p class="count">{resources.length} {kind.toLowerCase()}{resources.length !== 1 ? 's' : ''}</p>
    {#if resources.length === 0}
      <p class="empty">No {kind} resources found.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Namespace</th>
              <th>Age</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each resources as entry (entry.namespace + '/' + entry.name)}
              {@const content = JSON.parse(entry.content)}
              <tr>
                <td>
                  <a
                    href={resourceDetailHref({
                      gvk,
                      namespace: entry.namespace || null,
                      name: entry.name,
                      label: kind,
                      extraParams: { plural },
                    })}
                  >
                    {entry.name}
                  </a>
                </td>
                <td>{entry.namespace || '—'}</td>
                <td>
                  {#if content?.metadata?.creationTimestamp}
                    {Math.floor((Date.now() - new Date(content.metadata.creationTimestamp).getTime()) / 1000) < 60
                      ? `${Math.floor((Date.now() - new Date(content.metadata.creationTimestamp).getTime()) / 1000)}s`
                      : Math.floor((Date.now() - new Date(content.metadata.creationTimestamp).getTime()) / 60000) < 60
                        ? `${Math.floor((Date.now() - new Date(content.metadata.creationTimestamp).getTime()) / 60000)}m`
                        : Math.floor((Date.now() - new Date(content.metadata.creationTimestamp).getTime()) / 3600000) < 24
                          ? `${Math.floor((Date.now() - new Date(content.metadata.creationTimestamp).getTime()) / 3600000)}h`
                          : `${Math.floor((Date.now() - new Date(content.metadata.creationTimestamp).getTime()) / 86400000)}d`}
                  {:else}
                    Unknown
                  {/if}
                </td>
                <td>
                  <button type="button" class="danger" onclick={() => (pendingDelete = entry)}>Delete</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<ConfirmDialog
  open={!!pendingDelete}
  title={`Delete ${kind}`}
  message={pendingDelete ? `Delete ${pendingDelete.name}${pendingDelete.namespace ? ` from namespace ${pendingDelete.namespace}` : ''}?` : ''}
  confirmText={deleting ? 'Deleting…' : 'Delete'}
  confirmValue={pendingDelete?.name ?? ''}
  requireType={true}
  onconfirm={confirmDelete}
  oncancel={() => {
    if (!deleting) pendingDelete = null;
  }}
/>

<style>
  .resource-page { padding: 1rem; color: #e0e0e0; background: #0f0f23; min-height: 100vh; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; gap: 1rem; }
  h1 { font-size: 1.5rem; margin: 0; }
  h2 { margin: 0; }
  .subtitle { margin: 0.125rem 0 0; font-size: 0.8rem; color: #8b949e; }
  .controls { display: flex; gap: 0.75rem; align-items: center; flex-wrap: wrap; }
  .last-updated { color: #757575; font-size: 0.75rem; }
  .ns-label { color: #9e9e9e; font-size: 0.85rem; }
  button { background: #1a73e8; color: white; border: none; padding: 0.375rem 0.75rem; border-radius: 4px; cursor: pointer; display: inline-flex; align-items: center; gap: 0.25rem; }
  button:hover { background: #1565c0; }
  button:disabled { opacity: 0.6; cursor: not-allowed; }
  button.secondary { background: #30363d; }
  button.secondary:hover { background: #3b434c; }
  button.primary { background: #238636; }
  button.primary:hover { background: #2ea043; }
  button.danger { background: #da3633; }
  button.danger:hover { background: #f85149; }
  .refresh-icon { display: inline-block; }
  .spinning .refresh-icon { animation: spin 1s linear infinite; }
  @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
  .count { color: #9e9e9e; margin-bottom: 0.5rem; font-size: 0.875rem; }
  .error-container { padding: 1.5rem; text-align: center; }
  .error { color: #ef5350; }
  .error-detail { color: #9e9e9e; font-size: 0.875rem; margin-top: 0.25rem; }
  .not-connected { text-align: center; padding: 3rem 1rem; color: #757575; }
  .not-connected p { margin: 0.25rem 0; font-size: 1.125rem; }
  .not-connected .hint { font-size: 0.875rem; color: #616161; }
  .breadcrumb { margin-bottom: 1rem; font-size: 0.8rem; color: #8b949e; display: flex; gap: 0.35rem; align-items: center; }
  .breadcrumb a { color: #58a6ff; text-decoration: none; }
  .breadcrumb a:hover { text-decoration: underline; }
  .create-panel {
    border: 1px solid #30363d;
    background: #161b22;
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1rem;
  }
  .create-header { display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start; margin-bottom: 0.75rem; }
  .create-header p { margin: 0.25rem 0 0; color: #8b949e; }
  .yaml-actions { display: flex; gap: 0.75rem; margin-bottom: 0.75rem; }
  .message { margin: 0 0 1rem; padding: 0.75rem 1rem; border-radius: 6px; }
  .success-message { background: rgba(46, 160, 67, 0.15); color: #3fb950; }
  .error-message { background: rgba(248, 81, 73, 0.15); color: #f85149; }
  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  thead th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #30363d;
    color: #8b949e;
    font-weight: 600;
    white-space: nowrap;
  }
  tbody td {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #21262d;
    white-space: nowrap;
  }
  tbody tr:hover { background: #161b22; }
  a { color: #58a6ff; text-decoration: none; }
  a:hover { text-decoration: underline; }
  .empty { color: #757575; padding: 2rem 0; text-align: center; }
</style>
