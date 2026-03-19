import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { listAksNodePools } from './api';

describe('listAksNodePools', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('rethrows list failures instead of falling back to an empty array', async () => {
    invokeMock.mockRejectedValue('ARM lookup failed');

    await expect(listAksNodePools()).rejects.toThrow('ARM lookup failed');
    expect(invokeMock).toHaveBeenCalledWith('list_aks_node_pools', undefined);
  });
});
