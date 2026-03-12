import { describe, it, expect } from 'vitest';
import { isProductionContext } from './prod-detection';

describe('isProductionContext', () => {
  it.each([
    'my-prod-cluster',
    'aks-production-eastus',
    'prd-cluster-01',
    'live-api-server',
    'PRODUCTION',
    'cluster-PRD',
    'west-LIVE-2',
  ])('returns true for "%s"', (name) => {
    expect(isProductionContext(name)).toBe(true);
  });

  it.each([
    'dev-cluster',
    'staging-east',
    'test-cluster',
    'my-team-preview',
    'uat-01',
    'local',
    '',
  ])('returns false for "%s"', (name) => {
    expect(isProductionContext(name)).toBe(false);
  });
});
