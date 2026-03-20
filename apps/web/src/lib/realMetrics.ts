import { browser } from '$app/environment';
import { checkMetricsAvailable, getNodeMetrics, getResources } from '$lib/api';
import type { NodeMetricsData, ResourceEntry } from '$lib/tauri-commands';
import { derived, readonly, writable } from 'svelte/store';

const HISTORY_LIMIT = 20;
const POLL_INTERVAL_MS = 5_000;

export interface Metrics {
  cpuUsage: number[];
  memoryUsage: number[];
  restarts: number[];
}

function createHistoryBuffer(fillValue = 0): number[] {
  return Array.from({ length: HISTORY_LIMIT }, () => fillValue);
}

function createInitialMetrics(): Metrics {
  return {
    cpuUsage: createHistoryBuffer(),
    memoryUsage: createHistoryBuffer(),
    restarts: createHistoryBuffer(),
  };
}

function roundToSingleDecimal(value: number): number {
  return Math.round(value * 10) / 10;
}

function pushHistory(history: number[], value: number): number[] {
  return [...history.slice(-(HISTORY_LIMIT - 1)), value];
}

function averageNodePercent(nodes: NodeMetricsData[], key: 'cpu_percent' | 'memory_percent'): number {
  if (nodes.length === 0) {
    return 0;
  }

  const total = nodes.reduce((sum, node) => sum + (Number.isFinite(node[key]) ? node[key] : 0), 0);
  return roundToSingleDecimal(total / nodes.length);
}

function sumPodRestarts(pods: ResourceEntry[]): number {
  let totalRestarts = 0;

  for (const pod of pods) {
    try {
      const parsed = JSON.parse(pod.content);
      const statuses = parsed?.status?.containerStatuses;
      if (!Array.isArray(statuses)) {
        continue;
      }

      totalRestarts += statuses.reduce((sum: number, status: { restartCount?: unknown }) => {
        return sum + (typeof status?.restartCount === 'number' ? status.restartCount : 0);
      }, 0);
    } catch {
      // Ignore malformed cached resources and continue with the rest.
    }
  }

  return totalRestarts;
}

const metricsStore = writable<Metrics>(createInitialMetrics());
const metricsAvailableStore = writable(false);

let pollTimer: ReturnType<typeof setInterval> | null = null;
let inFlightPoll: Promise<void> | null = null;

async function pollMetricsOnce(): Promise<void> {
  if (inFlightPoll) {
    return inFlightPoll;
  }

  inFlightPoll = (async () => {
    const available = await checkMetricsAvailable();
    metricsAvailableStore.set(available);

    if (!available) {
      metricsStore.set(createInitialMetrics());
      return;
    }

    try {
      const [nodeMetrics, pods] = await Promise.all([getNodeMetrics(), getResources('v1/Pod')]);
      const nextCpu = averageNodePercent(nodeMetrics, 'cpu_percent');
      const nextMemory = averageNodePercent(nodeMetrics, 'memory_percent');
      const nextRestarts = sumPodRestarts(pods);

      metricsStore.update((current) => ({
        cpuUsage: pushHistory(current.cpuUsage, nextCpu),
        memoryUsage: pushHistory(current.memoryUsage, nextMemory),
        restarts: pushHistory(current.restarts, nextRestarts),
      }));
    } catch {
      // Keep the last known values if the real metrics fetch fails.
    }
  })().finally(() => {
    inFlightPoll = null;
  });

  return inFlightPoll;
}

export function startMetricsPolling(): void {
  if (!browser || pollTimer) {
    return;
  }

  void pollMetricsOnce();
  pollTimer = setInterval(() => {
    void pollMetricsOnce();
  }, POLL_INTERVAL_MS);
}

export function stopMetricsPolling(): void {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

export const metrics = readonly(metricsStore);
export const metricsAvailable = readonly(metricsAvailableStore);

export const cpuCurrent = derived(metricsStore, ($metrics) => {
  const latest = $metrics.cpuUsage[$metrics.cpuUsage.length - 1] ?? 0;
  return roundToSingleDecimal(latest);
});

export const memoryCurrent = derived(metricsStore, ($metrics) => {
  const latest = $metrics.memoryUsage[$metrics.memoryUsage.length - 1] ?? 0;
  return roundToSingleDecimal(latest);
});

export const restartsCurrent = derived(metricsStore, ($metrics) => {
  return $metrics.restarts[$metrics.restarts.length - 1] ?? 0;
});
