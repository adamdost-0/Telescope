import { ENGINE_HTTP_BASE } from '$env/static/private';
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url }) => {
  const cluster = url.searchParams.get('cluster') ?? '';
  if (!cluster) return json({ namespaces: [] }, { status: 400 });

  if (!ENGINE_HTTP_BASE) {
    // Deterministic stub.
    return json({ namespaces: [{ name: 'default' }, { name: 'kube-system' }] }, { status: 200 });
  }

  const res = await fetch(`${ENGINE_HTTP_BASE}/api/clusters/${encodeURIComponent(cluster)}/namespaces`, {
    headers: { accept: 'application/json' }
  });

  const body = await res.json().catch(() => ({ namespaces: [] }));
  return json(body, { status: res.status });
};
