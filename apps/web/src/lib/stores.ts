import { writable, derived } from 'svelte/store';
import type { ConnectionState } from './tauri-commands';

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
