/**
 * Ring-buffer store for tracking CPU/memory metrics history over time.
 * Polls metrics every 30s and keeps the last 30 data points per resource.
 */
import { writable, type Writable } from 'svelte/store';

const MAX_POINTS = 30;

export interface MetricsSnapshot {
  cpu: number[];
  memory: number[];
}

type HistoryMap = Map<string, MetricsSnapshot>;

function createMetricsHistoryStore() {
  const { subscribe, update }: Writable<HistoryMap> = writable(new Map());

  function push(key: string, cpu: number, memory: number) {
    update((map) => {
      const entry = map.get(key) ?? { cpu: [], memory: [] };
      entry.cpu.push(cpu);
      entry.memory.push(memory);
      if (entry.cpu.length > MAX_POINTS) entry.cpu.shift();
      if (entry.memory.length > MAX_POINTS) entry.memory.shift();
      map.set(key, entry);
      return new Map(map);
    });
  }

  function get(map: HistoryMap, key: string): MetricsSnapshot {
    return map.get(key) ?? { cpu: [], memory: [] };
  }

  function clear() {
    update(() => new Map());
  }

  return { subscribe, push, get, clear };
}

export const podMetricsHistory = createMetricsHistoryStore();
export const nodeMetricsHistory = createMetricsHistoryStore();
