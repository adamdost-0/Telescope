export interface AksClusterInfo {
  subscriptionId: string;
  resourceGroup: string;
  clusterName: string;
  region: string;
}

/** Parse AKS cluster info from an AKS API server URL. */
export function parseAksUrl(serverUrl: string): AksClusterInfo | null {
  const match = serverUrl.match(
    /https?:\/\/([^.]+)(?:\.[^.]+)?\.hcp\.([^.]+)\.azmk8s\.(?:io|us)/,
  );
  if (!match) return null;
  return { subscriptionId: '', resourceGroup: '', clusterName: match[1], region: match[2] };
}

/** Returns true when the server URL matches an AKS managed cluster endpoint. */
export function isAksCluster(serverUrl: string): boolean {
  return (
    /\.hcp\.[^.]+\.azmk8s\.(io|us)/.test(serverUrl) ||
    serverUrl.includes('.cx.aks.containerservice.azure.us')
  );
}

/** Merge resolved AKS identity data into a parsed AKS cluster info object. */
export function mergeAksIdentity(
  parsed: AksClusterInfo | null,
  identity: { subscription_id: string; resource_group: string; cluster_name: string } | null,
): AksClusterInfo | null {
  if (!parsed && !identity) return null;
  return {
    subscriptionId: identity?.subscription_id ?? parsed?.subscriptionId ?? '',
    resourceGroup: identity?.resource_group ?? parsed?.resourceGroup ?? '',
    clusterName: identity?.cluster_name ?? parsed?.clusterName ?? '',
    region: parsed?.region ?? '',
  };
}
