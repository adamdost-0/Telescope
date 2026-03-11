<script lang="ts">
  import type { ResourceEntry } from '$lib/tauri-commands';

  let { events = [], showObject = true }: { events: ResourceEntry[]; showObject?: boolean } = $props();

  interface ParsedEvent {
    type: string;
    reason: string;
    object: string;
    message: string;
    count: number;
    firstSeen: string;
    lastSeen: string;
  }

  let parsed = $derived(events.map(parseEvent).sort((a, b) => {
    if (a.type !== b.type) return a.type === 'Warning' ? -1 : 1;
    return b.lastSeen.localeCompare(a.lastSeen);
  }));

  function parseEvent(entry: ResourceEntry): ParsedEvent {
    try {
      const e = JSON.parse(entry.content);
      const involved = e.involvedObject;
      const objRef = involved ? `${involved.kind ?? ''}/${involved.name ?? ''}` : '';
      return {
        type: e.type ?? 'Normal',
        reason: e.reason ?? '',
        object: objRef,
        message: e.message ?? '',
        count: e.count ?? 1,
        firstSeen: e.firstTimestamp ?? e.metadata?.creationTimestamp ?? '',
        lastSeen: e.lastTimestamp ?? e.firstTimestamp ?? '',
      };
    } catch {
      return { type: 'Normal', reason: '', object: '', message: '', count: 1, firstSeen: '', lastSeen: '' };
    }
  }

  function formatTime(ts: string): string {
    if (!ts) return '';
    const d = new Date(ts);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${Math.floor(diffHours / 24)}d ago`;
  }
</script>

{#if parsed.length === 0}
  <p class="empty">No events found.</p>
{:else}
  <div class="table-container">
    <table aria-label="Kubernetes events">
      <thead>
        <tr>
          <th scope="col">Type</th>
          <th scope="col">Reason</th>
          {#if showObject}<th scope="col">Object</th>{/if}
          <th scope="col">Message</th>
          <th scope="col">Count</th>
          <th scope="col">Last Seen</th>
        </tr>
      </thead>
      <tbody>
        {#each parsed as evt}
          <tr class={evt.type === 'Warning' ? 'warning-row' : ''}>
            <td>
              <span class="type-badge" class:warning={evt.type === 'Warning'} class:normal={evt.type === 'Normal'}>
                {evt.type}
              </span>
            </td>
            <td class="reason">{evt.reason}</td>
            {#if showObject}<td class="object">{evt.object}</td>{/if}
            <td class="message">{evt.message}</td>
            <td class="count">{evt.count}</td>
            <td class="time">{formatTime(evt.lastSeen)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .empty {
    color: #6e7681;
    font-size: 0.875rem;
    padding: 1rem;
  }

  .table-container {
    overflow-x: auto;
    border: 1px solid #21262d;
    border-radius: 6px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
    font-family: monospace;
  }

  thead {
    background: #1a1a2e;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  th {
    padding: 0.5rem 0.75rem;
    text-align: left;
    color: #8b949e;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-bottom: 1px solid #21262d;
  }

  td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid #2a2a3e;
    color: #e0e0e0;
  }

  tr:hover {
    background: #16213e;
  }

  .warning-row {
    background: rgba(255, 167, 38, 0.05);
  }
  .warning-row:hover {
    background: rgba(255, 167, 38, 0.1);
  }

  .type-badge {
    display: inline-block;
    padding: 0.1rem 0.45rem;
    border-radius: 3px;
    font-size: 0.75rem;
    font-weight: 600;
  }
  .type-badge.warning {
    color: #ffa726;
    background: rgba(255, 167, 38, 0.12);
    border: 1px solid rgba(255, 167, 38, 0.25);
  }
  .type-badge.normal {
    color: #66bb6a;
    background: rgba(102, 187, 106, 0.1);
    border: 1px solid rgba(102, 187, 106, 0.2);
  }

  .reason {
    font-weight: 500;
    white-space: nowrap;
  }

  .object {
    font-family: monospace;
    color: #4fc3f7;
    white-space: nowrap;
  }

  .message {
    max-width: 480px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    text-align: center;
    color: #8b949e;
  }

  .time {
    white-space: nowrap;
    color: #8b949e;
  }
</style>
