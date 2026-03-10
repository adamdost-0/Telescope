import type { PageLoad } from './$types';
import { listClusters } from '$lib/engine';

export const load: PageLoad = async ({ fetch }) => {
  const clusters = await listClusters(fetch);
  return { clusters };
};
