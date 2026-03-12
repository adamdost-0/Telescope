import { describe, it, expect } from 'vitest';
import { getSuggestion } from './error-suggestions';

describe('getSuggestion', () => {
  it('returns connection refused suggestion', () => {
    expect(getSuggestion('connection refused')).toContain('cluster is running');
    expect(getSuggestion('ECONNREFUSED')).toContain('cluster is running');
  });

  it('returns auth suggestion for 401', () => {
    expect(getSuggestion('Unauthorized')).toContain('credentials');
    expect(getSuggestion('HTTP 401')).toContain('credentials');
  });

  it('returns RBAC suggestion for 403', () => {
    expect(getSuggestion('Forbidden')).toContain('permission');
    expect(getSuggestion('403')).toContain('RBAC');
  });

  it('returns not found suggestion', () => {
    expect(getSuggestion('not found')).toContain('deleted');
    expect(getSuggestion('404')).toContain('refreshing');
  });

  it('returns timeout suggestion', () => {
    expect(getSuggestion('timeout')).toContain('overloaded');
  });

  it('returns kubelogin suggestion', () => {
    expect(getSuggestion('kubelogin')).toContain('PATH');
  });

  it('returns default suggestion for unknown errors', () => {
    expect(getSuggestion('something weird')).toContain('cluster connection');
  });

  it('returns empty string for empty input', () => {
    expect(getSuggestion('')).toBe('');
  });
});
