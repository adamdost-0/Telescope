export const AI_INSIGHTS_AUTH_MODES = ['azureLogin', 'apiKey'] as const;

export type AiInsightsAuthMode = (typeof AI_INSIGHTS_AUTH_MODES)[number];

export const AI_INSIGHTS_CLOUD_PROFILES = [
  'commercial',
  'usGovernment',
  'usGovernmentSecret',
  'usGovernmentTopSecret',
] as const;

export type AiInsightsCloudProfile = (typeof AI_INSIGHTS_CLOUD_PROFILES)[number];
export type AzureCloudProfile = AiInsightsCloudProfile;

export const AZURE_CLOUD_PROFILES = AI_INSIGHTS_CLOUD_PROFILES;

export const AI_INSIGHTS_PROVIDER_ERROR_CLASSES = [
  'configuration',
  'credential',
  'authorization',
  'endpoint',
  'timeout',
  'network',
  'unknown',
] as const;

export type AiInsightsProviderErrorClass =
  (typeof AI_INSIGHTS_PROVIDER_ERROR_CLASSES)[number];

export type AiInsightsRiskImpact = 'low' | 'medium' | 'high';

export interface AiInsightsRisk {
  title: string;
  detail: string;
  impact: AiInsightsRiskImpact;
}

export interface AiInsightsObservation {
  area: string;
  detail: string;
}

export interface AiInsightsRecommendation {
  action: string;
  rationale: string;
  confidence: number;
}

export interface AiInsightsReference {
  kind: string;
  name: string;
  namespace: string | null;
}

export interface AiInsightsResponse {
  summary: string;
  risks: AiInsightsRisk[];
  observations: AiInsightsObservation[];
  recommendations: AiInsightsRecommendation[];
  references: AiInsightsReference[];
}

export interface AiInsightsConnectionTestResult {
  normalizedEndpoint: string;
  chatCompletionsUrl: string;
  model: string;
}

export type AiInsightsScope =
  | { kind: 'cluster' }
  | { kind: 'namespace'; namespace: string };

export interface AiInsightsHistoryEntry {
  createdAt: string;
  scope: AiInsightsScope;
  response: AiInsightsResponse;
}

export interface AiInsightsSettings {
  endpoint: string;
  deploymentName: string;
  authMode: AiInsightsAuthMode;
  cloudProfile: AiInsightsCloudProfile;
  modelName: string | null;
}

export interface AiInsightsDevDiagnostics {
  promptVersion?: string | null;
  redactionPolicyVersion?: string | null;
  cloudProfile: AiInsightsCloudProfile;
  authMode: AiInsightsAuthMode;
  contextSize?: AiInsightsContextSize | null;
  schemaValidationFailure?: AiInsightsSchemaValidationFailure | null;
  providerErrorClassification?: AiInsightsProviderErrorClass | null;
}

export const AI_INSIGHTS_SETTINGS_KEYS = {
  endpoint: 'ai_insights_endpoint',
  deploymentName: 'ai_insights_deployment_name',
  authMode: 'ai_insights_auth_mode',
  cloudProfile: 'ai_insights_cloud_profile',
  modelName: 'ai_insights_model_name',
} as const;

export type AiInsightsSettingsKey =
  (typeof AI_INSIGHTS_SETTINGS_KEYS)[keyof typeof AI_INSIGHTS_SETTINGS_KEYS];

export const DEFAULT_AI_INSIGHTS_AUTH_MODE: AiInsightsAuthMode = 'azureLogin';
export const DEFAULT_AZURE_CLOUD_PROFILE: AiInsightsCloudProfile = 'commercial';

export interface AiInsightsContextSize {
  serializedBytes: number;
  resourceCount: number;
}

export interface AiInsightsSchemaValidationFailure {
  path?: string | null;
  message: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  return Object.keys(value).every((key) => keys.includes(key));
}

function hasExactKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  return Object.keys(value).length === keys.length && hasOnlyKeys(value, keys);
}

function isOptionalString(value: unknown): value is string | null | undefined {
  return value === undefined || value === null || typeof value === 'string';
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0;
}

export function isAiInsightsAuthMode(value: unknown): value is AiInsightsAuthMode {
  return typeof value === 'string'
    && (AI_INSIGHTS_AUTH_MODES as readonly string[]).includes(value);
}

export function isAiInsightsCloudProfile(value: unknown): value is AiInsightsCloudProfile {
  return typeof value === 'string'
    && (AI_INSIGHTS_CLOUD_PROFILES as readonly string[]).includes(value);
}

export function isAzureCloudProfile(value: unknown): value is AzureCloudProfile {
  return isAiInsightsCloudProfile(value);
}

export function isAiInsightsProviderErrorClass(value: unknown): value is AiInsightsProviderErrorClass {
  return typeof value === 'string'
    && (AI_INSIGHTS_PROVIDER_ERROR_CLASSES as readonly string[]).includes(value);
}

export function parseAiInsightsAuthMode(value: string | null | undefined): AiInsightsAuthMode {
  return isAiInsightsAuthMode(value) ? value : DEFAULT_AI_INSIGHTS_AUTH_MODE;
}

