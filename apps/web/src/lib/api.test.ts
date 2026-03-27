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
  clearAiInsightsHistory,
  generateAiInsights,
  getAiInsightsSettings,
  listAksNodePools,
  listAiInsightsHistory,
  onApiError,
  setAiInsightsSettings,
  testAiInsightsConnection,
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

describe('AI Insights generation wrappers', () => {
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

  it('uses exact backend command names for route actions', async () => {
    const connectionResult = {
      normalizedEndpoint: 'https://example.openai.azure.com',
      chatCompletionsUrl: 'https://example.openai.azure.com/openai/deployments/insights/chat/completions?api-version=2024-02-15-preview',
      model: 'gpt-4.1',
    };
    const generatedInsights = {
      summary: 'Cluster is healthy with one medium risk.',
      risks: [{ title: 'Node pressure', detail: 'One node is memory constrained.', impact: 'medium' }],
      observations: [{ area: 'Workloads', detail: 'Deployments are available.' }],
      recommendations: [{ action: 'Investigate pressure', rationale: 'Avoid future evictions.', confidence: 0.77 }],
      references: [{ kind: 'Node', name: 'aks-nodepool-01', namespace: null }],
    };
    const historyEntries = [
      {
        createdAt: '2026-01-01T00:00:00Z',
        scope: { kind: 'cluster' },
        response: generatedInsights,
      },
    ];

    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'test_ai_insights_connection') return connectionResult;
      if (command === 'generate_ai_insights') return generatedInsights;
      if (command === 'list_ai_insights_history') return historyEntries;
      if (command === 'clear_ai_insights_history') return undefined;
      throw new Error(`Unexpected command: ${command}`);
    });

    await expect(testAiInsightsConnection()).resolves.toEqual(connectionResult);
    await expect(generateAiInsights()).resolves.toEqual(generatedInsights);
    await expect(listAiInsightsHistory()).resolves.toEqual(historyEntries);
    await expect(clearAiInsightsHistory()).resolves.toBeUndefined();

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'test_ai_insights_connection', undefined);
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'generate_ai_insights', undefined);
    expect(invokeMock).toHaveBeenNthCalledWith(3, 'list_ai_insights_history', undefined);
    expect(invokeMock).toHaveBeenNthCalledWith(4, 'clear_ai_insights_history', undefined);
  });

  it('passes apiKey to AI invoke commands without persisting or storing it', async () => {
    const localStorageMock = createLocalStorageMock();
    vi.stubGlobal('localStorage', localStorageMock);

    const connectionResult = {
      normalizedEndpoint: 'https://example.openai.azure.com',
      chatCompletionsUrl:
        'https://example.openai.azure.com/openai/deployments/insights/chat/completions?api-version=2024-02-15-preview',
      model: 'gpt-4.1',
    };
    const generatedInsights = {
      summary: 'Cluster is healthy with one medium risk.',
      risks: [{ title: 'Node pressure', detail: 'One node is memory constrained.', impact: 'medium' }],
      observations: [{ area: 'Workloads', detail: 'Deployments are available.' }],
      recommendations: [
        { action: 'Investigate pressure', rationale: 'Avoid future evictions.', confidence: 0.77 },
      ],
      references: [{ kind: 'Node', name: 'aks-nodepool-01', namespace: null }],
    };

    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'test_ai_insights_connection') return connectionResult;
      if (command === 'generate_ai_insights') return generatedInsights;
      throw new Error(`Unexpected command: ${command}`);
    });

    const apiKey = 'test-api-key';
    await expect(testAiInsightsConnection(apiKey)).resolves.toEqual(connectionResult);
    await expect(generateAiInsights(apiKey)).resolves.toEqual(generatedInsights);

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'test_ai_insights_connection', { apiKey });
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'generate_ai_insights', { apiKey });
    expect(getPreferenceCalls()).toEqual([]);
    expect(setPreferenceCalls()).toEqual([]);
    expect(localStorageMock.getItem).not.toHaveBeenCalled();
    expect(localStorageMock.setItem).not.toHaveBeenCalled();
    expect(localStorageMock.removeItem).not.toHaveBeenCalled();
    expect(localStorageMock.clear).not.toHaveBeenCalled();
    expect(console.error).not.toHaveBeenCalled();
  });

  it('notifies API listeners and rethrows wrapper failures', async () => {
    const listener = vi.fn();
    const unsubscribe = onApiError(listener);

    invokeMock
      .mockRejectedValueOnce('connection check failed')
      .mockRejectedValueOnce(new Error('generation failed'))
      .mockRejectedValueOnce({ message: 'history failed' })
      .mockRejectedValueOnce('clear failed');

    await expect(testAiInsightsConnection()).rejects.toThrow('connection check failed');
    await expect(generateAiInsights()).rejects.toThrow('generation failed');
    await expect(listAiInsightsHistory()).rejects.toThrow('history failed');
    await expect(clearAiInsightsHistory()).rejects.toThrow('clear failed');

    expect(listener).toHaveBeenNthCalledWith(1, {
      command: 'test_ai_insights_connection',
      message: 'connection check failed',
    });
    expect(listener).toHaveBeenNthCalledWith(2, {
      command: 'generate_ai_insights',
      message: 'generation failed',
    });
    expect(listener).toHaveBeenNthCalledWith(3, {
      command: 'list_ai_insights_history',
      message: 'history failed',
    });
    expect(listener).toHaveBeenNthCalledWith(4, {
      command: 'clear_ai_insights_history',
      message: 'clear failed',
    });

    unsubscribe();
  });

  it('rejects malformed payloads that violate insights contracts', async () => {
    invokeMock
      .mockResolvedValueOnce({
        normalizedEndpoint: 42,
        chatCompletionsUrl: 'https://example.openai.azure.com/openai/deployments/insights/chat/completions?api-version=2024-02-15-preview',
        model: 'gpt-4.1',
      })
      .mockResolvedValueOnce({
        summary: 'ok',
        risks: [],
        observations: [],
        recommendations: [],
        references: [{ kind: 'Node', name: 'aks-nodepool-01', namespace: null, extra: true }],
      })
      .mockResolvedValueOnce([
        {
          createdAt: '2026-01-01T00:00:00Z',
          scope: 'cluster',
          response: {
            summary: 'ok',
            risks: [],
            observations: [],
            recommendations: [],
            references: [],
          },
        },
      ]);

    await expect(testAiInsightsConnection()).rejects.toThrow(
      'Invalid test_ai_insights_connection payload from backend.',
    );
    await expect(generateAiInsights()).rejects.toThrow(
      'Invalid generate_ai_insights payload from backend.',
    );
    await expect(listAiInsightsHistory()).rejects.toThrow(
      'Invalid list_ai_insights_history payload from backend.',
    );
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
