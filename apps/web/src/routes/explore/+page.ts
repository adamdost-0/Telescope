import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { listClusters, listKinds, listNamespaces, listResources } from '$lib/engine';

export const load: PageLoad = async ({ url, fetch }) => {
  const clusterId = url.searchParams.get('cluster');
  if (!clusterId) throw redirect(302, '/clusters');

  const namespace = url.searchParams.get('namespace') ?? 'default';
  const kind = url.searchParams.get('kind') ?? 'Pods';

  const [clusters, kinds, namespaces] = await Promise.all([
    listClusters(fetch),
    listKinds(fetch),
    listNamespaces(fetch, clusterId)
  ]);

  const kindMeta = kinds.find((k) => k.kind === kind) ?? { kind, namespaced: true };
  const effectiveNamespace = kindMeta.namespaced ? namespace : undefined;

  const items = await listResources(fetch, { clusterId, namespace: effectiveNamespace, kind });

  return { clusterId, namespace, kind, clusters, kinds, namespaces, items };
};
