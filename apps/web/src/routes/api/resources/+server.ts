import { ENGINE_HTTP_BASE } from '$env/static/private';
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url }) => {
  const cluster = url.searchParams.get('cluster') ?? '';
  const kind = url.searchParams.get('kind') ?? '';
  const namespace = url.searchParams.get('namespace') ?? undefined;

  if (!cluster || !kind) return json({ items: [] }, { status: 400 });

  if (!ENGINE_HTTP_BASE) {
    // Deterministic stub rows.
    const items =
      kind === 'Nodes'
        ? [{ name: 'node-1', status: 'Ready', age: '1d' }]
        : kind === 'Deployments'
          ? [{ name: 'api', namespace: namespace ?? 'default', status: 'Available', age: '3h' }]
          : [{ name: 'api-7d9', namespace: namespace ?? 'default', status: 'Running', age: '12m' }];

    return json({ items }, { status: 200 });
  }

  const qs = new URLSearchParams();
  qs.set('cluster', cluster);
  qs.set('kind', kind);
  if (namespace) qs.set('namespace', namespace);

  const res = await fetch(`${ENGINE_HTTP_BASE}/api/resources?${qs.toString()}`, {
    headers: { accept: 'application/json' }
  });

  const body = await res.json().catch(() => ({ items: [] }));
  return json(body, { status: res.status });
};
