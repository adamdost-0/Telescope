import { describe, expect, it } from 'vitest';

import {
  createDefaultAiInsightsSettings,
  DEFAULT_AI_INSIGHTS_AUTH_MODE,
  DEFAULT_AZURE_CLOUD_PROFILE,
  AI_INSIGHTS_SETTINGS_KEYS,
  formatAiInsightsScope,
  isAiInsightsConnectionTestResult,
  isAiInsightsCloudProfile,
  isAiInsightsDevDiagnostics,
  isAiInsightsHistoryEntry,
  isAiInsightsResponse,
  isAiInsightsScope,
  parseAiInsightsAuthMode,
  parseAiInsightsCloudProfile,
  parseAzureCloudProfile,
} from './insights';

describe('AI Insights contract helpers', () => {
  it('falls back to the explicit default auth mode for unknown values', () => {
    expect(parseAiInsightsAuthMode('something-else')).toBe(DEFAULT_AI_INSIGHTS_AUTH_MODE);
    expect(parseAiInsightsAuthMode('apiKey')).toBe('apiKey');
  });

  it('parses cloud profiles using Rust enum serialization', () => {
    expect(isAiInsightsCloudProfile('usGovernment')).toBe(true);
    expect(parseAiInsightsCloudProfile('usGovernment')).toBe('usGovernment');
    expect(parseAzureCloudProfile('auto')).toBe(DEFAULT_AZURE_CLOUD_PROFILE);
  });

  it('creates the default non-secret settings model with the selected cloud profile', () => {
    expect(createDefaultAiInsightsSettings('usGovernmentTopSecret')).toEqual({
      endpoint: '',
      deploymentName: '',
      authMode: DEFAULT_AI_INSIGHTS_AUTH_MODE,
      cloudProfile: 'usGovernmentTopSecret',
      modelName: null,
    });
  });

  it('uses the dedicated AI settings keys including model name', () => {
    expect(AI_INSIGHTS_SETTINGS_KEYS).toEqual({
      endpoint: 'ai_insights_endpoint',
      deploymentName: 'ai_insights_deployment_name',
      authMode: 'ai_insights_auth_mode',
      cloudProfile: 'ai_insights_cloud_profile',
      modelName: 'ai_insights_model_name',
    });
  });

  it('accepts only PRD-shaped AI insights responses', () => {
    const validResponse = {
      summary: 'Cluster health is stable with one elevated risk.',
      risks: [{ title: 'Node pressure', detail: 'One node is under memory pressure.', impact: 'medium' }],
      observations: [{ area: 'Workloads', detail: 'All critical deployments are available.' }],
      recommendations: [{ action: 'Investigate the affected node', rationale: 'Memory pressure can cascade into evictions.', confidence: 0.82 }],
      references: [{ kind: 'Node', name: 'aks-nodepool-1', namespace: null }],
    };

    expect(isAiInsightsResponse(validResponse)).toBe(true);
    expect(isAiInsightsResponse({ ...validResponse, chat: [] })).toBe(false);
    expect(isAiInsightsResponse({
      ...validResponse,
      risks: [{ ...validResponse.risks[0], extra: true }],
    })).toBe(false);
    expect(isAiInsightsResponse({ ...validResponse, references: [{ kind: 'Node', name: 'aks-nodepool-1' }] })).toBe(false);
  });

  it('parses scope contracts from backend-tagged enum payloads', () => {
    expect(isAiInsightsScope({ kind: 'cluster' })).toBe(true);
    expect(isAiInsightsScope({ kind: 'namespace', namespace: 'payments' })).toBe(true);
    expect(isAiInsightsScope({ kind: 'namespace' })).toBe(false);
    expect(isAiInsightsScope('cluster')).toBe(false);

    expect(formatAiInsightsScope({ kind: 'cluster' })).toBe('cluster');
    expect(formatAiInsightsScope({ kind: 'namespace', namespace: 'payments' })).toBe('namespace/payments');
  });

  it('validates connection-test and history-entry payload contracts', () => {
    const validResponse = {
      summary: 'Cluster health is stable with one elevated risk.',
      risks: [{ title: 'Node pressure', detail: 'One node is under memory pressure.', impact: 'medium' }],
      observations: [{ area: 'Workloads', detail: 'All critical deployments are available.' }],
      recommendations: [{ action: 'Investigate the affected node', rationale: 'Memory pressure can cascade into evictions.', confidence: 0.82 }],
      references: [{ kind: 'Node', name: 'aks-nodepool-1', namespace: null }],
    };

    expect(isAiInsightsConnectionTestResult({
      normalizedEndpoint: 'https://example.openai.azure.com',
      chatCompletionsUrl: 'https://example.openai.azure.com/openai/deployments/insights/chat/completions?api-version=2024-02-15-preview',
      model: 'gpt-4.1',
    })).toBe(true);
    expect(isAiInsightsConnectionTestResult({
      normalizedEndpoint: 'https://example.openai.azure.com',
      chatCompletionsUrl: 'https://example.openai.azure.com/openai/deployments/insights/chat/completions?api-version=2024-02-15-preview',
    })).toBe(false);

    expect(isAiInsightsHistoryEntry({
      createdAt: '2026-01-01T00:00:00Z',
      scope: { kind: 'cluster' },
      response: validResponse,
    })).toBe(true);
    expect(isAiInsightsHistoryEntry({
      createdAt: '2026-01-01T00:00:00Z',
      scope: 'cluster',
      response: validResponse,
    })).toBe(false);
  });

  it('validates the dev diagnostics metadata contract with optional structured fields', () => {
    expect(isAiInsightsDevDiagnostics({
      promptVersion: 'v1',
      redactionPolicyVersion: 'v1',
      cloudProfile: 'commercial',
      authMode: 'azureLogin',
      contextSize: { serializedBytes: 2048, resourceCount: 12 },
      schemaValidationFailure: {
        path: 'recommendations[0].confidence',
        message: 'confidence must be between 0 and 1',
      },
      providerErrorClassification: 'authorization',
    })).toBe(true);

    expect(isAiInsightsDevDiagnostics({
      cloudProfile: 'commercial',
      authMode: 'azureLogin',
    })).toBe(true);

    expect(isAiInsightsDevDiagnostics({
      promptVersion: 'v1',
      redactionPolicyVersion: 'v1',
      cloudProfile: 'commercial',
      authMode: 'azureLogin',
      contextSize: { serializedBytes: 2048, resourceCount: 12, extra: 1 },
      providerErrorClassification: null,
    })).toBe(false);

    expect(isAiInsightsDevDiagnostics({
      promptVersion: 'v1',
      redactionPolicyVersion: 'v1',
      cloudProfile: 'commercial',
      authMode: 'azureLogin',
      providerErrorClassification: 'authorization',
      extra: true,
    })).toBe(false);
  });
});
