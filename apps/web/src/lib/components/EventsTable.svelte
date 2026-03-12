<script lang="ts">
  import type { ResourceEntry } from '$lib/tauri-commands';

  let { events = [], showObject = true }: { events: ResourceEntry[]; showObject?: boolean } = $props();

  interface ParsedEvent {
    type: string;
    reason: string;
    namespace: string;
    object: string;
    message: string;
    count: number;
    firstSeen: string;
    lastSeen: string;
  }

  type SortField = 'type' | 'namespace' | 'reason' | 'object' | 'message' | 'count' | 'lastSeen';
  let sortKey = $state<SortField | null>(null);
  let sortDir = $state<'asc' | 'desc'>('asc');

  function toggleSort(key: SortField) {
    if (sortKey !== key) {
      sortKey = key;
      sortDir = 'asc';
    } else if (sortDir === 'asc') {
      sortDir = 'desc';
    } else {
      sortKey = null;
      sortDir = 'asc';
    }
  }

  function sortIndicator(key: SortField): string {
    if (sortKey !== key) return ' ◇';
    return sortDir === 'asc' ? ' ▲' : ' ▼';
  }

  function eventSortValue(evt: ParsedEvent, key: SortField): string | number {
    switch (key) {
      case 'type': return evt.type;
      case 'namespace': return evt.namespace;
      case 'reason': return evt.reason;
      case 'object': return evt.object;
      case 'message': return evt.message;
      case 'count': return evt.count;
      case 'lastSeen': return evt.lastSeen;
    }
  }

  let parsed = $derived.by(() => {
    const base = events.map(parseEvent);

    if (sortKey !== null) {
      const key = sortKey;
      return [...base].sort((a, b) => {
        const aVal = eventSortValue(a, key);
        const bVal = eventSortValue(b, key);
        let cmp: number;
        if (typeof aVal === 'number' && typeof bVal === 'number') {
          cmp = aVal - bVal;
        } else {
          cmp = String(aVal).localeCompare(String(bVal), undefined, { numeric: true, sensitivity: 'base' });
        }
        return sortDir === 'asc' ? cmp : -cmp;
      });
    }

    // Default sort: warnings first, then by lastSeen descending
    return base.sort((a, b) => {
      if (a.type !== b.type) return a.type === 'Warning' ? -1 : 1;
      return b.lastSeen.localeCompare(a.lastSeen);
    });
  });

  function parseEvent(entry: ResourceEntry): ParsedEvent {
    try {
      const e = JSON.parse(entry.content);
      const involved = e.involvedObject;
      const objRef = involved ? `${involved.kind ?? ''}/${involved.name ?? ''}` : '';
      return {
        type: e.type ?? 'Normal',
        reason: e.reason ?? '',
        namespace: e.metadata?.namespace ?? entry.namespace ?? '',
        object: objRef,
        message: e.message ?? '',
        count: e.count ?? 1,
        firstSeen: e.firstTimestamp ?? e.metadata?.creationTimestamp ?? '',
        lastSeen: e.lastTimestamp ?? e.firstTimestamp ?? '',
      };
    } catch {
      return { type: 'Normal', reason: '', namespace: '', object: '', message: '', count: 1, firstSeen: '', lastSeen: '' };
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
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('type')} aria-label="Sort by Type">Type{sortIndicator('type')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('namespace')} aria-label="Sort by Namespace">Namespace{sortIndicator('namespace')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('reason')} aria-label="Sort by Reason">Reason{sortIndicator('reason')}</button></th>
          {#if showObject}<th scope="col"><button class="sort-btn" onclick={() => toggleSort('object')} aria-label="Sort by Object">Object{sortIndicator('object')}</button></th>{/if}
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('message')} aria-label="Sort by Message">Message{sortIndicator('message')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('count')} aria-label="Sort by Count">Count{sortIndicator('count')}</button></th>
          <th scope="col"><button class="sort-btn" onclick={() => toggleSort('lastSeen')} aria-label="Sort by Last Seen">Last Seen{sortIndicator('lastSeen')}</button></th>
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
            <td class="namespace">{evt.namespace}</td>
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

  .sort-btn {
    all: unset;
    cursor: pointer;
    color: inherit;
    font: inherit;
    width: 100%;
    display: inline-flex;
    align-items: center;
    white-space: nowrap;
    user-select: none;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    font-size: 0.75rem;
  }
  .sort-btn:hover {
    color: #58a6ff;
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

  .namespace {
    font-family: monospace;
    color: #8b949e;
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
