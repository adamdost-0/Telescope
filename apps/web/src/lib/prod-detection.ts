const PROD_PATTERNS = [/prod/i, /production/i, /\bprd\b/i, /\blive\b/i];

/** Returns true if the context name matches common production naming patterns. */
export function isProductionContext(contextName: string): boolean {
  return PROD_PATTERNS.some((p) => p.test(contextName));
}
