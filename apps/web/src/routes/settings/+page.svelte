<script lang="ts">
  import { onMount } from 'svelte';
  import { getAzureCloud, getPreference, resolveAksIdentity, setAzureCloud, setPreference } from '$lib/api';
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
  let azureDetecting = $state(false);
  let azureSaving = $state(false);
  let azureError = $state<string | null>(null);
  let saving = $state(false);
  let saved = $state(false);

  const PREF_KEYS = {
    theme: 'theme',
    productionPatterns: 'production_patterns',
    defaultNamespace: 'default_namespace',
    autoRefreshInterval: 'auto_refresh_interval',
    azureCloud: 'azure_cloud',
    azureSubscription: 'azure_subscription',
    azureResourceGroup: 'azure_resource_group',
    azureClusterName: 'azure_cluster_name',
  } as const;

  const AZURE_CLOUD_OPTIONS = [
    { value: 'auto', label: 'Auto-detect' },
    { value: 'Commercial', label: 'Commercial' },
    { value: 'UsGovernment', label: 'US Government' },
    { value: 'UsGovSecret', label: 'US Gov Secret' },
    { value: 'UsGovTopSecret', label: 'US Gov Top Secret' },
  ] as const;

  function azureCloudLabel(cloud: string): string {
    return AZURE_CLOUD_OPTIONS.find((option) => option.value === cloud)?.label ?? cloud;
  }

  onMount(async () => {
    const [
      t,
      pp,
      ns,
      ari,
      storedAzureCloud,
      effectiveAzureCloud,
      storedAzureSubscription,
      storedAzureResourceGroup,
      storedAzureClusterName,
    ] = await Promise.all([
      getPreference(PREF_KEYS.theme),
      getPreference(PREF_KEYS.productionPatterns),
      getPreference(PREF_KEYS.defaultNamespace),
      getPreference(PREF_KEYS.autoRefreshInterval),
      getPreference(PREF_KEYS.azureCloud),
      getAzureCloud(),
      getPreference(PREF_KEYS.azureSubscription),
      getPreference(PREF_KEYS.azureResourceGroup),
      getPreference(PREF_KEYS.azureClusterName),
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
    if (storedAzureSubscription) azureSubscription = storedAzureSubscription;
    if (storedAzureResourceGroup) azureResourceGroup = storedAzureResourceGroup;
    if (storedAzureClusterName) azureClusterName = storedAzureClusterName;
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
      const identity = await resolveAksIdentity();
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

  async function save() {
    saving = true;
    saved = false;
    try {
      await Promise.all([
        setPreference(PREF_KEYS.theme, theme),
        setPreference(PREF_KEYS.productionPatterns, productionPatterns),
        setPreference(PREF_KEYS.defaultNamespace, defaultNamespace),
        setPreference(PREF_KEYS.autoRefreshInterval, autoRefreshInterval),
        setPreference(PREF_KEYS.azureSubscription, azureSubscription),
        setPreference(PREF_KEYS.azureResourceGroup, azureResourceGroup),
        setPreference(PREF_KEYS.azureClusterName, azureClusterName),
      ]);
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
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } finally {
      saving = false;
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
    <span class="field-hint" style="display:block;margin-bottom:0.75rem">
      Override auto-detected AKS identity. These values take priority over CLI detection.
    </span>
    <label class="field">
      <span class="field-label">Subscription ID</span>
      <input type="text" bind:value={azureSubscription} placeholder="00000000-0000-0000-0000-000000000000" />
    </label>
    <label class="field">
      <span class="field-label">Resource Group</span>
      <input type="text" bind:value={azureResourceGroup} placeholder="my-resource-group" />
    </label>
    <label class="field">
      <span class="field-label">Cluster Name</span>
      <input type="text" bind:value={azureClusterName} placeholder="my-aks-cluster" />
    </label>
    <button class="detect-btn" onclick={detectAzureIdentity} disabled={azureDetecting}>
      {azureDetecting ? 'Detecting…' : 'Auto-detect from cluster'}
    </button>
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
  .field-error {
    font-size: 0.75rem;
    color: #f85149;
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
