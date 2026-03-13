export interface AksClusterInfo {
  subscriptionId: string;
  resourceGroup: string;
  clusterName: string;
  region: string;
}

export type AzureCloud = 'Commercial' | 'UsGovernment' | 'UsGovSecret' | 'UsGovTopSecret';

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

/** Build an Azure Portal deep-link for the given AKS cluster. */
export function getAzurePortalUrl(
  info: AksClusterInfo,
  cloud: AzureCloud | string = 'Commercial',
): string | null {
  if (!info.subscriptionId || !info.resourceGroup || !info.clusterName) return null;
  const portalBase =
    cloud === 'UsGovernment'
      ? 'https://portal.azure.us'
      : cloud === 'UsGovSecret'
        ? 'https://portal.azure.microsoft.scloud'
        : cloud === 'UsGovTopSecret'
          ? 'https://portal.azure.microsoft.eaglex.ic.gov'
          : 'https://portal.azure.com';

  return (
    `${portalBase}/#resource/subscriptions/${info.subscriptionId}` +
    `/resourceGroups/${info.resourceGroup}` +
    `/providers/Microsoft.ContainerService/managedClusters/${info.clusterName}/overview`
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
