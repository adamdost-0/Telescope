<script lang="ts">
  import { page } from '$app/state';
  import { isConnected } from '$lib/stores';

  let collapsed = $state(false);

  interface NavItem {
    label: string;
    href: string;
    icon: string;
  }

  interface NavSection {
    title: string;
    items: NavItem[];
  }

  const sections: NavSection[] = [
    {
      title: 'Cluster',
      items: [
        { label: 'Overview', href: '/', icon: '🏠' },
        { label: 'Events', href: '/events', icon: '⚡' },
      ]
    },
    {
      title: 'Workloads',
      items: [
        { label: 'Pods', href: '/pods', icon: '📦' },
        { label: 'Deployments', href: '/resources/deployments', icon: '🚀' },
        { label: 'StatefulSets', href: '/resources/statefulsets', icon: '🗄️' },
        { label: 'DaemonSets', href: '/resources/daemonsets', icon: '🔄' },
        { label: 'Jobs', href: '/resources/jobs', icon: '⚙️' },
        { label: 'CronJobs', href: '/resources/cronjobs', icon: '🕐' },
      ]
    },
    {
      title: 'Network',
      items: [
        { label: 'Services', href: '/resources/services', icon: '🌐' },
        { label: 'Ingresses', href: '/resources/ingresses', icon: '🚪' },
      ]
    },
    {
      title: 'Config',
      items: [
        { label: 'ConfigMaps', href: '/resources/configmaps', icon: '📋' },
        { label: 'Secrets', href: '/resources/secrets', icon: '🔒' },
      ]
    },
    {
      title: 'Storage',
      items: [
        { label: 'PVCs', href: '/resources/pvcs', icon: '💾' },
      ]
    },
  ];

  function isActive(href: string): boolean {
    if (href === '/') return page.url.pathname === '/';
    return page.url.pathname.startsWith(href);
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
              <li>
                <a href={item.href} class:active={isActive(item.href)} class:disabled={!$isConnected && item.href !== '/'}>
                  <span class="icon">{item.icon}</span>
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
          <a href={item.href} class="icon-only" class:active={isActive(item.href)} title={item.label}>
            {item.icon}
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
    background: #0d1117;
    border-right: 1px solid #21262d;
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
    color: #8b949e;
    padding: 0.75rem;
    cursor: pointer;
    text-align: right;
    font-size: 0.875rem;
  }
  .toggle:hover { color: #e0e0e0; }
  .section { margin-bottom: 0.5rem; }
  .section-title {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #484f58;
    padding: 0.25rem 0.75rem;
    margin: 0.5rem 0 0.25rem;
  }
  ul { list-style: none; padding: 0; margin: 0; }
  li a {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    color: #8b949e;
    text-decoration: none;
    font-size: 0.85rem;
    border-radius: 4px;
    margin: 1px 4px;
    transition: background 0.15s;
  }
  li a:hover { background: #161b22; color: #e0e0e0; }
  li a.active { background: #1f2937; color: #58a6ff; font-weight: 500; }
  li a.disabled { opacity: 0.4; pointer-events: none; }
  .icon { font-size: 1rem; width: 1.25rem; text-align: center; }
  .icon-only {
    display: block;
    padding: 0.5rem;
    text-align: center;
    color: #8b949e;
    text-decoration: none;
    font-size: 1.1rem;
  }
  .icon-only:hover { background: #161b22; }
  .icon-only.active { background: #1f2937; color: #58a6ff; }
</style>
