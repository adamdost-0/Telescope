<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import {
    applyDynamicResource,
    applyResource,
    deleteDynamicResource,
    getDynamicResource,
    getEvents,
    getPods,
    getResource,
    getResources,
    getSecret,
    listCrds,
    rolloutRestart,
    rolloutStatus,
    scaleResource
  } from '$lib/api';
  import type { RolloutStatus } from '$lib/api';
  import Tabs from '$lib/components/Tabs.svelte';
  import EventsTable from '$lib/components/EventsTable.svelte';
  import YamlEditor from '$lib/components/YamlEditor.svelte';
  import ScaleDialog from '$lib/components/ScaleDialog.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import AzureIdentitySection from '$lib/components/AzureIdentitySection.svelte';
  import Breadcrumbs from '$lib/components/Breadcrumbs.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { decodeNamespaceParam, labelForGvk, parseGvk, resourceCollectionHref } from '$lib/resource-routing';
  import { isProduction } from '$lib/stores';
  import type { ResourceEntry } from '$lib/tauri-commands';

  const KIND_MAP: Record<string, { gvk: string; label: string }> = {
    deployments: { gvk: 'apps/v1/Deployment', label: 'Deployment' },
    services: { gvk: 'v1/Service', label: 'Service' },
    statefulsets: { gvk: 'apps/v1/StatefulSet', label: 'StatefulSet' },
    daemonsets: { gvk: 'apps/v1/DaemonSet', label: 'DaemonSet' },
    replicasets: { gvk: 'apps/v1/ReplicaSet', label: 'ReplicaSet' },
    jobs: { gvk: 'batch/v1/Job', label: 'Job' },
    cronjobs: { gvk: 'batch/v1/CronJob', label: 'CronJob' },
    configmaps: { gvk: 'v1/ConfigMap', label: 'ConfigMap' },
    secrets: { gvk: 'v1/Secret', label: 'Secret' },
    ingresses: { gvk: 'networking.k8s.io/v1/Ingress', label: 'Ingress' },
    networkpolicies: { gvk: 'networking.k8s.io/v1/NetworkPolicy', label: 'NetworkPolicy' },
    endpointslices: { gvk: 'discovery.k8s.io/v1/EndpointSlice', label: 'EndpointSlice' },
    pvcs: { gvk: 'v1/PersistentVolumeClaim', label: 'PVC' },
    resourcequotas: { gvk: 'v1/ResourceQuota', label: 'ResourceQuota' },
    limitranges: { gvk: 'v1/LimitRange', label: 'LimitRange' },
    roles: { gvk: 'rbac.authorization.k8s.io/v1/Role', label: 'Role' },
    clusterroles: { gvk: 'rbac.authorization.k8s.io/v1/ClusterRole', label: 'ClusterRole' },
    rolebindings: { gvk: 'rbac.authorization.k8s.io/v1/RoleBinding', label: 'RoleBinding' },
    clusterrolebindings: { gvk: 'rbac.authorization.k8s.io/v1/ClusterRoleBinding', label: 'ClusterRoleBinding' },
    serviceaccounts: { gvk: 'v1/ServiceAccount', label: 'ServiceAccount' },
    validatingwebhookconfigurations: {
      gvk: 'admissionregistration.k8s.io/v1/ValidatingWebhookConfiguration',
      label: 'ValidatingWebhookConfiguration'
    },
    mutatingwebhookconfigurations: {
      gvk: 'admissionregistration.k8s.io/v1/MutatingWebhookConfiguration',
      label: 'MutatingWebhookConfiguration'
    },
    hpas: { gvk: 'autoscaling/v2/HorizontalPodAutoscaler', label: 'HPA' },
    poddisruptionbudgets: { gvk: 'policy/v1/PodDisruptionBudget', label: 'PodDisruptionBudget' },
    priorityclasses: { gvk: 'scheduling.k8s.io/v1/PriorityClass', label: 'PriorityClass' },
    storageclasses: { gvk: 'storage.k8s.io/v1/StorageClass', label: 'StorageClass' },
    persistentvolumes: { gvk: 'v1/PersistentVolume', label: 'PersistentVolume' },
  };

  const WORKLOAD_KINDS = new Set(['deployments', 'statefulsets', 'daemonsets']);
  const SCALABLE_KINDS = new Set(['deployments', 'statefulsets']);
  const RESTARTABLE_KINDS = new Set(['deployments', 'statefulsets', 'daemonsets']);
  const ROLLOUT_STATUS_KINDS = new Set(['deployments', 'statefulsets']);
  const READONLY_KINDS = new Set(['validatingwebhookconfigurations', 'mutatingwebhookconfigurations']);

  let kind = $derived(page.params.kind);
  let namespaceParam = $derived(page.params.namespace);
  let namespace = $derived(decodeNamespaceParam(namespaceParam));
  let resourceNamespace = $derived(namespace ?? '');
  let namespaceLabel = $derived(namespace ?? 'Cluster-scoped');
  let resourceName = $derived(page.params.name);
  let kindInfo = $derived(KIND_MAP[kind]);
  let fallbackGvk = $derived(page.url.searchParams.get('gvk') ?? '');
  let effectiveGvk = $derived(kindInfo?.gvk ?? fallbackGvk);
  let resourceLabel = $derived(kindInfo?.label ?? page.url.searchParams.get('label') ?? (effectiveGvk ? labelForGvk(effectiveGvk) : kind));
  let parsedFallbackGvk = $derived(parseGvk(effectiveGvk));
  let dynamicGroup = $derived(parsedFallbackGvk.group ?? '');
  let dynamicVersion = $derived(parsedFallbackGvk.version ?? '');
  let dynamicKind = $derived(parsedFallbackGvk.kind);
  let dynamicPlural = $state(page.url.searchParams.get('plural') ?? '');
  let isDynamicResource = $derived(!kindInfo && !!effectiveGvk);
  let collectionHref = $derived.by(() => {
    if (!effectiveGvk) return null;
    if (isDynamicResource && dynamicGroup && dynamicVersion) {
      const params = new URLSearchParams({ version: dynamicVersion });
      if (dynamicPlural) params.set('plural', dynamicPlural);
      return `/crds/${encodeURIComponent(dynamicGroup)}/${encodeURIComponent(dynamicKind)}?${params.toString()}`;
    }
    return resourceCollectionHref(effectiveGvk);
  });
  let namespaceHref = $derived(kindInfo && namespace ? `/resources/${kind}?namespace=${encodeURIComponent(namespace)}` : null);
  let isSecret = $derived(kind === 'secrets');
  let isWorkload = $derived(WORKLOAD_KINDS.has(kind));
  let isScalable = $derived(SCALABLE_KINDS.has(kind));
  let isRestartable = $derived(RESTARTABLE_KINDS.has(kind));
  let hasRolloutStatus = $derived(ROLLOUT_STATUS_KINDS.has(kind));
  let isReadOnly = $derived(READONLY_KINDS.has(kind));
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);
  let showScale = $state(false);
  let currentReplicas = $derived(resource?.spec?.replicas ?? 1);

  let resource: any = $state(null);
  let events: ResourceEntry[] = $state([]);
  let ownedPods: ResourceEntry[] = $state([]);
  let loading = $state(true);
  let activeTab = $state('summary');
  let error: string | null = $state(null);

  let rollout: RolloutStatus | null = $state(null);
  let rolloutLoading = $state(false);
  let rolloutMessage: string | null = $state(null);
  let showRestartConfirm = $state(false);
  let showProdRestartConfirm = $state(false);
  let pendingScaleReplicas: number | null = $state(null);
  let showScaleConfirm = $state(false);

  // YAML editor state
  let editedYaml = $state('');
  let yamlContent = $derived(resource ? JSON.stringify(resource, null, 2) : '');
  let yamlDirty = $derived(editedYaml !== '' && editedYaml !== yamlContent);
  let applying = $state(false);
  let applyMessage: string | null = $state(null);
  let applyError = $state(false);

  let tabs = $derived(buildTabs());

  async function resolveDynamicPlural(): Promise<string | null> {
    if (!isDynamicResource || !dynamicGroup || !dynamicVersion) return null;
    if (dynamicPlural) return dynamicPlural;
    try {
      const crds = await listCrds();
      const match = crds.find(
        (crd) => crd.group === dynamicGroup && crd.version === dynamicVersion && crd.kind === dynamicKind,
      );
      if (match) {
        dynamicPlural = match.plural;
        return match.plural;
      }
    } catch {
      // Ignore CRD lookup failures and let the caller surface a better error.
    }
    return null;
  }

  function buildTabs() {
    const t = [
      { id: 'summary', label: 'Summary' },
      { id: 'events', label: 'Events' },
      { id: 'yaml', label: 'YAML' },
    ];
    if (isWorkload) {
      t.splice(1, 0, { id: 'pods', label: 'Pods' });
    }
    return t;
  }

  async function loadResource() {
    loading = true;
    error = null;
    resource = null;
    events = [];
    ownedPods = [];

    try {
      if (!effectiveGvk) {
        error = `Unknown resource kind: "${kind}"`;
        return;
      }

      let entry: ResourceEntry | null = null;
      if (isSecret && namespace) {
        entry = await getSecret(resourceNamespace, resourceName);
      } else if (isDynamicResource) {
        const plural = await resolveDynamicPlural();
        if (!plural) {
          error = `Unknown resource kind: "${kind}"`;
          return;
        }
        entry = await getDynamicResource(dynamicGroup, dynamicVersion, plural, namespace, resourceName);
      } else {
        entry = namespace
          ? (await getResource(effectiveGvk, resourceNamespace, resourceName))
            ?? (await getResources(effectiveGvk, resourceNamespace)).find((r: ResourceEntry) => r.name === resourceName)
          : (await getResources(effectiveGvk)).find((r: ResourceEntry) => r.name === resourceName);
      }

      if (entry) {
        resource = JSON.parse(entry.content);
      } else {
        error = `${resourceLabel} "${resourceName}" not found${namespace ? ` in namespace "${namespace}"` : ''}`;
        return;
      }

      events = await getEvents(namespace, resourceName);

      if (isWorkload && resource?.spec?.selector?.matchLabels) {
        const allPods = await getPods(resourceNamespace);
        const selectorLabels = resource.spec.selector.matchLabels as Record<string, string>;
        ownedPods = allPods.filter((p: ResourceEntry) => {
          try {
            const pod = JSON.parse(p.content);
            const podLabels = pod.metadata?.labels ?? {};
            return Object.entries(selectorLabels).every(([k, v]) => podLabels[k] === v);
          } catch {
            return false;
          }
        });
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load resource';
    } finally {
      loading = false;
      loadRolloutStatus();
    }
  }

  $effect(() => {
    void kind;
    void namespaceParam;
    void resourceName;
    loadResource();
  });

  async function handleApply(dryRun: boolean) {
    applying = true;
    applyMessage = null;
    applyError = false;
    try {
      let result;
      if (isDynamicResource) {
        const plural = await resolveDynamicPlural();
        if (!plural) {
          throw new Error(`Unable to resolve plural for ${dynamicKind}`);
        }
        result = await applyDynamicResource(
          dynamicGroup,
          dynamicVersion,
          dynamicKind,
          plural,
          namespace,
          editedYaml,
          dryRun,
        );
      } else {
        result = await applyResource(editedYaml, dryRun);
      }
      if (result.success) {
        applyMessage = result.message;
        if (!dryRun) {
          await loadResource();
        }
      } else {
        applyMessage = result.message;
        applyError = true;
      }
    } catch (e) {
      applyMessage = e instanceof Error ? e.message : String(e);
      applyError = true;
    } finally {
      applying = false;
    }
  }

  function resetYaml() {
    editedYaml = yamlContent;
    applyMessage = null;
    applyError = false;
  }

  async function handleDelete() {
    if (!isDynamicResource) return;
    deleting = true;
    applyMessage = null;
    applyError = false;
    try {
      const plural = await resolveDynamicPlural();
      if (!plural) {
        throw new Error(`Unable to resolve plural for ${dynamicKind}`);
      }
      await deleteDynamicResource(dynamicGroup, dynamicVersion, dynamicKind, plural, namespace, resourceName);
      showDeleteConfirm = false;
      await goto(collectionHref ?? '/crds');
    } catch (e) {
      applyMessage = e instanceof Error ? e.message : String(e);
      applyError = true;
    } finally {
      deleting = false;
    }
  }

  async function loadRolloutStatus() {
    if (!hasRolloutStatus || !kindInfo) return;
    try {
      rollout = await rolloutStatus(kindInfo.gvk, resourceNamespace, resourceName);
    } catch {
      rollout = null;
    }
  }

  async function handleRolloutRestart() {
    showRestartConfirm = false;
    showProdRestartConfirm = false;
    rolloutLoading = true;
    rolloutMessage = null;
    try {
      rolloutMessage = await rolloutRestart(kindInfo.gvk, resourceNamespace, resourceName);
      setTimeout(loadRolloutStatus, 2000);
    } catch (e) {
      rolloutMessage = `Error: ${e instanceof Error ? e.message : e}`;
    } finally {
      rolloutLoading = false;
    }
  }

  function formatAge(ts: string): string {
    if (!ts) return 'N/A';
    const diffMs = Date.now() - new Date(ts).getTime();
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 60) return `${diffSec}s`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h`;
    return `${Math.floor(diffHours / 24)}d`;
  }

  function podPhase(entry: ResourceEntry): string {
    try {
      return JSON.parse(entry.content).status?.phase ?? 'Unknown';
    } catch {
      return 'Unknown';
    }
  }

  function podRestarts(entry: ResourceEntry): number {
    try {
      const statuses = JSON.parse(entry.content).status?.containerStatuses ?? [];
      return statuses.reduce((sum: number, s: any) => sum + (s.restartCount ?? 0), 0);
    } catch {
      return 0;
    }
  }

  function podAge(entry: ResourceEntry): string {
    try {
      return formatAge(JSON.parse(entry.content).metadata?.creationTimestamp ?? '');
    } catch {
      return 'N/A';
    }
  }

  function truncate(str: string, max: number): string {
    return str.length > max ? str.slice(0, max) + '…' : str;
  }

  function resourceQuantity(value: unknown): string {
    if (value === null || value === undefined || value === '') return '—';
    return String(value);
  }

  function formatQuotaRows() {
    const hard = resource?.status?.hard ?? resource?.spec?.hard ?? {};
    const used = resource?.status?.used ?? {};
    const resourceNames = Array.from(new Set([...Object.keys(hard), ...Object.keys(used)])).sort();
    return resourceNames.map((name) => ({
      name,
      hard: resourceQuantity(hard[name]),
      used: resourceQuantity(used[name]),
    }));
  }

  function formatLimitValue(limit: Record<string, unknown> | undefined, key: string): string {
    if (!limit) return '—';
    const value = limit[key];
    if (!value || typeof value !== 'object') return '—';
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return '—';
    return entries.map(([name, amount]) => `${name}: ${resourceQuantity(amount)}`).join(', ');
  }

  function formatLimitRangeSections() {
    const limits = Array.isArray(resource?.spec?.limits) ? resource.spec.limits : [];
    const grouped = new Map<string, Array<{
      defaultValue: string;
      defaultRequest: string;
      min: string;
      max: string;
    }>>();

    for (const limit of limits) {
      const type = String(limit?.type ?? 'Unknown');
      const rows = grouped.get(type) ?? [];
      rows.push({
        defaultValue: formatLimitValue(limit, 'default'),
        defaultRequest: formatLimitValue(limit, 'defaultRequest'),
        min: formatLimitValue(limit, 'min'),
        max: formatLimitValue(limit, 'max'),
      });
      grouped.set(type, rows);
    }

    return Array.from(grouped.entries());
  }

  function formatList(values: unknown[] | undefined, fallback = 'All'): string {
    if (!Array.isArray(values) || values.length === 0) return fallback;
    return values.join(', ');
  }

  function formatClientConfig(clientConfig: any): string {
    if (clientConfig?.service?.name) {
      const serviceNamespace = clientConfig.service.namespace ?? 'default';
      const path = clientConfig.service.path ? ` (${clientConfig.service.path})` : '';
      const port = clientConfig.service.port ? `:${clientConfig.service.port}` : '';
      return `${serviceNamespace}/${clientConfig.service.name}${port}${path}`;
    }
    if (clientConfig?.url) return clientConfig.url;
    return 'None';
  }

  function selectorParts(selector: any): string[] {
    if (!selector) return [];

    const labels = Object.entries(selector.matchLabels ?? {}).map(([key, value]) => `${key}=${value}`);
    const expressions = (selector.matchExpressions ?? []).map((expression: any) => {
      const operator = expression.operator ?? 'Exists';
      const values = Array.isArray(expression.values) && expression.values.length > 0
        ? ` (${expression.values.join(', ')})`
        : '';
      return `${expression.key ?? 'unknown'} ${operator}${values}`;
    });

    return [...labels, ...expressions];
  }

  function formatSelector(selector: any): string {
    const parts = selectorParts(selector);
    return parts.length > 0 ? parts.join(', ') : 'All';
  }

  function effectiveNetworkPolicyTypes(spec: any): string[] {
    const explicitTypes = spec?.policyTypes;
    if (Array.isArray(explicitTypes) && explicitTypes.length > 0) {
      return explicitTypes;
    }

    const inferredTypes = ['Ingress'];
    if (spec?.egress !== undefined) {
      inferredTypes.push('Egress');
    }

    return inferredTypes;
  }

  function formatNetworkPolicyPeers(peers: any[] | undefined, direction: 'from' | 'to'): string {
    if (!peers || peers.length === 0) {
      return 'All';
    }

    return peers.map((peer: any) => {
      const parts: string[] = [];

      if (peer?.podSelector) {
        parts.push(`Pods: ${formatSelector(peer.podSelector)}`);
      }

      if (peer?.namespaceSelector) {
        parts.push(`Namespaces: ${formatSelector(peer.namespaceSelector)}`);
      }

      if (peer?.ipBlock?.cidr) {
        const except = peer.ipBlock.except?.length ? ` except ${peer.ipBlock.except.join(', ')}` : '';
        parts.push(`CIDR: ${peer.ipBlock.cidr}${except}`);
      }

      if (parts.length === 0) {
        return direction === 'from' ? 'All sources' : 'All destinations';
      }

      return parts.join(' · ');
    }).join('; ');
  }

  function formatPortValue(value: unknown): string {
    if (value === undefined || value === null || value === '') return 'any';
    if (typeof value === 'string' || typeof value === 'number') return String(value);
    return JSON.stringify(value);
  }

  function formatNetworkPolicyPorts(ports: any[] | undefined): string {
    if (!ports || ports.length === 0) {
      return 'All';
    }

    return ports.map((port: any) => {
      const protocol = port?.protocol ?? 'TCP';
      const start = formatPortValue(port?.port);
      const end = port?.endPort ? `-${port.endPort}` : '';
      return `${protocol} ${start}${end}`;
    }).join(', ');
  }

  function formatEndpointConditions(conditions: any): string {
    if (!conditions) return 'Unknown';

    const entries = [
      ['ready', conditions.ready],
      ['serving', conditions.serving],
      ['terminating', conditions.terminating],
    ].filter(([, value]) => value !== undefined);

    if (entries.length === 0) return 'Unknown';
    return entries.map(([key, value]) => `${key}=${value ? 'true' : 'false'}`).join(', ');
  }

  function formatTargetRef(targetRef: any): string {
    if (!targetRef?.kind || !targetRef?.name) return 'N/A';
    const ns = targetRef.namespace ? `${targetRef.namespace}/` : '';
    return `${targetRef.kind} ${ns}${targetRef.name}`;
  }

  function formatEndpointZone(endpoint: any): string {
    if (endpoint?.zone) return endpoint.zone;
    const hintedZones = endpoint?.hints?.forZones?.map((zone: any) => zone?.name).filter(Boolean) ?? [];
    return hintedZones.join(', ') || 'N/A';
  }

  async function handleScale(replicas: number) {
    showScale = false;
    if (!kindInfo) return;
    if ($isProduction) {
      pendingScaleReplicas = replicas;
      showScaleConfirm = true;
      return;
    }
    await executeScale(replicas);
  }

  async function executeScale(replicas: number) {
    if (!kindInfo) return;
    try {
      await scaleResource(kindInfo.gvk, resourceNamespace, resourceName, replicas);
      await loadResource();
    } catch (e) {
      console.error('Scale failed:', e);
    }
  }

  async function confirmScale() {
    showScaleConfirm = false;
    if (pendingScaleReplicas !== null) {
      await executeScale(pendingScaleReplicas);
      pendingScaleReplicas = null;
    }
  }
</script>

<div class="detail-page">
  <header class="detail-header">
    <Breadcrumbs crumbs={[
      { label: 'Overview', href: '/overview' },
      { label: resourceLabel, href: collectionHref ?? undefined },
      ...(namespace ? [{ label: namespace, href: namespaceHref ?? undefined }] : []),
      { label: resourceName }
    ]} />
    <h1>{resourceName}</h1>
    <span class="namespace-badge">{namespaceLabel}</span>
    {#if isScalable && resource}
      <button class="action-btn" onclick={() => showScale = true} data-testid="resource-scale-btn">
        <Icon name="scale" size={14} aria-hidden="true" /> Scale
      </button>
    {/if}
    {#if isDynamicResource && resource}
      <button class="action-btn danger" onclick={() => (showDeleteConfirm = true)} data-testid="resource-delete-btn">
        <Icon name="delete" size={14} aria-hidden="true" /> Delete
      </button>
    {/if}
  </header>

  {#if loading}
    <p role="status">Loading {resourceLabel.toLowerCase()} details…</p>
  {:else if error}
    <p role="alert" class="error">{error}</p>
  {:else if resource}
    <Tabs {tabs} {activeTab} onchange={(id) => activeTab = id} />

    {#if activeTab === 'summary'}
      <div class="summary">
        <!-- Deployment Summary -->
        {#if kind === 'deployments'}
          <h3>Replicas</h3>
          <dl>
            <dt>Desired</dt><dd>{resource.spec?.replicas ?? 0}</dd>
            <dt>Ready</dt><dd>{resource.status?.readyReplicas ?? 0}</dd>
            <dt>Updated</dt><dd>{resource.status?.updatedReplicas ?? 0}</dd>
            <dt>Available</dt><dd>{resource.status?.availableReplicas ?? 0}</dd>
          </dl>

          <h3>Strategy</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.strategy?.type ?? 'RollingUpdate'}</dd>
            {#if resource.spec?.strategy?.rollingUpdate}
              <dt>Max Surge</dt><dd>{resource.spec.strategy.rollingUpdate.maxSurge ?? 'N/A'}</dd>
              <dt>Max Unavailable</dt><dd>{resource.spec.strategy.rollingUpdate.maxUnavailable ?? 'N/A'}</dd>
            {/if}
          </dl>

          <h3>Selector</h3>
          {#if resource.spec?.selector?.matchLabels}
            <div class="labels">
              {#each Object.entries(resource.spec.selector.matchLabels) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector labels</p>
          {/if}

          <h3>Container Images</h3>
          {#each resource.spec?.template?.spec?.containers ?? [] as container}
            <div class="container-card">
              <strong>{container.name}</strong>
              <span class="muted">{container.image}</span>
            </div>
          {/each}

          <h3>Rollout</h3>
          <div class="rollout-section">
            {#if rollout}
              <dl>
                <dt>Status</dt>
                <dd>
                  {#if rollout.is_complete}
                    <span class="badge badge-success">Complete</span>
                  {:else}
                    <span class="badge badge-pending">In Progress</span>
                  {/if}
                  {rollout.message}
                </dd>
                <dt>Progress</dt>
                <dd>{rollout.ready}/{rollout.desired} ready, {rollout.updated}/{rollout.desired} updated, {rollout.available}/{rollout.desired} available</dd>
              </dl>
            {/if}
            <div class="rollout-actions">
              {#if showRestartConfirm}
                <span class="confirm-prompt">Restart all pods?</span>
                <button class="btn btn-danger btn-sm" onclick={handleRolloutRestart} disabled={rolloutLoading}>
                  {rolloutLoading ? 'Restarting…' : 'Confirm Restart'}
                </button>
                <button class="btn btn-secondary btn-sm" onclick={() => showRestartConfirm = false}>Cancel</button>
              {:else}
                <button class="btn btn-primary btn-sm" onclick={() => {
                  if ($isProduction) {
                    showProdRestartConfirm = true;
                  } else {
                    showRestartConfirm = true;
                  }
                }} disabled={rolloutLoading}>
                  Restart Rollout
                </button>
              {/if}
            </div>
            {#if rolloutMessage}
              <p class="rollout-message">{rolloutMessage}</p>
            {/if}
          </div>

        <!-- Service Summary -->
        {:else if kind === 'services'}
          <h3>Service Info</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.type ?? 'ClusterIP'}</dd>
            <dt>Cluster IP</dt><dd>{resource.spec?.clusterIP ?? 'None'}</dd>
            {#if resource.spec?.externalIPs?.length}
              <dt>External IPs</dt><dd>{resource.spec.externalIPs.join(', ')}</dd>
            {/if}
            {#if resource.spec?.loadBalancerIP}
              <dt>Load Balancer IP</dt><dd>{resource.spec.loadBalancerIP}</dd>
            {/if}
            {#if resource.status?.loadBalancer?.ingress?.length}
              <dt>External</dt>
              <dd>{resource.status.loadBalancer.ingress.map((i: any) => i.ip ?? i.hostname ?? '').join(', ')}</dd>
            {/if}
            <dt>Session Affinity</dt><dd>{resource.spec?.sessionAffinity ?? 'None'}</dd>
          </dl>

          <h3>Ports</h3>
          {#if resource.spec?.ports?.length}
            <table>
              <thead><tr><th scope="col">Name</th><th scope="col">Port</th><th scope="col">Target</th><th scope="col">Protocol</th><th scope="col">Node Port</th></tr></thead>
              <tbody>
                {#each resource.spec.ports as port}
                  <tr>
                    <td>{port.name ?? ''}</td>
                    <td>{port.port}</td>
                    <td>{port.targetPort ?? port.port}</td>
                    <td>{port.protocol ?? 'TCP'}</td>
                    <td>{port.nodePort ?? '—'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No ports defined</p>
          {/if}

          <h3>Selector</h3>
          {#if resource.spec?.selector}
            <div class="labels">
              {#each Object.entries(resource.spec.selector) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector</p>
          {/if}

        <!-- StatefulSet Summary -->
        {:else if kind === 'statefulsets'}
          <h3>Replicas</h3>
          <dl>
            <dt>Desired</dt><dd>{resource.spec?.replicas ?? 0}</dd>
            <dt>Ready</dt><dd>{resource.status?.readyReplicas ?? 0}</dd>
            <dt>Current</dt><dd>{resource.status?.currentReplicas ?? 0}</dd>
          </dl>

          <h3>Update Strategy</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.updateStrategy?.type ?? 'RollingUpdate'}</dd>
            <dt>Service Name</dt><dd>{resource.spec?.serviceName ?? 'N/A'}</dd>
            <dt>Pod Management</dt><dd>{resource.spec?.podManagementPolicy ?? 'OrderedReady'}</dd>
          </dl>

          <h3>Selector</h3>
          {#if resource.spec?.selector?.matchLabels}
            <div class="labels">
              {#each Object.entries(resource.spec.selector.matchLabels) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector labels</p>
          {/if}

          {#if resource.spec?.volumeClaimTemplates?.length}
            <h3>Volume Claim Templates</h3>
            {#each resource.spec.volumeClaimTemplates as vct}
              <div class="container-card">
                <strong>{vct.metadata?.name ?? 'unnamed'}</strong>
                <span class="muted">
                  {vct.spec?.accessModes?.join(', ') ?? 'N/A'} —
                  {vct.spec?.resources?.requests?.storage ?? 'N/A'}
                </span>
              </div>
            {/each}
          {/if}

          <h3>Rollout</h3>
          <div class="rollout-section">
            {#if rollout}
              <dl>
                <dt>Status</dt>
                <dd>
                  {#if rollout.is_complete}
                    <span class="badge badge-success">Complete</span>
                  {:else}
                    <span class="badge badge-pending">In Progress</span>
                  {/if}
                  {rollout.message}
                </dd>
                <dt>Progress</dt>
                <dd>{rollout.ready}/{rollout.desired} ready, {rollout.updated}/{rollout.desired} updated</dd>
              </dl>
            {/if}
            <div class="rollout-actions">
              {#if showRestartConfirm}
                <span class="confirm-prompt">Restart all pods?</span>
                <button class="btn btn-danger btn-sm" onclick={handleRolloutRestart} disabled={rolloutLoading}>
                  {rolloutLoading ? 'Restarting…' : 'Confirm Restart'}
                </button>
                <button class="btn btn-secondary btn-sm" onclick={() => showRestartConfirm = false}>Cancel</button>
              {:else}
                <button class="btn btn-primary btn-sm" onclick={() => {
                  if ($isProduction) {
                    showProdRestartConfirm = true;
                  } else {
                    showRestartConfirm = true;
                  }
                }} disabled={rolloutLoading}>
                  Restart Rollout
                </button>
              {/if}
            </div>
            {#if rolloutMessage}
              <p class="rollout-message">{rolloutMessage}</p>
            {/if}
          </div>

        <!-- DaemonSet Summary -->
        {:else if kind === 'daemonsets'}
          <h3>Status</h3>
          <dl>
            <dt>Desired</dt><dd>{resource.status?.desiredNumberScheduled ?? 0}</dd>
            <dt>Current</dt><dd>{resource.status?.currentNumberScheduled ?? 0}</dd>
            <dt>Ready</dt><dd>{resource.status?.numberReady ?? 0}</dd>
            <dt>Up-to-date</dt><dd>{resource.status?.updatedNumberScheduled ?? 0}</dd>
            <dt>Available</dt><dd>{resource.status?.numberAvailable ?? 0}</dd>
          </dl>

          <h3>Update Strategy</h3>
          <dl>
            <dt>Type</dt><dd>{resource.spec?.updateStrategy?.type ?? 'RollingUpdate'}</dd>
          </dl>

          <h3>Selector</h3>
          {#if resource.spec?.selector?.matchLabels}
            <div class="labels">
              {#each Object.entries(resource.spec.selector.matchLabels) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {:else}
            <p class="muted">No selector labels</p>
          {/if}

          <h3>Rollout Restart</h3>
          <div class="rollout-section">
            <div class="rollout-actions">
              {#if showRestartConfirm}
                <span class="confirm-prompt">Restart all pods?</span>
                <button class="btn btn-danger btn-sm" onclick={handleRolloutRestart} disabled={rolloutLoading}>
                  {rolloutLoading ? 'Restarting…' : 'Confirm Restart'}
                </button>
                <button class="btn btn-secondary btn-sm" onclick={() => showRestartConfirm = false}>Cancel</button>
              {:else}
                <button class="btn btn-primary btn-sm" onclick={() => {
                  if ($isProduction) {
                    showProdRestartConfirm = true;
                  } else {
                    showRestartConfirm = true;
                  }
                }} disabled={rolloutLoading}>
                  Restart Rollout
                </button>
              {/if}
            </div>
            {#if rolloutMessage}
              <p class="rollout-message">{rolloutMessage}</p>
            {/if}
          </div>

        <!-- Job Summary -->
        {:else if kind === 'jobs'}
          <h3>Spec</h3>
          <dl>
            <dt>Completions</dt><dd>{resource.spec?.completions ?? 1}</dd>
            <dt>Parallelism</dt><dd>{resource.spec?.parallelism ?? 1}</dd>
            <dt>Backoff Limit</dt><dd>{resource.spec?.backoffLimit ?? 6}</dd>
          </dl>

          <h3>Status</h3>
          <dl>
            <dt>Active</dt><dd>{resource.status?.active ?? 0}</dd>
            <dt>Succeeded</dt><dd>{resource.status?.succeeded ?? 0}</dd>
            <dt>Failed</dt><dd>{resource.status?.failed ?? 0}</dd>
            {#if resource.status?.startTime}
              <dt>Start Time</dt><dd>{resource.status.startTime}</dd>
            {/if}
            {#if resource.status?.completionTime}
              <dt>Completion Time</dt><dd>{resource.status.completionTime}</dd>
              <dt>Duration</dt>
              <dd>{formatAge(resource.status.startTime)} → completed</dd>
            {/if}
          </dl>

        <!-- CronJob Summary -->
        {:else if kind === 'cronjobs'}
          <h3>Schedule</h3>
          <dl>
            <dt>Schedule</dt><dd>{resource.spec?.schedule ?? 'N/A'}</dd>
            <dt>Suspend</dt><dd>{resource.spec?.suspend ? 'Yes' : 'No'}</dd>
            <dt>Concurrency Policy</dt><dd>{resource.spec?.concurrencyPolicy ?? 'Allow'}</dd>
            {#if resource.status?.lastScheduleTime}
              <dt>Last Scheduled</dt><dd>{resource.status.lastScheduleTime}</dd>
            {/if}
          </dl>

          {#if resource.status?.active?.length}
            <h3>Active Jobs</h3>
            {#each resource.status.active as job}
              <div class="container-card">
                <strong>{job.name ?? 'unnamed'}</strong>
                <span class="muted">{job.namespace ?? namespace}</span>
              </div>
            {/each}
          {/if}

        <!-- ConfigMap Summary -->
        {:else if kind === 'configmaps'}
          <h3>Data ({Object.keys(resource.data ?? {}).length} keys)</h3>
          {#if resource.data && Object.keys(resource.data).length > 0}
            <table>
              <thead><tr><th scope="col">Key</th><th scope="col">Value</th></tr></thead>
              <tbody>
                {#each Object.entries(resource.data) as [key, value]}
                  <tr>
                    <td class="resource-name">{key}</td>
                    <td class="muted">{truncate(String(value), 120)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No data</p>
          {/if}

        <!-- Secret Summary -->
        {:else if kind === 'secrets'}
          <h3>Info</h3>
          <dl>
            <dt>Type</dt><dd>{resource.type ?? 'Opaque'}</dd>
            <dt>Keys</dt><dd>{Object.keys(resource.data ?? {}).length}</dd>
          </dl>

          <h3>Data Keys</h3>
          {#if resource.data && Object.keys(resource.data).length > 0}
            <table>
              <thead><tr><th scope="col">Key</th><th scope="col">Value</th></tr></thead>
              <tbody>
                {#each Object.keys(resource.data) as key}
                  <tr>
                    <td class="resource-name">{key}</td>
                    <td class="muted">••••••••</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No data</p>
          {/if}

        <!-- Ingress Summary -->
        {:else if kind === 'ingresses'}
          <h3>Info</h3>
          <dl>
            {#if resource.spec?.ingressClassName}
              <dt>Ingress Class</dt><dd>{resource.spec.ingressClassName}</dd>
            {/if}
          </dl>

          {#if resource.spec?.tls?.length}
            <h3>TLS</h3>
            {#each resource.spec.tls as tls}
              <div class="container-card">
                <strong>{tls.secretName ?? 'unnamed'}</strong>
                <span class="muted">{(tls.hosts ?? []).join(', ')}</span>
              </div>
            {/each}
          {/if}

          {#if resource.spec?.rules?.length}
            <h3>Rules</h3>
            {#each resource.spec.rules as rule}
              <div class="container-card">
                <strong>{rule.host ?? '*'}</strong>
                {#each rule.http?.paths ?? [] as path}
                  <span class="muted">
                    {path.path ?? '/'} ({path.pathType ?? 'Prefix'}) →
                    {path.backend?.service?.name ?? '?'}:{path.backend?.service?.port?.number ?? path.backend?.service?.port?.name ?? '?'}
                  </span>
                {/each}
              </div>
            {/each}
          {/if}

        <!-- NetworkPolicy Summary -->
        {:else if kind === 'networkpolicies'}
          <h3>Info</h3>
          <dl>
            <dt>Pod Selector</dt><dd>{formatSelector(resource.spec?.podSelector)}</dd>
            <dt>Policy Types</dt><dd>{effectiveNetworkPolicyTypes(resource.spec).join(', ')}</dd>
          </dl>

          <h3>Ingress Rules</h3>
          {#if resource.spec?.ingress?.length}
            <table>
              <thead>
                <tr>
                  <th scope="col">Rule</th>
                  <th scope="col">From</th>
                  <th scope="col">Ports</th>
                </tr>
              </thead>
              <tbody>
                {#each resource.spec.ingress as rule, index}
                  <tr>
                    <td>#{index + 1}</td>
                    <td class="muted">{formatNetworkPolicyPeers(rule.from, 'from')}</td>
                    <td>{formatNetworkPolicyPorts(rule.ports)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No ingress rules</p>
          {/if}

          <h3>Egress Rules</h3>
          {#if resource.spec?.egress?.length}
            <table>
              <thead>
                <tr>
                  <th scope="col">Rule</th>
                  <th scope="col">To</th>
                  <th scope="col">Ports</th>
                </tr>
              </thead>
              <tbody>
                {#each resource.spec.egress as rule, index}
                  <tr>
                    <td>#{index + 1}</td>
                    <td class="muted">{formatNetworkPolicyPeers(rule.to, 'to')}</td>
                    <td>{formatNetworkPolicyPorts(rule.ports)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No egress rules</p>
          {/if}

        <!-- EndpointSlice Summary -->
        {:else if kind === 'endpointslices'}
          <h3>Info</h3>
          <dl>
            <dt>Address Type</dt><dd>{resource.addressType ?? 'N/A'}</dd>
          </dl>

          <h3>Ports</h3>
          {#if resource.ports?.length}
            <table>
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">Protocol</th>
                  <th scope="col">Port</th>
                  <th scope="col">App Protocol</th>
                </tr>
              </thead>
              <tbody>
                {#each resource.ports as port}
                  <tr>
                    <td>{port.name ?? 'N/A'}</td>
                    <td>{port.protocol ?? 'TCP'}</td>
                    <td>{port.port ?? 'N/A'}</td>
                    <td>{port.appProtocol ?? 'N/A'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No ports</p>
          {/if}

          <h3>Endpoints</h3>
          {#if resource.endpoints?.length}
            <table>
              <thead>
                <tr>
                  <th scope="col">Addresses</th>
                  <th scope="col">Conditions</th>
                  <th scope="col">Target Ref</th>
                  <th scope="col">Node</th>
                  <th scope="col">Zone</th>
                </tr>
              </thead>
              <tbody>
                {#each resource.endpoints as endpoint}
                  <tr>
                    <td>{(endpoint.addresses ?? []).join(', ') || 'N/A'}</td>
                    <td>{formatEndpointConditions(endpoint.conditions)}</td>
                    <td>{formatTargetRef(endpoint.targetRef)}</td>
                    <td>{endpoint.nodeName ?? 'N/A'}</td>
                    <td>{formatEndpointZone(endpoint)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No endpoints</p>
          {/if}

        <!-- PVC Summary -->
        {:else if kind === 'pvcs'}
          <h3>Info</h3>
          <dl>
            <dt>Status</dt><dd>{resource.status?.phase ?? 'Unknown'}</dd>
            <dt>Volume</dt><dd>{resource.spec?.volumeName ?? 'N/A'}</dd>
            <dt>Storage Class</dt><dd>{resource.spec?.storageClassName ?? 'N/A'}</dd>
            <dt>Access Modes</dt><dd>{(resource.spec?.accessModes ?? []).join(', ') || 'N/A'}</dd>
            <dt>Capacity</dt><dd>{resource.status?.capacity?.storage ?? 'N/A'}</dd>
            <dt>Requested</dt><dd>{resource.spec?.resources?.requests?.storage ?? 'N/A'}</dd>
          </dl>

        <!-- ResourceQuota Summary -->
        {:else if kind === 'resourcequotas'}
          <h3>Quota Usage</h3>
          {#if formatQuotaRows().length > 0}
            <table>
              <thead><tr><th scope="col">Resource</th><th scope="col">Hard</th><th scope="col">Used</th></tr></thead>
              <tbody>
                {#each formatQuotaRows() as row}
                  <tr>
                    <td class="resource-name">{row.name}</td>
                    <td>{row.hard}</td>
                    <td>{row.used}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No quota values reported</p>
          {/if}

        <!-- LimitRange Summary -->
        {:else if kind === 'limitranges'}
          <h3>Limits</h3>
          {#if formatLimitRangeSections().length > 0}
            {#each formatLimitRangeSections() as [type, rows]}
              <h4>{type}</h4>
              <table>
                <thead>
                  <tr>
                    <th scope="col">Default</th>
                    <th scope="col">DefaultRequest</th>
                    <th scope="col">Min</th>
                    <th scope="col">Max</th>
                  </tr>
                </thead>
                <tbody>
                  {#each rows as row}
                    <tr>
                      <td>{row.defaultValue}</td>
                      <td>{row.defaultRequest}</td>
                      <td>{row.min}</td>
                      <td>{row.max}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/each}
          {:else}
            <p class="muted">No limit range values reported</p>
          {/if}

        <!-- Role / ClusterRole Summary -->
        {:else if kind === 'roles' || kind === 'clusterroles'}
          <h3>Policy Rules ({(resource.rules ?? []).length})</h3>
          {#if resource.rules?.length}
            <table>
              <thead><tr><th scope="col">API Groups</th><th scope="col">Resources</th><th scope="col">Verbs</th></tr></thead>
              <tbody>
                {#each resource.rules as rule}
                  <tr>
                    <td>{(rule.apiGroups ?? []).map((g: string) => g || '*').join(', ')}</td>
                    <td>{(rule.resources ?? []).join(', ')}</td>
                    <td>{(rule.verbs ?? []).join(', ')}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No policy rules</p>
          {/if}

        <!-- RoleBinding / ClusterRoleBinding Summary -->
        {:else if kind === 'rolebindings' || kind === 'clusterrolebindings'}
          <h3>Role Reference</h3>
          <dl>
            <dt>Kind</dt><dd>{resource.roleRef?.kind ?? 'N/A'}</dd>
            <dt>Name</dt>
            <dd>
              {#if resource.roleRef?.kind === 'ClusterRole'}
                <a href={`/resources/clusterroles/_cluster/${encodeURIComponent(resource.roleRef?.name ?? '')}`}>{resource.roleRef?.name ?? 'N/A'}</a>
              {:else if resource.roleRef?.kind === 'Role' && namespace}
                <a href={`/resources/roles/${encodeURIComponent(namespace)}/${encodeURIComponent(resource.roleRef?.name ?? '')}`}>{resource.roleRef?.name ?? 'N/A'}</a>
              {:else}
                {resource.roleRef?.name ?? 'N/A'}
              {/if}
            </dd>
            <dt>API Group</dt><dd>{resource.roleRef?.apiGroup ?? 'N/A'}</dd>
          </dl>

          <h3>Subjects ({(resource.subjects ?? []).length})</h3>
          {#if resource.subjects?.length}
            <table>
              <thead><tr><th scope="col">Kind</th><th scope="col">Name</th><th scope="col">Namespace</th></tr></thead>
              <tbody>
                {#each resource.subjects as subject}
                  <tr>
                    <td>{subject.kind ?? 'N/A'}</td>
                    <td>
                      {#if subject.kind === 'ServiceAccount' && subject.namespace}
                        <a href={`/resources/serviceaccounts/${encodeURIComponent(subject.namespace)}/${encodeURIComponent(subject.name ?? '')}`}>{subject.name ?? 'N/A'}</a>
                      {:else}
                        {subject.name ?? 'N/A'}
                      {/if}
                    </td>
                    <td>{subject.namespace ?? '—'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="muted">No subjects</p>
          {/if}

        <!-- ServiceAccount Summary -->
        {:else if kind === 'serviceaccounts'}
          <h3>Info</h3>
          <dl>
            <dt>Automount Token</dt><dd>{resource.automountServiceAccountToken !== false ? 'Yes' : 'No'}</dd>
          </dl>

          {#if resource.secrets?.length}
            <h3>Secrets</h3>
            {#each resource.secrets as sec}
              <div class="container-card">
                <strong>{sec.name ?? 'unnamed'}</strong>
              </div>
            {/each}
          {/if}

          {#if resource.imagePullSecrets?.length}
            <h3>Image Pull Secrets</h3>
            {#each resource.imagePullSecrets as ips}
              <div class="container-card">
                <strong>{ips.name ?? 'unnamed'}</strong>
              </div>
            {/each}
          {/if}

          {#if resource.metadata?.annotations?.['azure.workload.identity/client-id'] || resource.metadata?.labels?.['azure.workload.identity/use']}
            <h3>Azure Workload Identity</h3>
            <dl>
              {#if resource.metadata?.annotations?.['azure.workload.identity/client-id']}
                <dt>Client ID</dt><dd>{resource.metadata.annotations['azure.workload.identity/client-id']}</dd>
              {/if}
              {#if resource.metadata?.annotations?.['azure.workload.identity/tenant-id']}
                <dt>Tenant ID</dt><dd>{resource.metadata.annotations['azure.workload.identity/tenant-id']}</dd>
              {/if}
              {#if resource.metadata?.labels?.['azure.workload.identity/use']}
                <dt>WI Enabled</dt><dd>{resource.metadata.labels['azure.workload.identity/use']}</dd>
              {/if}
            </dl>
          {/if}

        <!-- Admission Webhook Configuration Summary -->
        {:else if kind === 'validatingwebhookconfigurations' || kind === 'mutatingwebhookconfigurations'}
          <h3>Configuration</h3>
          <dl>
            <dt>Type</dt><dd>{kind === 'validatingwebhookconfigurations' ? 'Validating' : 'Mutating'}</dd>
            <dt>Webhooks</dt><dd>{resource.webhooks?.length ?? 0}</dd>
          </dl>

          <h3>Webhook Definitions</h3>
          {#if resource.webhooks?.length}
            {#each resource.webhooks as webhook}
              <div class="container-card webhook-card">
                <strong>{webhook.name ?? 'unnamed'}</strong>
                <dl class="webhook-meta">
                  <dt>Client</dt><dd>{formatClientConfig(webhook.clientConfig)}</dd>
                  <dt>Failure Policy</dt><dd>{webhook.failurePolicy ?? 'Not set'}</dd>
                  <dt>Match Policy</dt><dd>{webhook.matchPolicy ?? 'Exact'}</dd>
                  <dt>Side Effects</dt><dd>{webhook.sideEffects ?? 'Unknown'}</dd>
                  <dt>Timeout</dt><dd>{webhook.timeoutSeconds ? `${webhook.timeoutSeconds}s` : 'Default'}</dd>
                  <dt>Admission Review Versions</dt><dd>{formatList(webhook.admissionReviewVersions, 'None')}</dd>
                </dl>

                <div class="webhook-section">
                  <h4>Rules</h4>
                  {#if webhook.rules?.length}
                    <table aria-label="Webhook rules">
                      <thead>
                        <tr>
                          <th scope="col">Operations</th>
                          <th scope="col">API Groups</th>
                          <th scope="col">Versions</th>
                          <th scope="col">Resources</th>
                          <th scope="col">Scope</th>
                        </tr>
                      </thead>
                      <tbody>
                        {#each webhook.rules as rule}
                          <tr>
                            <td>{formatList(rule.operations, 'All')}</td>
                            <td>{formatList(rule.apiGroups, 'All')}</td>
                            <td>{formatList(rule.apiVersions, 'All')}</td>
                            <td>{formatList(rule.resources, 'All')}</td>
                            <td>{rule.scope ?? 'All'}</td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  {:else}
                    <p class="muted">No rules defined</p>
                  {/if}
                </div>

                <div class="webhook-selector-grid">
                  <div class="selector-block">
                    <h4>Namespace Selector</h4>
                    {#if selectorParts(webhook.namespaceSelector).length}
                      <div class="labels">
                        {#each selectorParts(webhook.namespaceSelector) as selector}
                          <span class="label-badge">{selector}</span>
                        {/each}
                      </div>
                    {:else}
                      <p class="muted">All namespaces</p>
                    {/if}
                  </div>

                  <div class="selector-block">
                    <h4>Object Selector</h4>
                    {#if selectorParts(webhook.objectSelector).length}
                      <div class="labels">
                        {#each selectorParts(webhook.objectSelector) as selector}
                          <span class="label-badge">{selector}</span>
                        {/each}
                      </div>
                    {:else}
                      <p class="muted">All objects</p>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          {:else}
            <p class="muted">No webhooks defined</p>
          {/if}

        <!-- HPA Summary -->
        {:else if kind === 'hpas'}
          <h3>Scale Target</h3>
          <dl>
            {#if resource.spec?.scaleTargetRef}
              <dt>Kind</dt><dd>{resource.spec.scaleTargetRef.kind ?? 'N/A'}</dd>
              <dt>Name</dt>
              <dd>
                {#if resource.spec.scaleTargetRef.kind === 'Deployment'}
                  <a href="/resources/deployments/{resourceNamespace}/{resource.spec.scaleTargetRef.name}">{resource.spec.scaleTargetRef.name}</a>
                {:else if resource.spec.scaleTargetRef.kind === 'StatefulSet'}
                  <a href="/resources/statefulsets/{resourceNamespace}/{resource.spec.scaleTargetRef.name}">{resource.spec.scaleTargetRef.name}</a>
                {:else}
                  {resource.spec.scaleTargetRef.name ?? 'N/A'}
                {/if}
              </dd>
              <dt>API Version</dt><dd>{resource.spec.scaleTargetRef.apiVersion ?? 'N/A'}</dd>
            {:else}
              <dt>Target</dt><dd>N/A</dd>
            {/if}
          </dl>

          <h3>Replicas</h3>
          <dl>
            <dt>Min Replicas</dt><dd>{resource.spec?.minReplicas ?? 1}</dd>
            <dt>Max Replicas</dt><dd>{resource.spec?.maxReplicas ?? 'N/A'}</dd>
            <dt>Current Replicas</dt><dd>{resource.status?.currentReplicas ?? 0}</dd>
            <dt>Desired Replicas</dt><dd>{resource.status?.desiredReplicas ?? 0}</dd>
          </dl>

          {#if resource.spec?.metrics?.length}
            <h3>Metrics</h3>
            <table>
              <thead><tr><th scope="col">Type</th><th scope="col">Name</th><th scope="col">Target</th><th scope="col">Current</th></tr></thead>
              <tbody>
                {#each resource.spec.metrics as metric, idx}
                  {@const currentMetric = resource.status?.currentMetrics?.[idx]}
                  <tr>
                    <td>{metric.type ?? 'N/A'}</td>
                    <td>
                      {#if metric.type === 'Resource'}
                        {metric.resource?.name ?? 'N/A'}
                      {:else if metric.type === 'Pods'}
                        {metric.pods?.metric?.name ?? 'N/A'}
                      {:else if metric.type === 'Object'}
                        {metric.object?.metric?.name ?? 'N/A'}
                      {:else if metric.type === 'External'}
                        {metric.external?.metric?.name ?? 'N/A'}
                      {:else}
                        N/A
                      {/if}
                    </td>
                    <td>
                      {#if metric.type === 'Resource'}
                        {metric.resource?.target?.averageUtilization != null ? `${metric.resource.target.averageUtilization}%` : metric.resource?.target?.averageValue ?? 'N/A'}
                      {:else}
                        N/A
                      {/if}
                    </td>
                    <td>
                      {#if currentMetric?.type === 'Resource'}
                        {currentMetric.resource?.current?.averageUtilization != null ? `${currentMetric.resource.current.averageUtilization}%` : currentMetric.resource?.current?.averageValue ?? 'N/A'}
                      {:else}
                        N/A
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}

          {#if resource.spec?.behavior}
            <h3>Behavior</h3>
            {#if resource.spec.behavior.scaleUp}
              <h4>Scale Up</h4>
              <dl>
                <dt>Stabilization Window</dt><dd>{resource.spec.behavior.scaleUp.stabilizationWindowSeconds ?? 0}s</dd>
                <dt>Select Policy</dt><dd>{resource.spec.behavior.scaleUp.selectPolicy ?? 'Max'}</dd>
              </dl>
            {/if}
            {#if resource.spec.behavior.scaleDown}
              <h4>Scale Down</h4>
              <dl>
                <dt>Stabilization Window</dt><dd>{resource.spec.behavior.scaleDown.stabilizationWindowSeconds ?? 300}s</dd>
                <dt>Select Policy</dt><dd>{resource.spec.behavior.scaleDown.selectPolicy ?? 'Max'}</dd>
              </dl>
            {/if}
          {/if}

        <!-- PodDisruptionBudget Summary -->
        {:else if kind === 'poddisruptionbudgets'}
          <h3>Budget</h3>
          <dl>
            {#if resource.spec?.minAvailable != null}
              <dt>Min Available</dt><dd>{resource.spec.minAvailable}</dd>
            {/if}
            {#if resource.spec?.maxUnavailable != null}
              <dt>Max Unavailable</dt><dd>{resource.spec.maxUnavailable}</dd>
            {/if}
          </dl>

          {#if resource.spec?.selector?.matchLabels}
            <h3>Selector</h3>
            <div class="labels">
              {#each Object.entries(resource.spec.selector.matchLabels) as [key, value]}
                <span class="label-badge">{key}={value}</span>
              {/each}
            </div>
          {/if}

          <h3>Status</h3>
          <dl>
            <dt>Current Healthy</dt><dd>{resource.status?.currentHealthy ?? 0}</dd>
            <dt>Desired Healthy</dt><dd>{resource.status?.desiredHealthy ?? 0}</dd>
            <dt>Disruptions Allowed</dt><dd>{resource.status?.disruptionsAllowed ?? 0}</dd>
            <dt>Expected Pods</dt><dd>{resource.status?.expectedPods ?? 0}</dd>
            <dt>Observed Generation</dt><dd>{resource.status?.observedGeneration ?? 'N/A'}</dd>
          </dl>

        <!-- PriorityClass Summary -->
        {:else if kind === 'priorityclasses'}
          <h3>Info</h3>
          <dl>
            <dt>Priority Value</dt><dd>{resource.value ?? 0}</dd>
            <dt>Global Default</dt><dd>{resource.globalDefault ? 'Yes' : 'No'}</dd>
            <dt>Preemption Policy</dt><dd>{resource.preemptionPolicy ?? 'PreemptLowerPriority'}</dd>
            {#if resource.description}
              <dt>Description</dt><dd>{resource.description}</dd>
            {/if}
          </dl>

        <!-- StorageClass Summary -->
        {:else if kind === 'storageclasses'}
          <h3>Info</h3>
          <dl>
            <dt>Provisioner</dt><dd>{resource.provisioner ?? 'N/A'}</dd>
            <dt>Reclaim Policy</dt><dd>{resource.reclaimPolicy ?? 'Delete'}</dd>
            <dt>Volume Binding Mode</dt><dd>{resource.volumeBindingMode ?? 'Immediate'}</dd>
            <dt>Allow Volume Expansion</dt><dd>{resource.allowVolumeExpansion ? 'Yes' : 'No'}</dd>
          </dl>

          {#if resource.parameters && Object.keys(resource.parameters).length > 0}
            <h3>Parameters</h3>
            <table>
              <thead><tr><th scope="col">Key</th><th scope="col">Value</th></tr></thead>
              <tbody>
                {#each Object.entries(resource.parameters) as [key, value]}
                  <tr>
                    <td class="resource-name">{key}</td>
                    <td class="muted">{String(value)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}

          {#if resource.mountOptions?.length}
            <h3>Mount Options</h3>
            <div class="labels">
              {#each resource.mountOptions as opt}
                <span class="label-badge">{opt}</span>
              {/each}
            </div>
          {/if}

        <!-- PersistentVolume Summary -->
        {:else if kind === 'persistentvolumes'}
          <h3>Info</h3>
          <dl>
            <dt>Capacity</dt><dd>{resource.spec?.capacity?.storage ?? 'N/A'}</dd>
            <dt>Access Modes</dt><dd>{(resource.spec?.accessModes ?? []).join(', ') || 'N/A'}</dd>
            <dt>Reclaim Policy</dt><dd>{resource.spec?.persistentVolumeReclaimPolicy ?? 'N/A'}</dd>
            <dt>Status</dt><dd>{resource.status?.phase ?? 'Unknown'}</dd>
            <dt>Storage Class</dt><dd>{resource.spec?.storageClassName ?? 'N/A'}</dd>
            <dt>Volume Mode</dt><dd>{resource.spec?.volumeMode ?? 'Filesystem'}</dd>
          </dl>

          {#if resource.spec?.claimRef}
            <h3>Claim Reference</h3>
            <dl>
              <dt>Namespace</dt><dd>{resource.spec.claimRef.namespace ?? 'N/A'}</dd>
              <dt>Name</dt>
              <dd>
                {#if resource.spec.claimRef.namespace && resource.spec.claimRef.name}
                  <a href={`/resources/pvcs/${encodeURIComponent(resource.spec.claimRef.namespace)}/${encodeURIComponent(resource.spec.claimRef.name)}`}>{resource.spec.claimRef.name}</a>
                {:else}
                  {resource.spec.claimRef.name ?? 'N/A'}
                {/if}
              </dd>
            </dl>
          {/if}

          {#if resource.spec?.csi}
            <h3>CSI Driver</h3>
            <dl>
              <dt>Driver</dt><dd>{resource.spec.csi.driver ?? 'N/A'}</dd>
              {#if resource.spec.csi.volumeHandle}
                <dt>Volume Handle</dt><dd>{resource.spec.csi.volumeHandle}</dd>
              {/if}
              {#if resource.spec.csi.fsType}
                <dt>FS Type</dt><dd>{resource.spec.csi.fsType}</dd>
              {/if}
              {#if resource.spec.csi.readOnly !== undefined}
                <dt>Read Only</dt><dd>{resource.spec.csi.readOnly ? 'Yes' : 'No'}</dd>
              {/if}
            </dl>
          {/if}

          {#if resource.spec?.mountOptions?.length}
            <h3>Mount Options</h3>
            <div class="labels">
              {#each resource.spec.mountOptions as opt}
                <span class="label-badge">{opt}</span>
              {/each}
            </div>
          {/if}

        <!-- Fallback for unknown kinds -->
        {:else}
          <h3>Metadata</h3>
          <dl>
            <dt>Name</dt><dd>{resource.metadata?.name ?? 'N/A'}</dd>
            <dt>Namespace</dt><dd>{resource.metadata?.namespace ?? 'Cluster-scoped'}</dd>
            <dt>API Version</dt><dd>{resource.apiVersion ?? effectiveGvk ?? 'N/A'}</dd>
            <dt>Kind</dt><dd>{resource.kind ?? resourceLabel}</dd>
            <dt>Created</dt><dd>{resource.metadata?.creationTimestamp ?? 'N/A'}</dd>
          </dl>
        {/if}

        <!-- Common: Labels -->
        <h3>Labels</h3>
        {#if resource.metadata?.labels && Object.keys(resource.metadata.labels).length > 0}
          <div class="labels">
            {#each Object.entries(resource.metadata.labels) as [key, value]}
              <span class="label-badge">{key}={value}</span>
            {/each}
          </div>
        {:else}
          <p class="muted">No labels</p>
        {/if}

        <!-- Common: Annotations -->
        <h3>Annotations</h3>
        {#if resource.metadata?.annotations && Object.keys(resource.metadata.annotations).length > 0}
          <div class="labels">
            {#each Object.entries(resource.metadata.annotations) as [key, value]}
              <span class="label-badge">{key}={truncate(String(value), 60)}</span>
            {/each}
          </div>
        {:else}
          <p class="muted">No annotations</p>
        {/if}

        <!-- Azure Identity (workload kinds use pod template spec) -->
        {#if WORKLOAD_KINDS.has(kind) && resource.spec?.template}
          <AzureIdentitySection pod={resource.spec.template} />
        {/if}

        <!-- Common: Conditions (for kinds that have them) -->
        {#if resource.status?.conditions?.length}
          <h3>Conditions</h3>
          <table>
            <thead><tr><th scope="col">Type</th><th scope="col">Status</th><th scope="col">Reason</th><th scope="col">Message</th></tr></thead>
            <tbody>
              {#each resource.status.conditions as cond}
                <tr>
                  <td>{cond.type}</td>
                  <td class={cond.status === 'True' ? 'status-ok' : 'status-bad'}>{cond.status}</td>
                  <td>{cond.reason ?? ''}</td>
                  <td class="message">{cond.message ?? ''}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

    {:else if activeTab === 'pods'}
      <div class="pods-tab">
        {#if ownedPods.length === 0}
          <p class="muted">No pods found matching selector labels.</p>
        {:else}
          <p class="count">{ownedPods.length} pod{ownedPods.length !== 1 ? 's' : ''}</p>
          <table aria-label="Owned pods">
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col">Status</th>
                <th scope="col">Restarts</th>
                <th scope="col">Age</th>
              </tr>
            </thead>
            <tbody>
              {#each ownedPods as pod}
                <tr>
                  <td class="resource-name">
                    <a href="/pods/{pod.namespace}/{pod.name}">{pod.name}</a>
                  </td>
                  <td class={podPhase(pod) === 'Running' ? 'status-ok' : podPhase(pod) === 'Failed' ? 'status-bad' : ''}>
                    {podPhase(pod)}
                  </td>
                  <td>{podRestarts(pod)}</td>
                  <td>{podAge(pod)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

    {:else if activeTab === 'events'}
      <EventsTable events={events} showObject={false} />

    {:else if activeTab === 'yaml'}
      <div class="yaml-tab">
        {#if isSecret}
          <p class="muted">Secret values are redacted by default. Editing is disabled to avoid applying masked data.</p>
          <YamlEditor content={yamlContent} readonly={true} />
        {:else if isReadOnly}
          <p class="muted">{resourceLabel} resources are read-only in Telescope. YAML is available for inspection only.</p>
          <YamlEditor content={yamlContent} readonly={true} />
        {:else}
          <div class="yaml-actions">
            <button onclick={() => handleApply(true)} disabled={!yamlDirty || applying} class="action-btn" data-testid="resource-yaml-dry-run">
              <Icon name="dry-run" size={14} aria-hidden="true" /> Dry Run
            </button>
            <button onclick={() => handleApply(false)} disabled={!yamlDirty || applying} class="action-btn primary" data-testid="resource-yaml-apply">
              <Icon name="apply" size={14} aria-hidden="true" /> Apply
            </button>
            <button onclick={resetYaml} disabled={!yamlDirty} class="action-btn" data-testid="resource-yaml-reset">
              <Icon name="reset" size={14} aria-hidden="true" /> Reset
            </button>
            {#if isDynamicResource}
              <button onclick={() => (showDeleteConfirm = true)} disabled={deleting} class="action-btn danger" data-testid="resource-yaml-delete">
                <Icon name="delete" size={14} aria-hidden="true" /> Delete
              </button>
            {/if}
            {#if applyMessage}<span class={applyError ? 'apply-error' : 'apply-success'}>{applyMessage}</span>{/if}
          </div>
          <YamlEditor content={yamlContent} onchange={(v) => editedYaml = v} />
        {/if}
      </div>
    {/if}
  {/if}
</div>

<ScaleDialog
  open={showScale}
  resourceName={resourceName}
  currentReplicas={currentReplicas}
  onscale={handleScale}
  oncancel={() => showScale = false}
/>

<ConfirmDialog
  open={showScaleConfirm}
  title="Scale in Production"
  message={`You are about to scale "${resourceName}" to ${pendingScaleReplicas} replica(s) in a PRODUCTION context.`}
  confirmText="Scale"
  confirmValue={resourceName}
  requireType={true}
  productionContext={true}
  onconfirm={confirmScale}
  oncancel={() => { showScaleConfirm = false; pendingScaleReplicas = null; }}
/>

<ConfirmDialog
  open={showProdRestartConfirm}
  title="Rollout Restart in Production"
  message={`You are about to restart all pods for "${resourceName}" in a PRODUCTION context.`}
  confirmText="Restart"
  confirmValue={resourceName}
  requireType={true}
  productionContext={true}
  onconfirm={handleRolloutRestart}
  oncancel={() => showProdRestartConfirm = false}
/>

<ConfirmDialog
  open={showDeleteConfirm}
  title={`Delete ${resourceLabel}`}
  message={`Delete "${resourceName}"${namespace ? ` from namespace "${namespace}"` : ''}?`}
  confirmText={deleting ? 'Deleting…' : 'Delete'}
  confirmValue={resourceName}
  requireType={true}
  onconfirm={handleDelete}
  oncancel={() => { if (!deleting) showDeleteConfirm = false; }}
/>

<style>
  .detail-page {
    padding: 1.5rem;
    color: #e0e0e0;
    max-width: 960px;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  h1 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
    color: #e0e0e0;
  }

  .namespace-badge {
    background: #1a1a2e;
    color: #8b949e;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    border: 1px solid #21262d;
  }

  .action-btn.danger {
    background: #da3633;
  }

  .action-btn.danger:hover {
    background: #f85149;
  }

  .error {
    color: #ef5350;
    background: rgba(239, 83, 80, 0.1);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    border: 1px solid rgba(239, 83, 80, 0.3);
  }

  .summary h3 {
    color: #8b949e;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 1.25rem 0 0.5rem;
    border-bottom: 1px solid #21262d;
    padding-bottom: 0.25rem;
  }
  .summary h3:first-child {
    margin-top: 0;
  }

  dl {
    display: grid;
    grid-template-columns: 10rem 1fr;
    gap: 0.25rem 1rem;
    margin: 0;
    font-size: 0.875rem;
  }
  dt {
    color: #8b949e;
    font-weight: 500;
  }
  dd {
    color: #e0e0e0;
    margin: 0;
    font-family: monospace;
  }

  .labels {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }
  .label-badge {
    background: #1a1a2e;
    color: #8b949e;
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    font-size: 0.75rem;
    font-family: monospace;
    border: 1px solid #21262d;
  }

  .container-card {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
  }
  .container-card strong {
    color: #4fc3f7;
    font-size: 0.875rem;
  }

  .webhook-card {
    gap: 1rem;
  }

  .webhook-meta {
    grid-template-columns: 12rem 1fr;
  }

  .webhook-section h4,
  .selector-block h4 {
    margin: 0 0 0.5rem;
    color: #8b949e;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .webhook-selector-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1rem;
  }

  .selector-block {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .muted { color: #6e7681; font-size: 0.875rem; }
  .count { color: #9e9e9e; margin-bottom: 0.5rem; font-size: 0.875rem; }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
    font-family: monospace;
  }
  thead {
    background: #1a1a2e;
    color: #e0e0e0;
  }
  th, td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid #2a2a3e;
  }
  tr:hover { background: #16213e; }

  .resource-name {
    font-weight: 500;
    color: #4fc3f7;
  }
  .resource-name a {
    color: #4fc3f7;
    text-decoration: none;
  }
  .resource-name a:hover {
    text-decoration: underline;
    color: #58a6ff;
  }

  .status-ok { color: #66bb6a; }
  .status-bad { color: #ef5350; }

  .message {
    max-width: 360px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .yaml-tab { margin-top: 0.5rem; }
  .yaml-actions { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
  .action-btn { background: #21262d; color: #c9d1d9; border: 1px solid #30363d; padding: 0.35rem 0.75rem; border-radius: 6px; font-size: 0.8rem; cursor: pointer; }
  .action-btn:hover:not(:disabled) { background: #30363d; }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-btn.primary { background: #238636; border-color: #2ea043; }
  .action-btn.primary:hover:not(:disabled) { background: #2ea043; }
  .apply-success { color: #66bb6a; font-size: 0.8rem; }
  .apply-error { color: #ef5350; font-size: 0.8rem; }

  .pods-tab {
    margin-top: 0.5rem;
  }

  .rollout-section { margin-top: 0.5rem; }
  .rollout-actions { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.5rem; }
  .confirm-prompt { color: #fbbf24; font-weight: 500; }
  .rollout-message { margin-top: 0.5rem; color: #a0aec0; font-size: 0.85rem; }
  .badge { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: 600; margin-right: 0.5rem; }
  .badge-success { background: #065f46; color: #6ee7b7; }
  .badge-pending { background: #92400e; color: #fcd34d; }
  .btn { border: none; padding: 0.35rem 0.75rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; font-weight: 500; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary { background: #2563eb; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #1d4ed8; }
  .btn-danger { background: #dc2626; color: #fff; }
  .btn-danger:hover:not(:disabled) { background: #b91c1c; }
  .btn-secondary { background: #374151; color: #e0e0e0; }
  .btn-secondary:hover:not(:disabled) { background: #4b5563; }
  .btn-sm { padding: 0.25rem 0.6rem; font-size: 0.75rem; }
</style>
