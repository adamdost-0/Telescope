const BINARY_UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'] as const;
const BINARY_BASE = 1024;

// Hysteresis thresholds to reduce unit-transition jitter
const CPU_DECIMAL_THRESHOLD = 100; // Show decimals below this value
const MEMORY_UNIT_HYSTERESIS = 0.95; // Stay in lower unit until 95% of next threshold

export function formatCpuMillicores(millicores: number | null | undefined): string {
  if (millicores === null || millicores === undefined || !Number.isFinite(millicores)) {
    return '—';
  }

  // Use 1 decimal place for small values to reduce jitter, round for larger values
  if (millicores < CPU_DECIMAL_THRESHOLD) {
    return `${millicores.toFixed(1)}m`;
  }

  return `${Math.round(millicores)}m`;
}

export function formatPercent(percent: number | null | undefined): string {
  if (percent === null || percent === undefined || !Number.isFinite(percent)) {
    return '—';
  }

  return `${percent.toFixed(1)}%`;
}

export function formatBinaryBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes)) {
    return '—';
  }

  const isNegative = bytes < 0;
  let value = Math.abs(bytes);
  let unitIndex = 0;

  // Apply hysteresis: only move to next unit when value clearly exceeds threshold
  const threshold = BINARY_BASE * MEMORY_UNIT_HYSTERESIS;
  while (value >= threshold && unitIndex < BINARY_UNITS.length - 1) {
    value /= BINARY_BASE;
    unitIndex += 1;
  }

  const fractionDigits = unitIndex === 0 ? 0 : 1;
  const formatted = value.toFixed(fractionDigits);

  return `${isNegative ? '-' : ''}${formatted} ${BINARY_UNITS[unitIndex]}`;
}
