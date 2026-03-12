<script lang="ts">
  import { onMount } from 'svelte';
  import { getPods } from '$lib/api';
  import type { ResourceEntry } from '$lib/tauri-commands';

  interface AddonPattern {
    name: string;
    patterns: string[];
    namespace: string;
    icon: string;
  }

  const ADDON_PATTERNS: AddonPattern[] = [
    { name: 'Container Insights', patterns: ['ama-logs', 'omsagent'], namespace: 'kube-system', icon: '📊' },
    { name: 'Azure Policy', patterns: ['azure-policy'], namespace: 'kube-system', icon: '🛡️' },
    { name: 'Key Vault CSI', patterns: ['secrets-store-csi'], namespace: 'kube-system', icon: '🔑' },
    { name: 'KEDA', patterns: ['keda-operator'], namespace: 'kube-system', icon: '⚡' },
    { name: 'Flux GitOps', patterns: ['flux'], namespace: 'flux-system', icon: '🔄' },
    { name: 'Ingress NGINX', patterns: ['ingress-nginx'], namespace: 'ingress-nginx', icon: '🚪' },
  ];

  type AddonStatus = 'Healthy' | 'Degraded' | 'Not Installed';

  interface AddonResult {
    name: string;
    icon: string;
    status: AddonStatus;
    total: number;
    running: number;
  }

  let addons: AddonResult[] = $state([]);
  let loading = $state(true);

  function getPodPhase(pod: ResourceEntry): string {
    try {
      const obj = JSON.parse(pod.content);
      return obj?.status?.phase ?? 'Unknown';
    } catch {
      return 'Unknown';
    }
  }

  function matchesAddon(podName: string, patterns: string[]): boolean {
    return patterns.some((p) => podName.includes(p));
  }

  async function detectAddons() {
    // Collect unique namespaces to query
    const namespacesToQuery = [...new Set(ADDON_PATTERNS.map((a) => a.namespace))];
    const podsByNamespace = new Map<string, ResourceEntry[]>();

    await Promise.all(
      namespacesToQuery.map(async (ns) => {
        const pods = await getPods(ns);
        podsByNamespace.set(ns, pods);
      }),
    );

    addons = ADDON_PATTERNS.map((addon) => {
      const nsPods = podsByNamespace.get(addon.namespace) ?? [];
      const matched = nsPods.filter((p) => matchesAddon(p.name, addon.patterns));

      if (matched.length === 0) {
        return { name: addon.name, icon: addon.icon, status: 'Not Installed' as const, total: 0, running: 0 };
      }

      const running = matched.filter((p) => getPodPhase(p) === 'Running').length;
      const status: AddonStatus = running === matched.length ? 'Healthy' : 'Degraded';
      return { name: addon.name, icon: addon.icon, status, total: matched.length, running };
    });

    loading = false;
  }

  onMount(() => {
    detectAddons();
  });

  const statusColor: Record<AddonStatus, string> = {
    Healthy: '#66bb6a',
    Degraded: '#ffa726',
    'Not Installed': '#484f58',
  };

  const statusBg: Record<AddonStatus, string> = {
    Healthy: 'rgba(102, 187, 106, 0.15)',
    Degraded: 'rgba(255, 167, 38, 0.15)',
    'Not Installed': 'rgba(72, 79, 88, 0.15)',
  };

  let installedCount = $derived(addons.filter((a) => a.status !== 'Not Installed').length);
</script>

<section class="aks-addons" aria-label="AKS Add-ons" data-testid="aks-addons">
  <h2>AKS Add-ons</h2>
  {#if loading}
    <p role="status">Detecting add-ons…</p>
  {:else}
    <p class="addon-summary">{installedCount} of {addons.length} add-ons detected</p>
    <div class="addon-grid">
      {#each addons as addon}
        <div
          class="addon-card"
          data-testid="addon-card-{addon.name.toLowerCase().replace(/\s+/g, '-')}"
          style="border-color: {statusColor[addon.status]};"
        >
          <span class="addon-icon">{addon.icon}</span>
          <div class="addon-body">
            <span class="addon-name">{addon.name}</span>
            <span
              class="addon-status"
              style="color: {statusColor[addon.status]}; background: {statusBg[addon.status]};"
            >
              {#if addon.status === 'Healthy'}✓ Healthy{:else if addon.status === 'Degraded'}⚠ Degraded ({addon.running}/{addon.total}){:else}— Not Installed{/if}
            </span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .aks-addons h2 {
    font-size: 1.1rem;
    color: #8b949e;
    margin: 1.5rem 0 0.75rem;
  }
  .addon-summary {
    font-size: 0.8rem;
    color: #484f58;
    margin: 0 0 0.75rem;
  }
  .addon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.75rem;
  }
  .addon-card {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-left: 3px solid;
    border-radius: 8px;
  }
  .addon-icon {
    font-size: 1.5rem;
  }
  .addon-body {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .addon-name {
    font-size: 0.85rem;
    color: #e0e0e0;
    font-weight: 500;
  }
  .addon-status {
    font-size: 0.7rem;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    display: inline-block;
    width: fit-content;
  }
</style>
