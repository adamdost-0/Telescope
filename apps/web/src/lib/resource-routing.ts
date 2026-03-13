import type { ResourceEntry } from '$lib/tauri-commands';

export const CLUSTER_SCOPED_NAMESPACE = '_cluster';

type ResourceRoute = {
  label: string;
  gvk: string;
  slug?: string;
  listHref: string | null;
  detailHref: (namespace: string | null | undefined, name: string) => string;
};

function encodeSegment(value: string): string {
  return encodeURIComponent(value);
}

function namespaceSegment(namespace: string | null | undefined): string {
  return namespace && namespace.length > 0
    ? encodeSegment(namespace)
    : CLUSTER_SCOPED_NAMESPACE;
}

const RESOURCE_ROUTES: ResourceRoute[] = [
  {
    label: 'Pod',
    gvk: 'v1/Pod',
    listHref: '/pods',
    detailHref: (namespace, name) => `/pods/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'Node',
    gvk: 'v1/Node',
    listHref: '/nodes',
    detailHref: (_namespace, name) => `/nodes/${encodeSegment(name)}`,
  },
  {
    label: 'Event',
    gvk: 'v1/Event',
    listHref: '/events',
    detailHref: () => '/events',
  },
  {
    label: 'Deployment',
    gvk: 'apps/v1/Deployment',
    slug: 'deployments',
    listHref: '/resources/deployments',
    detailHref: (namespace, name) => `/resources/deployments/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'StatefulSet',
    gvk: 'apps/v1/StatefulSet',
    slug: 'statefulsets',
    listHref: '/resources/statefulsets',
    detailHref: (namespace, name) => `/resources/statefulsets/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'DaemonSet',
    gvk: 'apps/v1/DaemonSet',
    slug: 'daemonsets',
    listHref: '/resources/daemonsets',
    detailHref: (namespace, name) => `/resources/daemonsets/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'ReplicaSet',
    gvk: 'apps/v1/ReplicaSet',
    listHref: null,
    detailHref: (namespace, name) => {
      const params = new URLSearchParams({
        gvk: 'apps/v1/ReplicaSet',
        label: 'ReplicaSet',
      });
      return `/resources/replicaset/${namespaceSegment(namespace)}/${encodeSegment(name)}?${params.toString()}`;
    },
  },
  {
    label: 'Job',
    gvk: 'batch/v1/Job',
    slug: 'jobs',
    listHref: '/resources/jobs',
    detailHref: (namespace, name) => `/resources/jobs/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'CronJob',
    gvk: 'batch/v1/CronJob',
    slug: 'cronjobs',
    listHref: '/resources/cronjobs',
    detailHref: (namespace, name) => `/resources/cronjobs/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'Service',
    gvk: 'v1/Service',
    slug: 'services',
    listHref: '/resources/services',
    detailHref: (namespace, name) => `/resources/services/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'Ingress',
    gvk: 'networking.k8s.io/v1/Ingress',
    slug: 'ingresses',
    listHref: '/resources/ingresses',
    detailHref: (namespace, name) => `/resources/ingresses/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'NetworkPolicy',
    gvk: 'networking.k8s.io/v1/NetworkPolicy',
    slug: 'networkpolicies',
    listHref: '/resources/networkpolicies',
    detailHref: (namespace, name) => `/resources/networkpolicies/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'EndpointSlice',
    gvk: 'discovery.k8s.io/v1/EndpointSlice',
    slug: 'endpointslices',
    listHref: '/resources/endpointslices',
    detailHref: (namespace, name) => `/resources/endpointslices/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'ConfigMap',
    gvk: 'v1/ConfigMap',
    slug: 'configmaps',
    listHref: '/resources/configmaps',
    detailHref: (namespace, name) => `/resources/configmaps/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'PersistentVolumeClaim',
    gvk: 'v1/PersistentVolumeClaim',
    slug: 'pvcs',
    listHref: '/resources/pvcs',
    detailHref: (namespace, name) => `/resources/pvcs/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'Secret',
    gvk: 'v1/Secret',
    slug: 'secrets',
    listHref: '/resources/secrets',
    detailHref: (namespace, name) => `/resources/secrets/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'ResourceQuota',
    gvk: 'v1/ResourceQuota',
    slug: 'resourcequotas',
    listHref: '/resources/resourcequotas',
    detailHref: (namespace, name) => `/resources/resourcequotas/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'LimitRange',
    gvk: 'v1/LimitRange',
    slug: 'limitranges',
    listHref: '/resources/limitranges',
    detailHref: (namespace, name) => `/resources/limitranges/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'Role',
    gvk: 'rbac.authorization.k8s.io/v1/Role',
    slug: 'roles',
    listHref: '/resources/roles',
    detailHref: (namespace, name) => `/resources/roles/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'ClusterRole',
    gvk: 'rbac.authorization.k8s.io/v1/ClusterRole',
    slug: 'clusterroles',
    listHref: '/resources/roles',
    detailHref: (_namespace, name) => `/resources/clusterroles/${CLUSTER_SCOPED_NAMESPACE}/${encodeSegment(name)}`,
  },
  {
    label: 'RoleBinding',
    gvk: 'rbac.authorization.k8s.io/v1/RoleBinding',
    slug: 'rolebindings',
    listHref: '/resources/rolebindings',
    detailHref: (namespace, name) => `/resources/rolebindings/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'ClusterRoleBinding',
    gvk: 'rbac.authorization.k8s.io/v1/ClusterRoleBinding',
    slug: 'clusterrolebindings',
    listHref: '/resources/rolebindings',
    detailHref: (_namespace, name) => `/resources/clusterrolebindings/${CLUSTER_SCOPED_NAMESPACE}/${encodeSegment(name)}`,
  },
  {
    label: 'ServiceAccount',
    gvk: 'v1/ServiceAccount',
    slug: 'serviceaccounts',
    listHref: '/resources/serviceaccounts',
    detailHref: (namespace, name) => `/resources/serviceaccounts/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'ValidatingWebhookConfiguration',
    gvk: 'admissionregistration.k8s.io/v1/ValidatingWebhookConfiguration',
    slug: 'validatingwebhookconfigurations',
    listHref: '/resources/webhooks',
    detailHref: (_namespace, name) => `/resources/validatingwebhookconfigurations/${CLUSTER_SCOPED_NAMESPACE}/${encodeSegment(name)}`,
  },
  {
    label: 'MutatingWebhookConfiguration',
    gvk: 'admissionregistration.k8s.io/v1/MutatingWebhookConfiguration',
    slug: 'mutatingwebhookconfigurations',
    listHref: '/resources/webhooks',
    detailHref: (_namespace, name) => `/resources/mutatingwebhookconfigurations/${CLUSTER_SCOPED_NAMESPACE}/${encodeSegment(name)}`,
  },
  {
    label: 'HorizontalPodAutoscaler',
    gvk: 'autoscaling/v2/HorizontalPodAutoscaler',
    slug: 'hpas',
    listHref: '/resources/hpas',
    detailHref: (namespace, name) => `/resources/hpas/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'PodDisruptionBudget',
    gvk: 'policy/v1/PodDisruptionBudget',
    slug: 'poddisruptionbudgets',
    listHref: '/resources/poddisruptionbudgets',
    detailHref: (namespace, name) => `/resources/poddisruptionbudgets/${namespaceSegment(namespace)}/${encodeSegment(name)}`,
  },
  {
    label: 'PriorityClass',
    gvk: 'scheduling.k8s.io/v1/PriorityClass',
    slug: 'priorityclasses',
    listHref: '/resources/priorityclasses',
    detailHref: (_namespace, name) => `/resources/priorityclasses/${CLUSTER_SCOPED_NAMESPACE}/${encodeSegment(name)}`,
  },
];

function normalizeKind(kind: string): string {
  return kind.replace(/[\s-]+/g, '').toLowerCase();
}

function routeForKind(kind: string): ResourceRoute | undefined {
  return RESOURCE_ROUTES.find((route) => normalizeKind(kindFromGvk(route.gvk)) === normalizeKind(kind));
}

export function parseGvk(gvk: string): { group: string | null; version: string | null; kind: string } {
  const parts = gvk.split('/').filter(Boolean);

  if (parts.length >= 3) {
    return { group: parts[0], version: parts[1], kind: parts[parts.length - 1] };
  }

  if (parts.length === 2) {
    return { group: null, version: parts[0], kind: parts[1] };
  }

  return { group: null, version: null, kind: parts[0] ?? gvk };
}

export function kindFromGvk(gvk: string): string {
  return parseGvk(gvk).kind;
}

export function gvkForKind(kind: string): string | null {
  return routeForKind(kind)?.gvk ?? null;
}

export function labelForGvk(gvk: string): string {
  return routeForKind(kindFromGvk(gvk))?.label ?? kindFromGvk(gvk);
}

export function decodeNamespaceParam(namespaceParam: string): string | null {
  return namespaceParam === CLUSTER_SCOPED_NAMESPACE ? null : decodeURIComponent(namespaceParam);
}

export function resourceCollectionHref(gvk: string): string | null {
  const known = routeForKind(kindFromGvk(gvk));
  if (known) {
    return known.listHref;
  }

  const { group, version, kind } = parseGvk(gvk);
  if (!group || !version) {
    return null;
  }

  const params = new URLSearchParams({ version });
  return `/crds/${encodeSegment(group)}/${encodeSegment(kind)}?${params.toString()}`;
}

function crdHrefFromContent(content: string): string | null {
  try {
    const resource = JSON.parse(content);
    const group = resource.spec?.group;
    const kind = resource.spec?.names?.kind;
    const plural = resource.spec?.names?.plural;
    const scope = resource.spec?.scope;
    const version = resource.spec?.versions?.find((candidate: { storage?: boolean; name?: string }) => candidate.storage)?.name
      ?? resource.spec?.versions?.[0]?.name
      ?? resource.spec?.version;

    if (!group || !kind || !version) {
      return null;
    }

    const params = new URLSearchParams({ version });
    if (plural) params.set('plural', plural);
    if (scope) params.set('scope', scope);

    return `/crds/${encodeSegment(group)}/${encodeSegment(kind)}?${params.toString()}`;
  } catch {
    return null;
  }
}

export function resourceDetailHref(options: {
  gvk: string;
  namespace?: string | null;
  name: string;
  label?: string;
}): string {
  const { gvk, namespace, name, label } = options;
  const kind = kindFromGvk(gvk);
  const normalizedKind = normalizeKind(kind);

  if (normalizedKind === 'customresourcedefinition') {
    return '/crds';
  }

  const known = routeForKind(kind);
  if (known) {
    return known.detailHref(namespace, name);
  }

  const params = new URLSearchParams({ gvk, label: label ?? kind });
  return `/resources/${encodeSegment(normalizedKind)}/${namespaceSegment(namespace)}/${encodeSegment(name)}?${params.toString()}`;
}

export function routeForSearchEntry(entry: ResourceEntry): string {
  if (normalizeKind(kindFromGvk(entry.gvk)) === 'customresourcedefinition') {
    return crdHrefFromContent(entry.content) ?? '/crds';
  }

  return resourceDetailHref({
    gvk: entry.gvk,
    namespace: entry.namespace,
    name: entry.name,
  });
}
