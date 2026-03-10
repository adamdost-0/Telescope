import { PUBLIC_ENGINE_HTTP_BASE } from '$env/static/public';
import { json } from '@sveltejs/kit';

export const GET = async () => {
  if (!PUBLIC_ENGINE_HTTP_BASE) {
    return json({ clusters: [] }, { status: 200 });
  }

  const res = await globalThis.fetch(`${PUBLIC_ENGINE_HTTP_BASE}/api/clusters`, {
    headers: { accept: 'application/json' }
  });

  const body = await res.json().catch(() => ({ clusters: [] }));
  return json(body, { status: res.status });
};
