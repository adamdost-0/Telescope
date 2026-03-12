export interface AksClusterInfo {
  subscriptionId: string;
  resourceGroup: string;
  clusterName: string;
  region: string;
}

/** Parse AKS cluster info from an AKS API server URL. */
export function parseAksUrl(serverUrl: string): AksClusterInfo | null {
  // Pattern: https://{dns-prefix}.hcp.{region}.azmk8s.io:443
  const match = serverUrl.match(/https?:\/\/([^.]+)\.hcp\.([^.]+)\.azmk8s\.io/);
  if (!match) return null;
  return { subscriptionId: '', resourceGroup: '', clusterName: match[1], region: match[2] };
}

/** Returns true when the server URL matches an AKS managed cluster endpoint. */
export function isAksCluster(serverUrl: string): boolean {
  return /\.hcp\.[^.]+\.azmk8s\.io/.test(serverUrl);
}

/** Build an Azure Portal deep-link for the given AKS cluster. */
export function getAzurePortalUrl(info: AksClusterInfo): string | null {
  if (!info.subscriptionId || !info.resourceGroup) return null;
  return (
    `https://portal.azure.com/#resource/subscriptions/${info.subscriptionId}` +
    `/resourceGroups/${info.resourceGroup}` +
    `/providers/Microsoft.ContainerService/managedClusters/${info.clusterName}/overview`
  );
}
