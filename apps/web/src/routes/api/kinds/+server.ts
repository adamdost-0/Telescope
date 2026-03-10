import { ENGINE_HTTP_BASE } from '$env/static/private';
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async () => {
  if (!ENGINE_HTTP_BASE) {
    // Deterministic stub set for M1 explorer.
    return json(
      {
        kinds: [
          { kind: 'Pods', namespaced: true },
          { kind: 'Deployments', namespaced: true },
          { kind: 'Services', namespaced: true },
          { kind: 'ConfigMaps', namespaced: true },
          { kind: 'Nodes', namespaced: false },
          { kind: 'Namespaces', namespaced: false }
        ]
      },
      { status: 200 }
    );
  }

  const res = await fetch(`${ENGINE_HTTP_BASE}/api/kinds`, { headers: { accept: 'application/json' } });
  const body = await res.json().catch(() => ({ kinds: [] }));
  return json(body, { status: res.status });
};
