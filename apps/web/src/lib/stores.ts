import { writable, derived } from 'svelte/store';
import type { ConnectionState } from './tauri-commands';
import { isAksCluster } from './azure-utils';
import {
  ensureProductionPatternsLoaded,
  isProductionContext,
  productionPatterns,
} from './prod-detection';

/** Currently selected kubeconfig context name. */
export const selectedContext = writable<string | null>(null);

/** Currently selected namespace. */
export const selectedNamespace = writable<string>('default');

/** Available namespaces for the connected cluster. */
export const namespaces = writable<string[]>(['default']);

/** Current connection state. */
export const connectionState = writable<ConnectionState>({ state: 'Disconnected' });

/** Whether we're connected to a cluster. */
export const isConnected = derived(connectionState, ($state) => $state.state === 'Ready');

/** Whether the selected context looks like a production cluster. */
export const isProduction = derived([selectedContext, productionPatterns], ([$ctx, $patterns]) => {
  ensureProductionPatternsLoaded();
  if (!$ctx) return false;
  return isProductionContext($ctx, $patterns);
});

/** API server URL of the currently connected cluster. */
export const clusterServerUrl = writable<string | null>(null);

/** Whether the connected cluster is an AKS managed cluster. */
export const isAks = derived(clusterServerUrl, ($url) => ($url ? isAksCluster($url) : false));

/** Reset client-side connection state after disconnecting from a cluster. */
export function resetConnectionStores(): void {
  selectedContext.set(null);
  selectedNamespace.set('default');
  namespaces.set(['default']);
  connectionState.set({ state: 'Disconnected' });
  clusterServerUrl.set(null);
}
