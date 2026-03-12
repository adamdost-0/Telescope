/**
 * Map common Kubernetes / cluster error strings to user-friendly suggestions.
 */
export function getSuggestion(error: string): string {
  if (!error) return '';
  const lower = error.toLowerCase();

  if (lower.includes('connection refused') || lower.includes('econnrefused'))
    return 'Check if the cluster is running and accessible.';
  if (lower.includes('unauthorized') || lower.includes('401'))
    return 'Your credentials may have expired. Try reconnecting.';
  if (lower.includes('forbidden') || lower.includes('403'))
    return 'You may not have permission. Check your RBAC role bindings.';
  if (lower.includes('not found') || lower.includes('404'))
    return 'The resource may have been deleted. Try refreshing.';
  if (lower.includes('timeout'))
    return 'The cluster may be overloaded. Try again in a moment.';
  if (lower.includes('kubelogin') || lower.includes('exec'))
    return 'Ensure kubelogin is installed and in your PATH.';
  if (lower.includes('network') || lower.includes('dns'))
    return 'Check your network connection and DNS settings.';

  return 'Check your cluster connection and try again.';
}
