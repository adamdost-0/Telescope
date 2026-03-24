<script lang="ts">
  import { page } from '$app/state';
  import { isAks, isConnected } from '$lib/stores';

  let collapsed = $state(false);

  import Icon from '$lib/icons/Icon.svelte';
import type { ResourceIconName } from '$lib/icons';

  interface NavItem {
    label: string;
    href: string | null;
    icon: ResourceIconName;
    external?: boolean;
  }

  interface NavSection {
    title: string;
    items: NavItem[];
  }

  const baseSections: NavSection[] = [
    {
      title: 'Workloads',
      items: [
        { label: 'Pods', href: '/pods', icon: 'pods' },
        { label: 'Deployments', href: '/resources/deployments', icon: 'deployments' },
        { label: 'StatefulSets', href: '/resources/statefulsets', icon: 'statefulsets' },
        { label: 'DaemonSets', href: '/resources/daemonsets', icon: 'daemonsets' },
        { label: 'ReplicaSets', href: '/resources/replicasets', icon: 'replicasets' },
        { label: 'Jobs', href: '/resources/jobs', icon: 'jobs' },
        { label: 'CronJobs', href: '/resources/cronjobs', icon: 'cronjobs' },
        { label: 'HPAs', href: '/resources/hpas', icon: 'hpas' },
        { label: 'PDBs', href: '/resources/poddisruptionbudgets', icon: 'poddisruptionbudgets' },
      ]
    },
    {
      title: 'Network',
      items: [
        { label: 'Services', href: '/resources/services', icon: 'services' },
        { label: 'Ingresses', href: '/resources/ingresses', icon: 'ingresses' },
        { label: 'NetworkPolicies', href: '/resources/networkpolicies', icon: 'networkpolicies' },
        { label: 'EndpointSlices', href: '/resources/endpointslices', icon: 'endpointslices' },
      ]
    },
    {
      title: 'Config',
      items: [
        { label: 'ConfigMaps', href: '/resources/configmaps', icon: 'configmaps' },
        { label: 'Secrets', href: '/resources/secrets', icon: 'secrets' },
        { label: 'ResourceQuotas', href: '/resources/resourcequotas', icon: 'resourcequotas' },
        { label: 'LimitRanges', href: '/resources/limitranges', icon: 'limitranges' },
      ]
    },
    {
      title: 'Storage',
      items: [
        { label: 'PVCs', href: '/resources/pvcs', icon: 'pvcs' },
        { label: 'Persistent Volumes', href: '/resources/persistentvolumes', icon: 'persistentvolumes' },
        { label: 'Storage Classes', href: '/resources/storageclasses', icon: 'storageclasses' },
      ]
    },
    {
      title: 'Helm',
      items: [
        { label: 'Releases', href: '/helm', icon: 'helm' },
      ]
    },
    {
      title: 'Custom Resources',
      items: [
        { label: 'CRDs', href: '/crds', icon: 'crds' },
      ]
    },
    {
      title: 'System',
      items: [
        { label: 'ClusterRoles', href: '/resources/clusterroles', icon: 'clusterroles' },
        { label: 'ClusterRoleBindings', href: '/resources/clusterrolebindings', icon: 'clusterrolebindings' },
        { label: 'Settings', href: '/settings', icon: 'settings' },
      ]
    },
  ];

  const sections = $derived.by((): NavSection[] => {
    const clusterItems: NavItem[] = [
      { label: 'Overview', href: '/overview', icon: 'overview' },
      { label: 'Namespaces', href: '/namespaces', icon: 'namespaces' },
      { label: 'Create', href: '/create', icon: 'create' },
      { label: 'Nodes', href: '/nodes', icon: 'nodes' },
      { label: 'Priority Classes', href: '/resources/priorityclasses', icon: 'priorityclasses' },
      { label: 'Events', href: '/events', icon: 'events' },
    ];

    // Add Node Pools to Cluster section if on AKS and connected
    if ($isAks && $isConnected) {
      clusterItems.push({ label: 'Node Pools', href: '/azure/node-pools', icon: 'node-pools' });
    }

    const clusterSection: NavSection = {
      title: 'Cluster',
      items: clusterItems
    };

    return [
      clusterSection,
      ...baseSections
    ];
  });

  function isActive(item: NavItem): boolean {
    if (item.external || !item.href) return false;
    if (item.href === '/overview') return page.url.pathname === '/overview';
    return page.url.pathname.startsWith(item.href);
  }

  function isDisabled(item: NavItem): boolean {
    if (item.external) return !$isConnected || !item.href;
    if (!item.href) return true;
    return !$isConnected && item.href !== '/' && item.href !== '/overview' && item.href !== '/settings';
  }

  function getItemTitle(item: NavItem, iconOnly = false): string | undefined {
    if (isDisabled(item)) {
      return iconOnly ? `${item.label} — connect to a cluster first` : 'Connect to a cluster first';
    }

    return iconOnly ? item.label : undefined;
  }
