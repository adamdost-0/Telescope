<script lang="ts">
  import { get } from 'svelte/store';
  import { selectedNamespace } from '$lib/stores';
  import YamlEditor from '$lib/components/YamlEditor.svelte';
  import { applyResource } from '$lib/api';

  interface TemplateEntry {
    label: string;
    yaml: Record<string, unknown>;
  }

  const TEMPLATES: Record<string, TemplateEntry> = {
    pod: {
      label: 'Pod',
      yaml: {
        apiVersion: 'v1',
        kind: 'Pod',
        metadata: { name: 'my-pod', namespace: 'default' },
        spec: {
          containers: [{ name: 'main', image: 'nginx:alpine', ports: [{ containerPort: 80 }] }]
        }
      }
    },
    deployment: {
      label: 'Deployment',
      yaml: {
        apiVersion: 'apps/v1',
        kind: 'Deployment',
        metadata: { name: 'my-deployment', namespace: 'default' },
        spec: {
          replicas: 1,
          selector: { matchLabels: { app: 'my-app' } },
          template: {
            metadata: { labels: { app: 'my-app' } },
            spec: {
              containers: [
                { name: 'main', image: 'nginx:alpine', ports: [{ containerPort: 80 }] }
              ]
            }
          }
        }
      }
    },
    service: {
      label: 'Service',
      yaml: {
        apiVersion: 'v1',
        kind: 'Service',
        metadata: { name: 'my-service', namespace: 'default' },
        spec: {
          selector: { app: 'my-app' },
          ports: [{ port: 80, targetPort: 80 }],
          type: 'ClusterIP'
        }
      }
    },
    configmap: {
      label: 'ConfigMap',
      yaml: {
        apiVersion: 'v1',
        kind: 'ConfigMap',
        metadata: { name: 'my-config', namespace: 'default' },
        data: { key1: 'value1' }
      }
    },
    secret: {
      label: 'Secret',
      yaml: {
        apiVersion: 'v1',
        kind: 'Secret',
        metadata: { name: 'my-secret', namespace: 'default' },
        type: 'Opaque',
        stringData: { password: 'changeme' }
      }
    }
  };

  function templateToYaml(key: string): string {
    const tmpl = structuredClone(TEMPLATES[key].yaml);
    const ns = get(selectedNamespace);
    if (tmpl.metadata && typeof tmpl.metadata === 'object') {
      (tmpl.metadata as Record<string, unknown>).namespace = ns;
    }
    return JSON.stringify(tmpl, null, 2);
  }

  let selectedTemplate = $state('pod');
  let yamlContent = $state(templateToYaml('pod'));
  let applying = $state(false);
  let result: { success: boolean; message: string } | null = $state(null);

  function handleTemplateChange(e: Event) {
    const key = (e.target as HTMLSelectElement).value;
    selectedTemplate = key;
    yamlContent = templateToYaml(key);
    result = null;
  }

  async function handleApply(dryRun: boolean) {
    applying = true;
    result = null;
    try {
      const res = await applyResource(yamlContent, dryRun);
      result = {
        success: res.success,
        message: dryRun
          ? `Dry run ${res.success ? 'passed' : 'failed'}: ${res.message}`
          : res.message
      };
    } catch (err) {
      result = {
        success: false,
        message: err instanceof Error ? err.message : String(err)
      };
    } finally {
      applying = false;
    }
  }
</script>

<div class="create-page">
  <header class="page-header">
    <h1>➕ Create Resource</h1>
    <p class="subtitle">Select a template and edit the manifest, then create or dry-run.</p>
  </header>

  <div class="controls">
    <label class="template-label" for="template-select">Template</label>
    <select id="template-select" value={selectedTemplate} onchange={handleTemplateChange}>
      {#each Object.entries(TEMPLATES) as [key, tmpl]}
        <option value={key}>{tmpl.label}</option>
      {/each}
    </select>
  </div>

  <div class="editor-wrapper">
    <YamlEditor content={yamlContent} onchange={(v) => (yamlContent = v)} />
  </div>

  <div class="actions">
    <button class="btn btn-secondary" disabled={applying} onclick={() => handleApply(true)}>
      {applying ? 'Validating…' : '🔍 Dry Run'}
    </button>
    <button class="btn btn-primary" disabled={applying} onclick={() => handleApply(false)}>
      {applying ? 'Creating…' : '🚀 Create'}
    </button>
  </div>

  {#if result}
    <div class="result" class:success={result.success} class:error={!result.success}>
      <span class="result-icon">{result.success ? '✅' : '❌'}</span>
      <span>{result.message}</span>
    </div>
  {/if}
</div>

<style>
  .create-page {
    padding: 1.5rem 2rem;
    max-width: 960px;
    color: #e0e0e0;
  }

  .page-header h1 {
    font-size: 1.4rem;
    font-weight: 600;
    margin: 0 0 0.25rem;
    color: #e0e0e0;
  }

  .subtitle {
    font-size: 0.85rem;
    color: #8b949e;
    margin: 0 0 1rem;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .template-label {
    font-size: 0.85rem;
    color: #8b949e;
    font-weight: 500;
  }

  select {
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.4rem 0.75rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
  select:focus {
    border-color: #58a6ff;
    outline: none;
  }

  .editor-wrapper {
    margin-bottom: 1rem;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .btn {
    padding: 0.5rem 1.25rem;
    border: none;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-primary {
    background: #1a73e8;
    color: #fff;
  }
  .btn-primary:hover:not(:disabled) {
    background: #1565c0;
  }

  .btn-secondary {
    background: #21262d;
    color: #c9d1d9;
  }
  .btn-secondary:hover:not(:disabled) {
    background: #30363d;
  }

  .result {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    line-height: 1.5;
    word-break: break-word;
  }
  .result.success {
    background: rgba(102, 187, 106, 0.1);
    border: 1px solid rgba(102, 187, 106, 0.3);
    color: #66bb6a;
  }
  .result.error {
    background: rgba(239, 83, 80, 0.1);
    border: 1px solid rgba(239, 83, 80, 0.3);
    color: #ef5350;
  }
  .result-icon {
    flex-shrink: 0;
  }
</style>
