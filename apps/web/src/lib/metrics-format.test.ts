import { describe, expect, it } from 'vitest';
import { formatBinaryBytes, formatCpuMillicores, formatPercent } from './metrics-format';

describe('formatCpuMillicores', () => {
  it('uses decimals for small values to reduce jitter', () => {
    expect(formatCpuMillicores(5.3)).toBe('5.3m');
    expect(formatCpuMillicores(12.7)).toBe('12.7m');
    expect(formatCpuMillicores(99.9)).toBe('99.9m');
  });

  it('rounds large values for cleaner display', () => {
    expect(formatCpuMillicores(100)).toBe('100m');
    expect(formatCpuMillicores(250.7)).toBe('251m');
    expect(formatCpuMillicores(999.4)).toBe('999m');
    expect(formatCpuMillicores(1000)).toBe('1000m');
    expect(formatCpuMillicores(1000.6)).toBe('1001m');
  });

  it('handles edge cases around decimal threshold', () => {
    expect(formatCpuMillicores(99.5)).toBe('99.5m');
    expect(formatCpuMillicores(100.5)).toBe('101m');
  });

  it('returns em dash when CPU value is missing', () => {
    expect(formatCpuMillicores(null)).toBe('—');
    expect(formatCpuMillicores(undefined)).toBe('—');
    expect(formatCpuMillicores(NaN)).toBe('—');
    expect(formatCpuMillicores(Infinity)).toBe('—');
  });
});

describe('formatBinaryBytes', () => {
  it('formats binary units with KiB support', () => {
    expect(formatBinaryBytes(512)).toBe('512 B');
    expect(formatBinaryBytes(1024)).toBe('1.0 KiB');
    expect(formatBinaryBytes(1024 * 1024)).toBe('1.0 MiB');
    expect(formatBinaryBytes(2 * 1024 * 1024 * 1024)).toBe('2.0 GiB');
  });

  it('uses hysteresis to reduce unit-transition jitter', () => {
    // Values just below threshold stay in lower unit (95% of 1024 = 972.8)
    expect(formatBinaryBytes(900)).toBe('900 B');
    expect(formatBinaryBytes(972)).toBe('972 B');
    
    // Values clearly above threshold move to next unit
    expect(formatBinaryBytes(973)).toBe('1.0 KiB');
    expect(formatBinaryBytes(1000)).toBe('1.0 KiB');
    
    // Same pattern for KiB → MiB (95% of 1048576 = 996147.2)
    // 972 KiB = 995328 bytes, which is below the threshold
    const belowMiB = 972 * 1024;
    expect(formatBinaryBytes(belowMiB)).toBe('972.0 KiB');
    
    // 1000 KiB = 1024000 bytes, which exceeds the threshold
    const overMiB = 1000 * 1024;
    expect(formatBinaryBytes(overMiB)).toBe('1.0 MiB');
  });

  it('handles small and large values correctly', () => {
    expect(formatBinaryBytes(0)).toBe('0 B');
    expect(formatBinaryBytes(1)).toBe('1 B');
    expect(formatBinaryBytes(5 * 1024 * 1024 * 1024 * 1024)).toBe('5.0 TiB');
  });

  it('handles negative values', () => {
    expect(formatBinaryBytes(-512)).toBe('-512 B');
    expect(formatBinaryBytes(-1024)).toBe('-1.0 KiB');
  });

  it('returns em dash when memory value is missing', () => {
    expect(formatBinaryBytes(null)).toBe('—');
    expect(formatBinaryBytes(undefined)).toBe('—');
    expect(formatBinaryBytes(NaN)).toBe('—');
    expect(formatBinaryBytes(Infinity)).toBe('—');
  });
});

describe('formatPercent', () => {
  it('uses one decimal place for stable percentage labels', () => {
    expect(formatPercent(35)).toBe('35.0%');
    expect(formatPercent(35.678)).toBe('35.7%');
    expect(formatPercent(99.95)).toBe('100.0%');
  });

  it('handles edge cases', () => {
    expect(formatPercent(0)).toBe('0.0%');
    expect(formatPercent(0.05)).toBe('0.1%');
    expect(formatPercent(100)).toBe('100.0%');
  });

  it('returns em dash when percent value is missing', () => {
    expect(formatPercent(null)).toBe('—');
    expect(formatPercent(undefined)).toBe('—');
    expect(formatPercent(NaN)).toBe('—');
  });
});
