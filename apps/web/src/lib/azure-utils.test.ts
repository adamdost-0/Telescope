import { describe, it, expect } from 'vitest';
import { parseAksUrl, isAksCluster } from './azure-utils';

describe('isAksCluster', () => {
  it('returns true for standard AKS URL', () => {
    expect(isAksCluster('https://myaks-dns-abc123.hcp.eastus2.azmk8s.io:443')).toBe(true);
  });

  it('returns true for Azure Government AKS URL', () => {
    expect(isAksCluster('https://myaks.hcp.usgovvirginia.azmk8s.us')).toBe(true);
  });

  it('returns true without port', () => {
    expect(isAksCluster('https://myaks.hcp.westeurope.azmk8s.io')).toBe(true);
  });

  it('returns false for non-AKS URL', () => {
    expect(isAksCluster('https://kubernetes.default.svc:443')).toBe(false);
  });

  it('returns false for EKS URL', () => {
    expect(isAksCluster('https://ABC123.gr7.us-east-1.eks.amazonaws.com')).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(isAksCluster('')).toBe(false);
  });
});

describe('parseAksUrl', () => {
  it('extracts cluster name and region from AKS URL', () => {
    const result = parseAksUrl('https://myaks-dns-abc123.hcp.eastus2.azmk8s.io:443');
    expect(result).toEqual({
      subscriptionId: '',
      resourceGroup: '',
      clusterName: 'myaks-dns-abc123',
      region: 'eastus2',
    });
  });

  it('returns null for non-AKS URL', () => {
    expect(parseAksUrl('https://kubernetes.default.svc:443')).toBeNull();
  });

  it('handles http scheme', () => {
    const result = parseAksUrl('http://test.hcp.westus.azmk8s.io');
    expect(result).not.toBeNull();
    expect(result!.region).toBe('westus');
  });

  it('extracts cluster name and region from Azure Government AKS URL', () => {
    const result = parseAksUrl('https://mygovaks.hcp.usgovvirginia.azmk8s.us:443');
    expect(result).toEqual({
      subscriptionId: '',
      resourceGroup: '',
      clusterName: 'mygovaks',
      region: 'usgovvirginia',
    });
  });
});
