<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getAiInsightsSettings,
    getAksIdentityOverride,
    getAzureCloud,
    getPreference,
    resolveAksIdentity,
    setAiInsightsSettings,
    setAksIdentityOverride,
    setAzureCloud,
    setPreference,
    testAiInsightsConnection,
  } from '$lib/api';
  import type { AiInsightsConnectionTestResult } from '$lib/tauri-commands';
  import { AI_INSIGHTS_AUTH_MODES, AI_INSIGHTS_CLOUD_PROFILES } from '$lib/tauri-commands';
  import type {
    AiInsightsAuthMode,
    AiInsightsCloudProfile,
    AksIdentityOverrideSettings,
  } from '$lib/tauri-commands';
  import { updateProductionPatterns } from '$lib/prod-detection';
  import { version } from '$lib/version';
  import Icon from '$lib/icons/Icon.svelte';

  let theme = $state('system');
  let productionPatterns = $state('prod\nproduction\nprd');
  let defaultNamespace = $state('default');
  let autoRefreshInterval = $state('30');
  let azureCloud = $state('auto');
  let detectedAzureCloud = $state('Commercial');
  let azureSubscription = $state('');
  let azureResourceGroup = $state('');
  let azureClusterName = $state('');
  let azureOverrideContextName = $state<string | null>(null);
  let azureOverrideClusterFqdn = $state<string | null>(null);
  let azureOverrideAvailable = $state(false);
  let azureOverrideHasSavedValue = $state(false);
  let azureDetecting = $state(false);
  let azureSaving = $state(false);
  let azureError = $state<string | null>(null);
  let aiInsightsEndpoint = $state('');
  let aiInsightsDeploymentName = $state('');
  let aiInsightsAuthMode = $state<AiInsightsAuthMode>('azureLogin');
  let aiInsightsCloudProfile = $state<AiInsightsCloudProfile>('commercial');
  let aiInsightsModelName = $state('');
  let saving = $state(false);
  let saved = $state(false);
  let aiTestingConnection = $state(false);
  let aiTestResult = $state<AiInsightsConnectionTestResult | null>(null);
  let aiTestError = $state<string | null>(null);
  let aiTestApiKey = $state('');

  const PREF_KEYS = {
    theme: 'theme',
    productionPatterns: 'production_patterns',
    defaultNamespace: 'default_namespace',
    autoRefreshInterval: 'auto_refresh_interval',
    azureCloud: 'azure_cloud',
  } as const;

  const AZURE_CLOUD_OPTIONS = [
    { value: 'auto', label: 'Auto-detect' },
    { value: 'Commercial', label: 'Commercial' },
    { value: 'UsGovernment', label: 'US Government' },
    { value: 'UsGovSecret', label: 'US Gov Secret' },
    { value: 'UsGovTopSecret', label: 'US Gov Top Secret' },
  ] as const;

  const AI_INSIGHTS_AUTH_MODE_LABELS: Record<AiInsightsAuthMode, string> = {
    azureLogin: 'Azure login',
    apiKey: 'API key',
  };

  const AI_INSIGHTS_CLOUD_PROFILE_LABELS: Record<AiInsightsCloudProfile, string> = {
    commercial: 'Commercial',
    usGovernment: 'US Government',
    usGovernmentSecret: 'US Government Secret',
    usGovernmentTopSecret: 'US Government Top Secret',
  };

  function azureCloudLabel(cloud: string): string {
    return AZURE_CLOUD_OPTIONS.find((option) => option.value === cloud)?.label ?? cloud;
  }

  function applyAksIdentityOverride(settings: AksIdentityOverrideSettings) {
    azureOverrideAvailable = settings.isConnected && settings.isAks;
    azureOverrideContextName = settings.contextName;
    azureOverrideClusterFqdn = settings.clusterFqdn;
    azureOverrideHasSavedValue = settings.hasOverride;
    azureSubscription = settings.subscriptionId;
    azureResourceGroup = settings.resourceGroup;
    azureClusterName = settings.clusterName;
  }

  const azureOverrideScopeLabel = $derived.by(() => {
    if (!azureOverrideAvailable) {
      return 'Connect to an AKS cluster to edit a scoped override.';
    }

    const parts = [azureOverrideContextName, azureOverrideClusterFqdn].filter(
      (value): value is string => Boolean(value),
    );
    return parts.join(' · ');
  });

  const azureOverrideHint = $derived.by(() => {
    if (!azureOverrideAvailable) {
      return 'The inputs stay visible, but saving and auto-detect are disabled until an AKS cluster is connected.';
    }
    if (azureOverrideHasSavedValue) {
      return 'Saved values apply only to this cluster and will not redirect ARM actions for other AKS contexts.';
    }
    return 'No manual override is saved for this cluster yet. Leave the fields blank and save to keep using Azure CLI detection.';
  });

  onMount(async () => {
    const [
      t,
      pp,
      ns,
      ari,
      storedAzureCloud,
      effectiveAzureCloud,
      aksIdentityOverride,
      aiInsightsSettings,
    ] = await Promise.all([
      getPreference(PREF_KEYS.theme),
      getPreference(PREF_KEYS.productionPatterns),
      getPreference(PREF_KEYS.defaultNamespace),
      getPreference(PREF_KEYS.autoRefreshInterval),
      getPreference(PREF_KEYS.azureCloud),
      getAzureCloud(),
      getAksIdentityOverride(),
      getAiInsightsSettings(),
    ]);
    if (t) theme = t;
    if (pp) productionPatterns = pp;
    if (ns) defaultNamespace = ns;
    if (ari) autoRefreshInterval = ari;
    if (storedAzureCloud) {
      azureCloud = storedAzureCloud;
    } else if (typeof localStorage !== 'undefined') {
      azureCloud = localStorage.getItem('telescope-azure-cloud-selection') ?? 'auto';
    }
    detectedAzureCloud = effectiveAzureCloud;
    applyAksIdentityOverride(aksIdentityOverride);
    aiInsightsEndpoint = aiInsightsSettings.endpoint;
    aiInsightsDeploymentName = aiInsightsSettings.deploymentName;
    aiInsightsAuthMode = aiInsightsSettings.authMode;
    aiInsightsCloudProfile = aiInsightsSettings.cloudProfile;
    aiInsightsModelName = aiInsightsSettings.modelName ?? '';
  });

  async function handleAzureCloudChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    azureCloud = value;
    azureSaving = true;
    azureError = null;

    try {
      await setAzureCloud(value);
      detectedAzureCloud = value === 'auto' ? await getAzureCloud() : value;
    } catch {
      azureError = 'Failed to save Azure cloud preference.';
    } finally {
      azureSaving = false;
    }
  }

  async function detectAzureIdentity() {
    azureDetecting = true;
    azureError = null;

    try {
      const identity = await resolveAksIdentity({ preferOverride: false });
      if (!identity) {
        azureError = 'Unable to detect AKS identity from the active cluster.';
        return;
      }

      azureSubscription = identity.subscription_id;
      azureResourceGroup = identity.resource_group;
      azureClusterName = identity.cluster_name;
    } catch {
      azureError = 'Unable to detect AKS identity from the active cluster.';
    } finally {
      azureDetecting = false;
    }
  }

  async function handleTestAiConnection() {
    aiTestingConnection = true;
    aiTestResult = null;
    aiTestError = null;
    try {
      const key = aiInsightsAuthMode === 'apiKey' ? aiTestApiKey || undefined : undefined;
      aiTestResult = await testAiInsightsConnection(key);
    } catch (e) {
      aiTestError = e instanceof Error ? e.message : 'Failed to test AI Insights connection.';
    } finally {
      aiTestingConnection = false;
    }
  }

  async function save() {
    saving = true;
    azureSaving = true;
    saved = false;
    azureError = null;
    try {
      const writes = [
        setPreference(PREF_KEYS.theme, theme),
        setPreference(PREF_KEYS.productionPatterns, productionPatterns),
        setPreference(PREF_KEYS.defaultNamespace, defaultNamespace),
        setPreference(PREF_KEYS.autoRefreshInterval, autoRefreshInterval),
        setAiInsightsSettings({
          endpoint: aiInsightsEndpoint,
          deploymentName: aiInsightsDeploymentName,
          authMode: aiInsightsAuthMode,
          cloudProfile: aiInsightsCloudProfile,
          modelName: aiInsightsModelName === '' ? null : aiInsightsModelName,
        }),
      ];
      if (azureOverrideAvailable) {
        writes.push(
          setAksIdentityOverride({
            subscriptionId: azureSubscription,
            resourceGroup: azureResourceGroup,
            clusterName: azureClusterName,
          }),
        );
      }
      await Promise.all(writes);
      updateProductionPatterns(productionPatterns);
      if (typeof document !== 'undefined') {
        const resolvedTheme = theme === 'system'
          ? (window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark')
          : theme;
        document.documentElement.setAttribute('data-theme', resolvedTheme);
      }
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('telescope-theme', theme === 'system'
          ? (window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark')
          : theme);
      }
      azureOverrideHasSavedValue = azureOverrideAvailable
        && [azureSubscription, azureResourceGroup, azureClusterName]
          .some((value) => value.trim() !== '');
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (error) {
      azureError = error instanceof Error ? error.message : 'Failed to save preferences.';
    } finally {
      saving = false;
      azureSaving = false;
    }
  }
</script>

<div class="settings-page">
  <h1><Icon name="settings" size={20} aria-hidden="true" /> Settings</h1>

  <section class="settings-section">
    <h2>Appearance</h2>
    <label class="field">
      <span class="field-label">Theme</span>
      <select bind:value={theme}>
        <option value="system">System</option>
        <option value="dark">Dark</option>
        <option value="light">Light</option>
      </select>
    </label>
  </section>

  <section class="settings-section">
    <h2>Cluster</h2>
    <label class="field">
      <span class="field-label">Default namespace</span>
      <input type="text" bind:value={defaultNamespace} placeholder="default" />
    </label>
    <label class="field">
      <span class="field-label">Auto-refresh interval (seconds)</span>
      <input type="number" bind:value={autoRefreshInterval} min="5" max="300" />
    </label>
  </section>

  <section class="settings-section">
    <h2>Safety</h2>
    <label class="field">
      <span class="field-label">Production patterns</span>
      <span class="field-hint">Use commas or new lines. Context names matching any pattern are flagged as production.</span>
      <textarea rows="5" bind:value={productionPatterns} placeholder="prod&#10;production&#10;prd"></textarea>
    </label>
  </section>

  <section class="settings-section">
    <h2>Azure</h2>
    <label class="field">
      <span class="field-label">Azure Cloud</span>
      <select bind:value={azureCloud} onchange={handleAzureCloudChange} disabled={azureSaving}>
        {#each AZURE_CLOUD_OPTIONS as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
      <span class="field-hint">
        {#if azureCloud === 'auto'}
          Detected cloud: {azureCloudLabel(detectedAzureCloud)}
        {:else}
          Current cloud: {azureCloudLabel(detectedAzureCloud)}
        {/if}
      </span>
      {#if azureError}
        <span class="field-error">{azureError}</span>
      {/if}
    </label>
    <span class="field-hint section-hint">
      Manual AKS identity overrides are saved only for the currently connected AKS cluster. Leave
      all three fields blank and save to clear the override and fall back to Azure CLI detection.
    </span>
    <div class="scope-card" aria-live="polite">
      <span class="field-label">Override scope</span>
      <strong class="scope-value">{azureOverrideScopeLabel}</strong>
      <span class="field-hint">{azureOverrideHint}</span>
    </div>
    <label class="field">
      <span class="field-label">Subscription ID</span>
      <input
        type="text"
        bind:value={azureSubscription}
        placeholder="00000000-0000-0000-0000-000000000000"
        disabled={!azureOverrideAvailable}
      />
    </label>
    <label class="field">
      <span class="field-label">Resource Group</span>
      <input
        type="text"
        bind:value={azureResourceGroup}
        placeholder="my-resource-group"
        disabled={!azureOverrideAvailable}
      />
    </label>
    <label class="field">
      <span class="field-label">Cluster Name</span>
      <input
        type="text"
        bind:value={azureClusterName}
        placeholder="my-aks-cluster"
        disabled={!azureOverrideAvailable}
      />
    </label>
    <button
      class="detect-btn"
      onclick={detectAzureIdentity}
      disabled={azureDetecting || !azureOverrideAvailable}
    >
      {#if !azureOverrideAvailable}
        Connect to AKS to detect
      {:else if azureDetecting}
        Detecting…
      {:else}
        Auto-detect from cluster
      {/if}
    </button>
  </section>

  <section class="settings-section">
    <h2>AI Insights</h2>
    <span class="field-hint section-hint">
      These settings are saved in preferences. API keys are never persisted and must be provided per session when API key auth is selected.
    </span>
    <label class="field">
      <span class="field-label">Endpoint</span>
      <input type="url" bind:value={aiInsightsEndpoint} placeholder="https://example.openai.azure.com/" />
    </label>
    <label class="field">
      <span class="field-label">Deployment name</span>
      <input type="text" bind:value={aiInsightsDeploymentName} placeholder="my-deployment" />
    </label>
    <label class="field">
      <span class="field-label">Authentication mode</span>
      <select bind:value={aiInsightsAuthMode}>
        {#each AI_INSIGHTS_AUTH_MODES as mode}
          <option value={mode}>{AI_INSIGHTS_AUTH_MODE_LABELS[mode]}</option>
        {/each}
      </select>
      <span class="field-hint">
        {#if aiInsightsAuthMode === 'apiKey'}
          API key values are session-only and are not stored in preferences or localStorage.
        {:else}
          Azure login uses your current Azure identity session.
        {/if}
      </span>
    </label>
    <label class="field">
      <span class="field-label">Cloud profile</span>
      <select bind:value={aiInsightsCloudProfile}>
        {#each AI_INSIGHTS_CLOUD_PROFILES as profile}
          <option value={profile}>{AI_INSIGHTS_CLOUD_PROFILE_LABELS[profile]}</option>
        {/each}
      </select>
    </label>
    <label class="field">
      <span class="field-label">Model name (optional)</span>
      <input type="text" bind:value={aiInsightsModelName} placeholder="Leave blank to use deployment default" />
    </label>
    {#if aiInsightsAuthMode === 'apiKey'}
      <label class="field">
        <span class="field-label">API key (session only)</span>
        <input type="password" bind:value={aiTestApiKey} placeholder="Enter API key for connection test" />
        <span class="field-hint">Used only for the connection test below. Not saved.</span>
      </label>
    {/if}
    <button
      class="detect-btn"
      data-testid="test-ai-connection"
      onclick={handleTestAiConnection}
      disabled={aiTestingConnection || !aiInsightsEndpoint || !aiInsightsDeploymentName}
    >
      {aiTestingConnection ? 'Testing...' : 'Test Connection'}
    </button>
    {#if aiTestResult}
      <span class="field-success" data-testid="ai-test-success">Connection successful -- {aiTestResult.model}</span>
    {/if}
    {#if aiTestError}
      <span class="field-error" data-testid="ai-test-error">{aiTestError}</span>
    {/if}
  </section>

  <div class="actions">
    <button class="save-btn" onclick={save} disabled={saving}>
      {saving ? 'Saving…' : 'Save preferences'}
    </button>
    {#if saved}
      <span class="saved-badge">✓ Saved</span>
    {/if}
  </div>

  <section class="settings-section about">
    <h2>About</h2>
    <dl>
      <dt>Version</dt>
      <dd>{version}</dd>
      <dt>Project</dt>
      <dd><a href="https://github.com/AKSoftCode/Telescope" target="_blank" rel="noopener">GitHub</a></dd>
      <dt>License</dt>
      <dd>MIT</dd>
    </dl>
  </section>
</div>

<style>
  .settings-page {
    max-width: 640px;
    margin: 0 auto;
    padding: 2rem 1.5rem;
    color: #c9d1d9;
  }
  h1 {
    font-size: 1.5rem;
    margin-bottom: 1.5rem;
    color: #e6edf3;
  }
  .settings-section {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    padding: 1.25rem;
    margin-bottom: 1rem;
  }
  h2 {
    font-size: 0.95rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #8b949e;
    margin: 0 0 1rem;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 1rem;
  }
  .field:last-child { margin-bottom: 0; }
  .field-label {
    font-size: 0.85rem;
    color: #c9d1d9;
  }
  .field-hint {
    font-size: 0.75rem;
    color: #6e7681;
  }
  .scope-card {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 1rem;
    padding: 0.75rem;
    border: 1px solid #30363d;
    border-radius: 6px;
    background: #0d1117;
  }
  .scope-value {
    font-size: 0.9rem;
    color: #e6edf3;
  }
  .section-hint {
    display: block;
    margin-bottom: 0.75rem;
  }
  .field-error {
    font-size: 0.75rem;
    color: #f85149;
  }
  .field-success {
    font-size: 0.75rem;
    color: #3fb950;
    margin-top: 0.25rem;
  }
  select, input, textarea {
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
    color: #c9d1d9;
    padding: 0.5rem 0.625rem;
    font-size: 0.875rem;
    font-family: inherit;
  }
  select:focus, input:focus, textarea:focus {
    outline: none;
    border-color: #58a6ff;
    box-shadow: 0 0 0 2px rgba(88,166,255,0.2);
  }
  textarea { resize: vertical; }
  .actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .save-btn {
    background: #238636;
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    cursor: pointer;
  }
  .save-btn:hover:not(:disabled) { background: #2ea043; }
  .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .detect-btn {
    background: #1f6feb;
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 0.4rem 0.75rem;
    font-size: 0.8rem;
    cursor: pointer;
    margin-top: 0.25rem;
  }
  .detect-btn:hover:not(:disabled) { background: #388bfd; }
  .detect-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .saved-badge {
    color: #3fb950;
    font-size: 0.85rem;
    font-weight: 500;
  }
  .about dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.25rem 1rem;
    margin: 0;
    font-size: 0.85rem;
  }
  .about dt { color: #8b949e; }
  .about dd { margin: 0; }
  .about a { color: #58a6ff; text-decoration: none; }
  .about a:hover { text-decoration: underline; }
</style>
