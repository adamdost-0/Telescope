import { PUBLIC_ENGINE_HTTP_BASE } from '$env/static/public';
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

/**
 * Cluster API.
 *
 * - If PUBLIC_ENGINE_HTTP_BASE is set, proxy through to the real engine.
 * - Otherwise return a deterministic stub payload for local dev + E2E.
 *
 * Optional query param: ?scenario=ok|empty|error
 */
export const GET: RequestHandler = async ({ url }) => {
  const scenario = url.searchParams.get('scenario') ?? 'ok';

  if (scenario === 'error') {
    return new Response('stub error', { status: 500 });
  }

  if (!PUBLIC_ENGINE_HTTP_BASE) {
    const clusters =
      scenario === 'empty'
        ? []
        : [
            { id: 'c1', name: 'Cluster One' },
            { id: 'c2', name: 'Cluster Two' }
          ];

    return json({ clusters }, { status: 200 });
  }

  const res = await globalThis.fetch(`${PUBLIC_ENGINE_HTTP_BASE}/api/clusters`, {
    headers: { accept: 'application/json' }
  });

  const body = await res.json().catch(() => ({ clusters: [] }));
  return json(body, { status: res.status });
};
