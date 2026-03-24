<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { createNamespace, deleteNamespace, listNamespaces, setNamespace } from '$lib/api';
  import { getPreferredNamespace, getAutoRefreshIntervalMs } from '$lib/preferences';
  import { isConnected, isProduction, namespaces as namespaceStore, selectedNamespace } from '$lib/stores';
  import Icon from '$lib/icons/Icon.svelte';

  const PROTECTED_NAMESPACES = new Set(['default', 'kube-public', 'kube-system']);
  const NAMESPACE_NAME_PATTERN = /^[a-z0-9](?:[-a-z0-9]{0,61}[a-z0-9])?$/;

  type NamespaceRow = {
    name: string;
    status: string;
    age: string;
    protected: boolean;
  };

  let rows: NamespaceRow[] = $state([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let creating = $state(false);
  let deleting = $state(false);
  let error: string | null = $state(null);
  let success: string | null = $state(null);
  let lastUpdated: Date | null = $state(null);
  let lastUpdatedText = $state('');
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let timestampTimer: ReturnType<typeof setInterval> | null = null;
  let createDialogOpen = $state(false);
  let newNamespaceName = $state('');
  let pendingDelete = $state<string | null>(null);
  let destroyed = false;

  const canCreate = $derived(
    !creating && NAMESPACE_NAME_PATTERN.test(newNamespaceName.trim()) && newNamespaceName.trim().length <= 63,
  );

  function formatUpdatedAt(): string {
    if (!lastUpdated) return '';
    const seconds = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    return `Updated ${minutes}m ago`;
  }

  function namespaceRows(namespaces: string[]): NamespaceRow[] {
    return namespaces
      .slice()
      .sort((left, right) => left.localeCompare(right))
      .map((name) => ({
        name,
        status: '—',
        age: '—',
        protected: PROTECTED_NAMESPACES.has(name),
      }));
  }

  async function syncNamespaceSelection(names: string[]) {
    namespaceStore.set(names);

    if (names.includes($selectedNamespace)) {
      return;
    }

    const nextNamespace = await getPreferredNamespace(names);
    selectedNamespace.set(nextNamespace);
    await setNamespace(nextNamespace);
  }

  async function loadNamespaces() {
    if (!$isConnected) {
      loading = false;
      refreshing = false;
      rows = [];
      lastUpdated = null;
      lastUpdatedText = '';
      error = null;
      success = null;
      namespaceStore.set(['default']);
      return;
    }

    const isInitial = rows.length === 0 && !lastUpdated;
    if (isInitial) {
      loading = true;
    } else {
      refreshing = true;
    }

    error = null;

    try {
      const names = await listNamespaces();
      rows = namespaceRows(names);
      await syncNamespaceSelection(names);
      lastUpdated = new Date();
      lastUpdatedText = formatUpdatedAt();
    } catch (err) {
      rows = [];
      error = err instanceof Error ? err.message : 'Failed to load namespaces';
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  async function handleCreateNamespace() {
    const name = newNamespaceName.trim();
    if (!NAMESPACE_NAME_PATTERN.test(name) || name.length > 63) {
      error =
        'Namespace names must be 1-63 characters of lowercase letters, numbers, or hyphens, starting and ending with an alphanumeric character.';
      return;
    }

    creating = true;
    error = null;
    success = null;

    try {
      const created = await createNamespace(name);
      success = `Created namespace ${created}.`;
      newNamespaceName = '';
      createDialogOpen = false;
      await loadNamespaces();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to create namespace';
    } finally {
      creating = false;
    }
  }

  async function handleDeleteNamespace() {
    if (!pendingDelete || deleting) {
      return;
    }

    const name = pendingDelete;
    deleting = true;
    error = null;
    success = null;

    try {
      const deleted = await deleteNamespace(name);
      success = `Deleted namespace ${deleted}.`;
      pendingDelete = null;
      await loadNamespaces();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to delete namespace';
    } finally {
      deleting = false;
    }
  }

  function openCreateDialog() {
    newNamespaceName = '';
    error = null;
    success = null;
    createDialogOpen = true;
  }

  $effect(() => {
    void $isConnected;
    loadNamespaces();
  });

  onMount(() => {
    void (async () => {
      const refreshIntervalMs = await getAutoRefreshIntervalMs(5000);
      if (!destroyed) {
        refreshTimer = setInterval(loadNamespaces, refreshIntervalMs);
      }
    })();

    timestampTimer = setInterval(() => {
      lastUpdatedText = formatUpdatedAt();
    }, 1000);
  });

  onDestroy(() => {
    destroyed = true;
    if (refreshTimer) clearInterval(refreshTimer);
    if (timestampTimer) clearInterval(timestampTimer);
  });
</script>

<svelte:head>
  <title>Namespaces · Telescope</title>
</svelte:head>

<div class="namespaces-page">
  <header class="page-header">
    <div>
      <h1>Namespaces</h1>
      <p class="subtitle">Create, inspect, and delete cluster namespaces.</p>
    </div>

    <div class="actions">
      {#if lastUpdatedText}
        <span class="last-updated">{lastUpdatedText}</span>
      {/if}
      <button type="button" class="btn btn-secondary" onclick={loadNamespaces} disabled={refreshing}>
        {#if refreshing}
          Refreshing…
        {:else}
          <Icon name="reload" size={14} aria-hidden="true" /> Refresh
        {/if}
      </button>
      <button type="button" class="btn btn-primary" onclick={openCreateDialog} disabled={!$isConnected}>
        <Icon name="create" size={16} aria-hidden="true" /> Create Namespace
      </button>
    </div>
  </header>

  {#if success}
    <div class="notice success">{success}</div>
  {/if}

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  {#if !$isConnected && !loading}
    <div class="empty-state">
      <p>Not connected to a cluster</p>
      <p class="hint">Select a context from the header to manage namespaces.</p>
    </div>
  {:else if loading}
    <div class="empty-state">
      <p>Loading namespaces…</p>
    </div>
  {:else}
    <div class="summary">
      <span>{rows.length} namespace{rows.length === 1 ? '' : 's'}</span>
      <span>Current: <strong>{$selectedNamespace}</strong></span>
    </div>

    <div class="table-wrapper">
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Status</th>
            <th>Age</th>
            <th aria-label="Actions"></th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.name)}
            <tr>
              <td>
                <div class="name-cell">
                  <span>{row.name}</span>
                  {#if row.name === $selectedNamespace}
                    <span class="badge current">Current</span>
                  {/if}
                  {#if row.protected}
                    <span class="badge protected">Protected</span>
                  {/if}
                </div>
              </td>
              <td>{row.status}</td>
              <td>{row.age}</td>
              <td class="actions-cell">
                <button
                  type="button"
                  class="btn btn-danger"
                  disabled={row.protected || deleting}
                  onclick={() => (pendingDelete = row.name)}
                  title={row.protected ? 'System namespaces cannot be deleted from Telescope.' : undefined}
                >
                  Delete
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

{#if createDialogOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" role="presentation" onclick={(event) => event.target === event.currentTarget && !creating && (createDialogOpen = false)}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="create-namespace-title"
      aria-describedby="create-namespace-description"
    >
      <h2 id="create-namespace-title">Create Namespace</h2>
      <p id="create-namespace-description">
        Enter a Kubernetes namespace name. Use lowercase letters, numbers, and hyphens only.
      </p>

      <label class="field">
        <span>Name</span>
        <input
          type="text"
          bind:value={newNamespaceName}
          placeholder="team-a"
          maxlength="63"
          disabled={creating}
        />
      </label>

      <div class="modal-actions">
        <button type="button" class="btn btn-secondary" onclick={() => (createDialogOpen = false)} disabled={creating}>
          Cancel
        </button>
        <button type="button" class="btn btn-primary" onclick={handleCreateNamespace} disabled={!canCreate}>
          {creating ? 'Creating…' : 'Create'}
        </button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  open={pendingDelete !== null}
  title="Delete namespace"
  message={
    pendingDelete
      ? `Delete namespace "${pendingDelete}"? This will remove namespaced resources and may disrupt workloads.`
      : 'Delete this namespace?'
  }
  confirmText={deleting ? 'Deleting…' : 'Delete namespace'}
  confirmValue={pendingDelete ?? ''}
  requireType={true}
  productionContext={$isProduction}
  onconfirm={handleDeleteNamespace}
  oncancel={() => !deleting && (pendingDelete = null)}
/>

<style>
  .namespaces-page {
    padding: 1.5rem 2rem;
    color: #e0e0e0;
    min-height: 100vh;
    background: #0f0f23;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  h1 {
    margin: 0 0 0.25rem;
    font-size: 1.5rem;
  }

  .subtitle {
    margin: 0;
    color: #8b949e;
    font-size: 0.9rem;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .last-updated {
    color: #8b949e;
    font-size: 0.8rem;
  }

  .btn {
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 0.5rem 0.9rem;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn-primary {
    background: #1a73e8;
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    background: #1565c0;
  }

  .btn-secondary {
    background: #21262d;
    border-color: #30363d;
    color: #c9d1d9;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #30363d;
  }

  .btn-danger {
    background: rgba(239, 83, 80, 0.15);
    border-color: rgba(239, 83, 80, 0.35);
    color: #ef5350;
  }

  .btn-danger:hover:not(:disabled) {
    background: rgba(239, 83, 80, 0.25);
  }

  .notice {
    border-radius: 8px;
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }

  .notice.success {
    background: rgba(102, 187, 106, 0.1);
    border: 1px solid rgba(102, 187, 106, 0.3);
    color: #81c784;
  }

  .notice.error {
    background: rgba(239, 83, 80, 0.12);
    border: 1px solid rgba(239, 83, 80, 0.3);
    color: #ef9a9a;
  }

  .summary {
    display: flex;
    gap: 1rem;
    margin-bottom: 0.75rem;
    color: #8b949e;
    font-size: 0.9rem;
  }

  .table-wrapper {
    border: 1px solid #21262d;
    border-radius: 12px;
    overflow: hidden;
    background: #111827;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 0.85rem 1rem;
    text-align: left;
    border-bottom: 1px solid #21262d;
  }

  th {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #8b949e;
    background: #0d1117;
  }

  tbody tr:hover {
    background: rgba(88, 166, 255, 0.05);
  }

  tbody tr:last-child td {
    border-bottom: none;
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.15rem 0.5rem;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .badge.current {
    background: rgba(26, 115, 232, 0.18);
    color: #90caf9;
  }

  .badge.protected {
    background: rgba(255, 193, 7, 0.12);
    color: #ffcc80;
  }

  .actions-cell {
    width: 1%;
    white-space: nowrap;
  }

  .empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: #8b949e;
  }

  .hint {
    font-size: 0.9rem;
    color: #6e7681;
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
    z-index: 1000;
  }

  .modal {
    width: min(440px, calc(100vw - 2rem));
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 12px;
    padding: 1.25rem;
  }

  .modal h2 {
    margin: 0 0 0.5rem;
    font-size: 1.15rem;
  }

  .modal p {
    margin: 0 0 1rem;
    color: #8b949e;
    font-size: 0.9rem;
    line-height: 1.5;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }

  .field input {
    border-radius: 8px;
    border: 1px solid #30363d;
    background: #0d1117;
    color: #e0e0e0;
    padding: 0.65rem 0.75rem;
    font-size: 0.95rem;
  }

  .field input:focus {
    outline: none;
    border-color: #58a6ff;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  @media (max-width: 800px) {
    .namespaces-page {
      padding: 1rem;
    }

    .page-header {
      flex-direction: column;
    }

    .summary {
      flex-direction: column;
      gap: 0.35rem;
    }

    .table-wrapper {
      overflow-x: auto;
    }
  }
</style>
