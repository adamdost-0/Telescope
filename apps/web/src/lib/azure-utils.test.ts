import { describe, it, expect } from 'vitest';
import { parseAksUrl, isAksCluster, getAzurePortalUrl } from './azure-utils';

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

describe('getAzurePortalUrl', () => {
  it('returns portal URL when subscription and resource group are set', () => {
    const url = getAzurePortalUrl({
      subscriptionId: 'sub-123',
      resourceGroup: 'rg-prod',
      clusterName: 'my-cluster',
      region: 'eastus2',
    });
    expect(url).toBe(
      'https://portal.azure.com/#resource/subscriptions/sub-123/resourceGroups/rg-prod/providers/Microsoft.ContainerService/managedClusters/my-cluster/overview',
    );
  });

  it('returns Azure Government portal URL when requested', () => {
    const url = getAzurePortalUrl(
      {
        subscriptionId: 'sub-123',
        resourceGroup: 'rg-prod',
        clusterName: 'my-cluster',
        region: 'usgovvirginia',
      },
      'UsGovernment',
    );
    expect(url).toBe(
      'https://portal.azure.us/#resource/subscriptions/sub-123/resourceGroups/rg-prod/providers/Microsoft.ContainerService/managedClusters/my-cluster/overview',
    );
  });

  it('returns null when subscriptionId is empty', () => {
    expect(
      getAzurePortalUrl({ subscriptionId: '', resourceGroup: 'rg', clusterName: 'c', region: 'r' }),
    ).toBeNull();
  });

  it('returns null when resourceGroup is empty', () => {
    expect(
      getAzurePortalUrl({ subscriptionId: 'sub', resourceGroup: '', clusterName: 'c', region: 'r' }),
    ).toBeNull();
  });
});
