import { get } from 'svelte/store';
import { getPreference } from './api';
import { writable } from 'svelte/store';

const DEFAULT_PRODUCTION_PATTERNS = [/prod/i, /production/i, /\bprd\b/i, /\blive\b/i];
const DEFAULT_NAMESPACE = 'default';
const AUTO_REFRESH_PREFERENCE_KEY = 'auto_refresh_interval';
const DEFAULT_NAMESPACE_PREFERENCE_KEY = 'default_namespace';
const PRODUCTION_PATTERNS_PREFERENCE_KEY = 'production_patterns';

export const productionPatterns = writable<RegExp[]>(DEFAULT_PRODUCTION_PATTERNS);

let productionPatternsRequested = false;

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function compilePattern(pattern: string): RegExp {
  try {
    return new RegExp(pattern, 'i');
  } catch {
    return new RegExp(escapeRegExp(pattern), 'i');
  }
}

export function parseProductionPatterns(value: string | null | undefined): RegExp[] {
  const patterns = (value ?? '')
    .split(/[\n,]+/)
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map(compilePattern);

  return patterns.length > 0 ? patterns : DEFAULT_PRODUCTION_PATTERNS;
}

export function updateProductionPatterns(value: string | null | undefined): void {
  productionPatterns.set(parseProductionPatterns(value));
  productionPatternsRequested = true;
}

export function ensureProductionPatternsLoaded(): void {
  if (productionPatternsRequested) {
    return;
  }

  productionPatternsRequested = true;
  void getPreference(PRODUCTION_PATTERNS_PREFERENCE_KEY)
    .then((savedPatterns) => {
      productionPatterns.set(parseProductionPatterns(savedPatterns));
    })
    .catch(() => {
      productionPatterns.set(DEFAULT_PRODUCTION_PATTERNS);
    });
}

export function isProductionContext(contextName: string, patterns: readonly RegExp[] = get(productionPatterns)): boolean {
  return patterns.some((pattern) => pattern.test(contextName));
}

export async function getPreferredNamespace(availableNamespaces: string[]): Promise<string> {
  const preferredNamespace = (await getPreference(DEFAULT_NAMESPACE_PREFERENCE_KEY))?.trim();

  if (preferredNamespace && availableNamespaces.includes(preferredNamespace)) {
    return preferredNamespace;
  }

  if (availableNamespaces.includes(DEFAULT_NAMESPACE)) {
    return DEFAULT_NAMESPACE;
  }

  return availableNamespaces[0] ?? DEFAULT_NAMESPACE;
}

export async function getAutoRefreshIntervalMs(fallbackMs: number): Promise<number> {
  const savedInterval = await getPreference(AUTO_REFRESH_PREFERENCE_KEY);
  const intervalSeconds = Number.parseInt(savedInterval ?? '', 10);

  if (Number.isFinite(intervalSeconds) && intervalSeconds > 0) {
    return intervalSeconds * 1000;
  }

  return fallbackMs;
}
