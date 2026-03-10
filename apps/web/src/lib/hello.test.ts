import { describe, it, expect } from 'vitest';
import { hello } from './hello';

describe('hello', () => {
  it('greets with name', () => {
    expect(hello('telescope')).toBe('hello telescope');
  });

  it('handles blank input', () => {
    expect(hello('')).toBe('hello');
    expect(hello('   ')).toBe('hello');
  });
});
