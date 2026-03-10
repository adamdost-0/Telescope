import { describe, it, expect } from 'vitest';
import { version } from './version';

describe('version', () => {
  it('is set', () => {
    expect(version).toBeTruthy();
  });
});