</script>

<aside class="sidebar" class:collapsed>
  <button class="toggle" onclick={() => collapsed = !collapsed} aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
    {collapsed ? '→' : '←'}
  </button>

  {#if !collapsed}
    <nav aria-label="Resource navigation">
      {#each sections as section}
        <div class="section">
          <h3 class="section-title">{section.title}</h3>
          <ul>
            {#each section.items as item}
              {@const disabled = isDisabled(item)}
              <li>
                <a
                  href={item.href ?? undefined}
                  target={item.external ? '_blank' : undefined}
                  rel={item.external ? 'noopener noreferrer' : undefined}
                  class:active={isActive(item)}
                  class:disabled={disabled}
                  title={getItemTitle(item)}
                  aria-disabled={disabled}
                  tabindex={disabled ? -1 : undefined}
                >
                  <span class="icon-wrapper">
                    <Icon name={item.icon} size={18} />
                  </span>
                  <span class="label">{item.label}</span>
                </a>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </nav>
  {:else}
    <nav aria-label="Resource navigation (collapsed)">
      {#each sections as section}
        {#each section.items as item}
          {@const disabled = isDisabled(item)}
          <a
            href={item.href ?? undefined}
            class="icon-only"
            target={item.external ? '_blank' : undefined}
            rel={item.external ? 'noopener noreferrer' : undefined}
            class:active={isActive(item)}
            class:disabled={disabled}
            title={getItemTitle(item, true)}
            aria-disabled={disabled}
            tabindex={disabled ? -1 : undefined}
>
            <span class="icon-wrapper">
              <Icon name={item.icon} size={20} />
            </span>
          </a>
        {/each}
      {/each}
    </nav>
  {/if}
</aside>

<style>
  .sidebar {
    width: 220px;
    min-height: 100vh;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    transition: width 0.2s;
    overflow-y: auto;
    flex-shrink: 0;
  }
  .sidebar.collapsed {
    width: 48px;
  }
  .toggle {
    background: none;
    border: none;
    color: var(--text-secondary);
    padding: 0.75rem;
    cursor: pointer;
    text-align: right;
    font-size: 0.875rem;
  }
  .toggle:hover { color: var(--text-primary); }
  .section { margin-bottom: 0.5rem; }
  .section-title {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    padding: 0.25rem 0.75rem;
    margin: 0.5rem 0 0.25rem;
  }
  ul { list-style: none; padding: 0; margin: 0; }
  li a {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    color: var(--text-secondary);
    text-decoration: none;
    font-size: 0.85rem;
    border-radius: 4px;
    margin: 1px 4px;
    transition: background 0.15s;
  }
  li a:hover { background: var(--bg-tertiary); color: var(--text-primary); }
  li a.active { background: var(--bg-hover); color: var(--accent); font-weight: 500; }
  li a.disabled { opacity: 0.4; pointer-events: none; }
  .icon-wrapper {
    width: 1.25rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .icon-wrapper :global(svg) {
    display: block;
  }
  .icon-only {
    display: inline-flex;
    padding: 0.5rem;
    text-align: center;
    color: var(--text-secondary);
    text-decoration: none;
  }
  .icon-only:hover { background: var(--bg-tertiary); }
  .icon-only.active { background: var(--bg-hover); color: var(--accent); }
  .icon-only.disabled { opacity: 0.4; pointer-events: none; }
</style>
