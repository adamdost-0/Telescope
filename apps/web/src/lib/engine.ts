export type Cluster = {
  id: string;
  name: string;
  context?: string;
  server?: string;
};

export async function listClusters(fetchFn: typeof fetch): Promise<Cluster[]> {
  const res = await fetchFn('/api/clusters', {
    headers: { accept: 'application/json' }
  });
  if (!res.ok) {
    throw new Error(`GET /api/clusters failed: ${res.status}`);
  }

  const body = (await res.json()) as { clusters?: Cluster[] };
  return body.clusters ?? [];
}
