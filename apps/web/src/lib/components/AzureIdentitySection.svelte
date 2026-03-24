<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';

  let { pod }: { pod: any } = $props();

  let hasWorkloadIdentity = $derived(
    pod?.metadata?.labels?.['azure.workload.identity/use'] === 'true'
  );

  let hasLegacyAadPodIdentity = $derived(
    !!pod?.metadata?.labels?.['aadpodidbinding']
  );

  let tokenMounts = $derived(
    (pod?.spec?.volumes ?? [])
      .filter((v: any) =>
        v?.projected?.sources?.some(
          (s: any) => s?.serviceAccountToken?.audience === 'api://AzureADTokenExchange'
        )
      )
      .map((v: any) => v.name)
  );
</script>

{#if hasWorkloadIdentity || hasLegacyAadPodIdentity}
  <div class="azure-identity-section">
    <h3>
      <Icon name="auth-token" size={18} aria-hidden="true" />
      <span>Azure Identity</span>
    </h3>

    {#if hasWorkloadIdentity}
      <div class="identity-card">
        <dl>
          <dt>Type</dt><dd>Azure Workload Identity</dd>
          <dt>Service Account</dt><dd>{pod?.spec?.serviceAccountName ?? 'N/A'}</dd>
          {#if tokenMounts.length > 0}
            <dt>Token Volume</dt><dd>{tokenMounts.join(', ')}</dd>
          {/if}
        </dl>
        <p class="hint">
          Check the service account annotations for <code>azure.workload.identity/client-id</code>
        </p>
      </div>
    {/if}

    {#if hasLegacyAadPodIdentity}
      <div class="legacy-warning" role="note">
        <Icon name="prod-warning" size={18} aria-hidden="true" />
        <span>
          This pod uses <strong>AAD Pod Identity</strong> (legacy). Consider migrating to
          <a
            href="https://learn.microsoft.com/en-us/azure/aks/workload-identity-overview"
            target="_blank"
            rel="noopener noreferrer"
          >Azure Workload Identity</a>.
          <br />Binding: <code>{pod.metadata.labels['aadpodidbinding']}</code>
        </span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .azure-identity-section h3 {
    color: #8b949e;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 1.25rem 0 0.5rem;
    border-bottom: 1px solid #21262d;
    padding-bottom: 0.25rem;
  }

  .identity-card {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
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

  .hint {
    color: #6e7681;
    font-size: 0.8rem;
    margin: 0.5rem 0 0;
  }
  .hint code {
    background: #1a1a2e;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    font-size: 0.75rem;
  }

  .legacy-warning {
    background: rgba(255, 167, 38, 0.08);
    border: 1px solid rgba(255, 167, 38, 0.3);
    border-radius: 6px;
    padding: 0.75rem;
    font-size: 0.85rem;
    color: #ffa726;
    line-height: 1.5;
    margin-top: 0.5rem;
  }
  .legacy-warning a {
    color: #58a6ff;
    text-decoration: underline;
  }
  .legacy-warning code {
    background: #1a1a2e;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    font-size: 0.8rem;
    color: #e0e0e0;
  }
</style>
