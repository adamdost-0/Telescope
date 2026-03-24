import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.fn();

function createLocalStorageMock() {
  const store = new Map<string, string>();

  return {
    getItem: vi.fn((key: string) => store.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store.set(key, value);
    }),
    removeItem: vi.fn((key: string) => {
      store.delete(key);
    }),
    clear: vi.fn(() => {
      store.clear();
    }),
  };
}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import {
  getAiInsightsSettings,
  listAksNodePools,
  setAiInsightsSettings,
} from './api';
import {
  AI_INSIGHTS_SETTINGS_KEYS,
  createDefaultAiInsightsSettings,
} from './tauri-commands';

type InvokeArgs = Record<string, unknown> | undefined;

function getPreferenceCalls() {
  return invokeMock.mock.calls
    .filter(([command]) => command === 'get_preference')
    .map(([command, args]) => [command, args]);
}

function setPreferenceCalls() {
  return invokeMock.mock.calls
    .filter(([command]) => command === 'set_preference')
    .map(([command, args]) => [command, args]);
}

describe('listAksNodePools', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.stubGlobal('window', {
      __TAURI_INTERNALS__: {
        invoke: invokeMock,
      },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('rethrows list failures instead of falling back to an empty array', async () => {
    invokeMock.mockRejectedValue('ARM lookup failed');

    await expect(listAksNodePools()).rejects.toThrow('ARM lookup failed');
    expect(invokeMock).toHaveBeenCalledWith('list_aks_node_pools', undefined);
  });
});

describe('AI Insights settings wrappers', () => {
  let localStorageMock: ReturnType<typeof createLocalStorageMock>;

  beforeEach(() => {
    invokeMock.mockReset();
    localStorageMock = createLocalStorageMock();
    vi.stubGlobal('window', {
      __TAURI_INTERNALS__: {
        invoke: invokeMock,
      },
    });
    vi.stubGlobal('localStorage', localStorageMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('round-trips a null model name with optional semantics', async () => {
    const storedPreferences = new Map<string, string | null>();

    invokeMock.mockImplementation(async (command: string, args: InvokeArgs) => {
      if (command === 'get_preference') {
        return storedPreferences.get(String(args?.key)) ?? null;
      }

      if (command === 'set_preference') {
        storedPreferences.set(String(args?.key), String(args?.value));
        return undefined;
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    await setAiInsightsSettings({
      endpoint: 'https://example.openai.azure.com',
      deploymentName: 'insights-deployment',
      authMode: 'apiKey',
      cloudProfile: 'commercial',
      modelName: null,
    });

    expect(storedPreferences.get(AI_INSIGHTS_SETTINGS_KEYS.modelName)).toBe('');

    await expect(getAiInsightsSettings()).resolves.toEqual({
      ...createDefaultAiInsightsSettings('commercial'),
      endpoint: 'https://example.openai.azure.com',
      deploymentName: 'insights-deployment',
      authMode: 'apiKey',
      cloudProfile: 'commercial',
      modelName: null,
    });
  });

  it('returns defaults merged with stored AI Insights preferences', async () => {
    const storedPreferences = new Map<string, string | null>([
      [AI_INSIGHTS_SETTINGS_KEYS.endpoint, 'https://example.openai.azure.com'],
      [AI_INSIGHTS_SETTINGS_KEYS.deploymentName, null],
      [AI_INSIGHTS_SETTINGS_KEYS.authMode, 'apiKey'],
      [AI_INSIGHTS_SETTINGS_KEYS.cloudProfile, 'usGovernment'],
      [AI_INSIGHTS_SETTINGS_KEYS.modelName, 'gpt-4.1'],
    ]);

    invokeMock.mockImplementation(async (command: string, args: InvokeArgs) => {
      if (command === 'get_preference') {
        return storedPreferences.get(String(args?.key)) ?? null;
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    await expect(getAiInsightsSettings()).resolves.toEqual({
      ...createDefaultAiInsightsSettings('usGovernment'),
      endpoint: 'https://example.openai.azure.com',
      authMode: 'apiKey',
      cloudProfile: 'usGovernment',
      modelName: 'gpt-4.1',
    });

    expect(getPreferenceCalls()).toEqual([
      ['get_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.endpoint }],
      ['get_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.deploymentName }],
      ['get_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.authMode }],
      ['get_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.cloudProfile }],
      ['get_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.modelName }],
    ]);
    expect(invokeMock).not.toHaveBeenCalledWith('get_azure_cloud', undefined);
  });

  it('persists the dedicated AI settings keys including model name and cloud profile', async () => {
    invokeMock.mockResolvedValue(undefined);

    await setAiInsightsSettings({
      endpoint: 'https://example.openai.azure.com',
      deploymentName: 'insights-deployment',
      authMode: 'apiKey',
      cloudProfile: 'usGovernmentTopSecret',
      modelName: 'gpt-4.1-mini',
    });

    expect(setPreferenceCalls()).toEqual([
      ['set_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.endpoint, value: 'https://example.openai.azure.com' }],
      ['set_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.deploymentName, value: 'insights-deployment' }],
      ['set_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.authMode, value: 'apiKey' }],
      ['set_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.cloudProfile, value: 'usGovernmentTopSecret' }],
      ['set_preference', { key: AI_INSIGHTS_SETTINGS_KEYS.modelName, value: 'gpt-4.1-mini' }],
    ]);
    expect(invokeMock).not.toHaveBeenCalledWith('set_azure_cloud', expect.anything());
  });

  it('does not touch the shared Azure cloud localStorage path for AI settings', async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'get_preference') {
        return null;
      }

      if (command === 'set_preference') {
        return undefined;
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    await getAiInsightsSettings();
    await setAiInsightsSettings(createDefaultAiInsightsSettings('commercial'));

    expect(localStorageMock.getItem).not.toHaveBeenCalledWith('telescope-azure-cloud');
    expect(localStorageMock.getItem).not.toHaveBeenCalledWith('telescope-azure-cloud-selection');
    expect(localStorageMock.setItem).not.toHaveBeenCalledWith('telescope-azure-cloud', expect.any(String));
    expect(localStorageMock.setItem).not.toHaveBeenCalledWith('telescope-azure-cloud-selection', expect.any(String));
  });
});
