import { ENGINE_HTTP_BASE } from '$env/static/private';
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

/**
 * Cluster API.
 *
 * - If PUBLIC_ENGINE_HTTP_BASE is set, proxy through to the stub engine (Playwright)
 *   or a real engine later.
 * - Otherwise return a deterministic stub payload for local dev.
 *
 * Optional query param: ?scenario=ok|empty|error
 */
export const GET: RequestHandler = async ({ url }) => {
  const scenario = url.searchParams.get('scenario') ?? 'ok';

  if (scenario === 'error') {
    return new Response('stub error', { status: 500 });
  }

  if (!ENGINE_HTTP_BASE) {
    const clusters =
      scenario === 'empty'
        ? []
        : [
            { id: 'local-dev', name: 'Local Dev Cluster' },
            { id: 'staging-aks', name: 'Staging AKS' }
          ];

    return json({ clusters }, { status: 200 });
  }

  const res = await globalThis.fetch(`${ENGINE_HTTP_BASE}/api/clusters`, {
    headers: { accept: 'application/json' }
  });

  const body = await res.json().catch(() => ({ clusters: [] }));
  return json(body, { status: res.status });
};
