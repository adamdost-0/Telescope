<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { getPods, getEvents } from '$lib/api';
  import Tabs from '$lib/components/Tabs.svelte';
  import LogViewer from '$lib/components/LogViewer.svelte';
  import EventsTable from '$lib/components/EventsTable.svelte';
  import type { ResourceEntry } from '$lib/tauri-commands';

  let namespace = $derived(page.params.namespace);
  let podName = $derived(page.params.name);

  let pod: any = $state(null);
  let events: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let activeTab = $state('summary');
  let error: string | null = $state(null);

  const tabs = [
    { id: 'summary', label: 'Summary' },
    { id: 'logs', label: 'Logs' },
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
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load pod';
    } finally {
      loading = false;
    }
  }

  onMount(loadPod);
</script>

<div class="detail-page">
  <header class="detail-header">
    <a href="/pods" class="back">← Pods</a>
    <h1>{podName}</h1>
    <span class="namespace-badge">{namespace}</span>
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
          </div>
        {/each}

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

    {:else if activeTab === 'events'}
      <EventsTable events={events} showObject={false} />

    {:else if activeTab === 'yaml'}
      <pre class="yaml-view"><code>{JSON.stringify(pod, null, 2)}</code></pre>
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
  .yaml-view {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 1rem;
    overflow: auto;
    max-height: 70vh;
    font-size: 0.8rem;
    line-height: 1.5;
    color: #c9d1d9;
  }
</style>
