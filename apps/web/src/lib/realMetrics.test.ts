import { afterEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import type { NodeMetricsData, ResourceEntry } from './tauri-commands';

const apiMocks = vi.hoisted(() => ({
  checkMetricsAvailable: vi.fn<() => Promise<boolean>>(),
  getNodeMetrics: vi.fn<() => Promise<NodeMetricsData[]>>(),
  getResources: vi.fn<() => Promise<ResourceEntry[]>>(),
}));

vi.mock('$app/environment', () => ({ browser: true }));
vi.mock('$lib/api', () => ({
  checkMetricsAvailable: apiMocks.checkMetricsAvailable,
  getNodeMetrics: apiMocks.getNodeMetrics,
  getResources: apiMocks.getResources,
}));

type RealMetricsModule = typeof import('./realMetrics');

let currentModule: RealMetricsModule | null = null;

function createNodeMetric(name: string, cpuPercent: number, memoryPercent: number): NodeMetricsData {
  return {
    name,
    cpu_millicores: 0,
    memory_bytes: 0,
    cpu_allocatable: 0,
    memory_allocatable: 0,
    cpu_percent: cpuPercent,
    memory_percent: memoryPercent,
  };
}

function createPodResource(name: string, restartCounts: number[]): ResourceEntry {
  return {
    gvk: 'v1/Pod',
    namespace: 'default',
    name,
    resource_version: '1',
    updated_at: '2025-01-01T00:00:00Z',
    content: JSON.stringify({
      status: {
        containerStatuses: restartCounts.map((restartCount, index) => ({
          name: `container-${index}`,
          restartCount,
        })),
      },
    }),
  };
}

async function loadRealMetrics(): Promise<RealMetricsModule> {
  vi.resetModules();
  currentModule = await import('./realMetrics');
  return currentModule;
}

async function flushPoll(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

afterEach(() => {
  currentModule?.stopMetricsPolling();
  currentModule = null;
  vi.clearAllTimers();
  vi.useRealTimers();
  apiMocks.checkMetricsAvailable.mockReset();
  apiMocks.getNodeMetrics.mockReset();
  apiMocks.getResources.mockReset();
});

describe('realMetrics', () => {
  it('averages node percentages and sums pod restart counts', async () => {
    vi.useFakeTimers();
    apiMocks.checkMetricsAvailable.mockResolvedValue(true);
    apiMocks.getNodeMetrics.mockResolvedValue([
      createNodeMetric('node-a', 25, 60),
      createNodeMetric('node-b', 50, 75),
    ]);
    apiMocks.getResources.mockResolvedValue([
      createPodResource('pod-a', [1, 2]),
      createPodResource('pod-b', [3]),
    ]);

    const realMetrics = await loadRealMetrics();
    realMetrics.startMetricsPolling();
    await flushPoll();

    expect(get(realMetrics.metricsAvailable)).toBe(true);
    expect(get(realMetrics.cpuCurrent)).toBe(37.5);
    expect(get(realMetrics.memoryCurrent)).toBe(67.5);
    expect(get(realMetrics.restartsCurrent)).toBe(6);
    expect(get(realMetrics.metrics).cpuUsage).toHaveLength(20);
    expect(get(realMetrics.metrics).cpuUsage.at(-1)).toBe(37.5);
    expect(get(realMetrics.metrics).memoryUsage.at(-1)).toBe(67.5);
    expect(get(realMetrics.metrics).restarts.at(-1)).toBe(6);
  });

  it('falls back to empty history when metrics are unavailable', async () => {
    vi.useFakeTimers();
    apiMocks.checkMetricsAvailable.mockResolvedValueOnce(true).mockResolvedValueOnce(false);
    apiMocks.getNodeMetrics.mockResolvedValue([createNodeMetric('node-a', 42, 84)]);
    apiMocks.getResources.mockResolvedValue([createPodResource('pod-a', [5])]);

    const realMetrics = await loadRealMetrics();
    realMetrics.startMetricsPolling();
    await flushPoll();

    expect(get(realMetrics.metricsAvailable)).toBe(true);
    expect(get(realMetrics.cpuCurrent)).toBe(42);
    expect(get(realMetrics.memoryCurrent)).toBe(84);
    expect(get(realMetrics.restartsCurrent)).toBe(5);

    await vi.advanceTimersByTimeAsync(10_000);
    await flushPoll();

    expect(get(realMetrics.metricsAvailable)).toBe(false);
    expect(get(realMetrics.metrics)).toEqual({
      cpuUsage: Array(20).fill(0),
      memoryUsage: Array(20).fill(0),
      restarts: Array(20).fill(0),
    });
    expect(apiMocks.getNodeMetrics).toHaveBeenCalledTimes(1);
    expect(apiMocks.getResources).toHaveBeenCalledTimes(1);
  });

  it('caps the history ring buffer at 20 entries', async () => {
    vi.useFakeTimers();

    let sample = 0;
    apiMocks.checkMetricsAvailable.mockResolvedValue(true);
    apiMocks.getNodeMetrics.mockImplementation(async () => {
      sample += 1;
      return [createNodeMetric('node-a', sample, sample * 2)];
    });
    apiMocks.getResources.mockResolvedValue([]);

    const realMetrics = await loadRealMetrics();
    realMetrics.startMetricsPolling();
    await flushPoll();

    for (let i = 0; i < 25; i += 1) {
      await vi.advanceTimersByTimeAsync(10_000);
      await flushPoll();
    }

    const { cpuUsage, memoryUsage, restarts } = get(realMetrics.metrics);

    expect(cpuUsage).toHaveLength(20);
    expect(memoryUsage).toHaveLength(20);
    expect(restarts).toHaveLength(20);
    expect(cpuUsage[0]).toBe(7);
    expect(cpuUsage.at(-1)).toBe(26);
    expect(memoryUsage[0]).toBe(14);
    expect(memoryUsage.at(-1)).toBe(52);
    expect(restarts.every((value) => value === 0)).toBe(true);
  });
});