export function parseAiInsightsCloudProfile(
  value: string | null | undefined,
): AiInsightsCloudProfile {
  return isAiInsightsCloudProfile(value) ? value : DEFAULT_AZURE_CLOUD_PROFILE;
}

export function parseAzureCloudProfile(value: string | null | undefined): AzureCloudProfile {
  return parseAiInsightsCloudProfile(value);
}

export function formatAiInsightsScope(scope: AiInsightsScope): string {
  return scope.kind === 'cluster' ? 'cluster' : `namespace/${scope.namespace}`;
}

export function createDefaultAiInsightsSettings(
  cloudProfile: AiInsightsCloudProfile = DEFAULT_AZURE_CLOUD_PROFILE,
): AiInsightsSettings {
  return {
    endpoint: '',
    deploymentName: '',
    authMode: DEFAULT_AI_INSIGHTS_AUTH_MODE,
    cloudProfile,
    modelName: null,
  };
}

export function isAiInsightsResponse(value: unknown): value is AiInsightsResponse {
  if (!isRecord(value)
    || !hasExactKeys(value, ['summary', 'risks', 'observations', 'recommendations', 'references'])
    || typeof value.summary !== 'string') {
    return false;
  }

  const { risks, observations, recommendations, references } = value;

  return Array.isArray(risks)
    && risks.every((risk) => isRecord(risk)
      && hasExactKeys(risk, ['title', 'detail', 'impact'])
      && typeof risk.title === 'string'
      && typeof risk.detail === 'string'
      && (risk.impact === 'low' || risk.impact === 'medium' || risk.impact === 'high'))
    && Array.isArray(observations)
    && observations.every((observation) => isRecord(observation)
      && hasExactKeys(observation, ['area', 'detail'])
      && typeof observation.area === 'string'
      && typeof observation.detail === 'string')
    && Array.isArray(recommendations)
    && recommendations.every((recommendation) => isRecord(recommendation)
      && hasExactKeys(recommendation, ['action', 'rationale', 'confidence'])
      && typeof recommendation.action === 'string'
      && typeof recommendation.rationale === 'string'
      && typeof recommendation.confidence === 'number'
      && Number.isFinite(recommendation.confidence))
    && Array.isArray(references)
    && references.every((reference) => isRecord(reference)
      && hasExactKeys(reference, ['kind', 'name', 'namespace'])
      && typeof reference.kind === 'string'
      && typeof reference.name === 'string'
      && (typeof reference.namespace === 'string' || reference.namespace === null));
}

export function isAiInsightsScope(value: unknown): value is AiInsightsScope {
  if (!isRecord(value) || typeof value.kind !== 'string') {
    return false;
  }

  if (value.kind === 'cluster') {
    return hasExactKeys(value, ['kind']);
  }

  if (value.kind === 'namespace') {
    return hasExactKeys(value, ['kind', 'namespace'])
      && typeof value.namespace === 'string';
  }

  return false;
}

export function isAiInsightsConnectionTestResult(
  value: unknown,
): value is AiInsightsConnectionTestResult {
  return isRecord(value)
    && hasExactKeys(value, ['normalizedEndpoint', 'chatCompletionsUrl', 'model'])
    && typeof value.normalizedEndpoint === 'string'
    && typeof value.chatCompletionsUrl === 'string'
    && typeof value.model === 'string';
}

export function isAiInsightsHistoryEntry(value: unknown): value is AiInsightsHistoryEntry {
  return isRecord(value)
    && hasExactKeys(value, ['createdAt', 'scope', 'response'])
    && typeof value.createdAt === 'string'
    && isAiInsightsScope(value.scope)
    && isAiInsightsResponse(value.response);
}

function isAiInsightsContextSize(value: unknown): value is AiInsightsContextSize {
  return isRecord(value)
    && hasExactKeys(value, ['serializedBytes', 'resourceCount'])
    && isNonNegativeInteger(value.serializedBytes)
    && isNonNegativeInteger(value.resourceCount);
}

function isAiInsightsSchemaValidationFailure(
  value: unknown,
): value is AiInsightsSchemaValidationFailure {
  return isRecord(value)
    && hasOnlyKeys(value, ['path', 'message'])
    && typeof value.message === 'string'
    && isOptionalString(value.path);
}

export function isAiInsightsDevDiagnostics(value: unknown): value is AiInsightsDevDiagnostics {
  return isRecord(value)
    && hasOnlyKeys(value, [
      'promptVersion',
      'redactionPolicyVersion',
      'cloudProfile',
      'authMode',
      'contextSize',
      'schemaValidationFailure',
      'providerErrorClassification',
    ])
    && isOptionalString(value.promptVersion)
    && isOptionalString(value.redactionPolicyVersion)
    && isAiInsightsCloudProfile(value.cloudProfile)
    && isAiInsightsAuthMode(value.authMode)
    && (value.contextSize === undefined
      || value.contextSize === null
      || isAiInsightsContextSize(value.contextSize))
    && (value.schemaValidationFailure === undefined
      || value.schemaValidationFailure === null
      || isAiInsightsSchemaValidationFailure(value.schemaValidationFailure))
    && (value.providerErrorClassification === undefined
      || value.providerErrorClassification === null
      || isAiInsightsProviderErrorClass(value.providerErrorClassification));
}
