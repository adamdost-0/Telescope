export type Cluster = { id: string; name: string };
export type Namespace = { name: string };

export type Kind = {
  /** Display name */
  kind: string;
  /** If false, treat as cluster-scoped (ignore namespace). */
  namespaced: boolean;
};

export type ResourceRow = {
  name: string;
  namespace?: string;
  status?: string;
  age?: string;
};

type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

async function getJson<T>(fetchLike: FetchLike, path: string): Promise<T> {
  const res = await fetchLike(path, { headers: { accept: 'application/json' } });
  if (!res.ok) throw new Error(`HTTP ${res.status} for ${path}`);
  return (await res.json()) as T;
}

// SSR-friendly helper used by +page.ts loaders.
export async function listClusters(fetchLike: FetchLike): Promise<Cluster[]> {
  const data = await getJson<{ clusters: Cluster[] }>(fetchLike, '/api/clusters');
  return data.clusters;
}

// Browser-friendly helper.
export async function getClusters(): Promise<Cluster[]> {
  return listClusters(globalThis.fetch);
}

export async function listNamespaces(fetchLike: FetchLike, clusterId: string): Promise<Namespace[]> {
  const data = await getJson<{ namespaces: Namespace[] }>(
    fetchLike,
    `/api/namespaces?cluster=${encodeURIComponent(clusterId)}`
  );
  return data.namespaces;
}

export async function getNamespaces(clusterId: string): Promise<Namespace[]> {
  return listNamespaces(globalThis.fetch, clusterId);
}

export async function listKinds(fetchLike: FetchLike): Promise<Kind[]> {
  const data = await getJson<{ kinds: Kind[] }>(fetchLike, '/api/kinds');
  return data.kinds;
}

export async function getKinds(): Promise<Kind[]> {
  return listKinds(globalThis.fetch);
}

export async function listResources(
  fetchLike: FetchLike,
  params: {
    clusterId: string;
    namespace?: string;
    kind: string;
  }
): Promise<ResourceRow[]> {
  const qs = new URLSearchParams();
  qs.set('cluster', params.clusterId);
  qs.set('kind', params.kind);
  if (params.namespace) qs.set('namespace', params.namespace);

  const data = await getJson<{ items: ResourceRow[] }>(fetchLike, `/api/resources?${qs.toString()}`);
  return data.items;
}

export async function getResources(params: {
  clusterId: string;
  namespace?: string;
  kind: string;
}): Promise<ResourceRow[]> {
  return listResources(globalThis.fetch, params);
}
